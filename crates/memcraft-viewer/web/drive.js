// Drives the browser build headlessly and screenshots it, as a smoke test.
//
//   node crates/memcraft-viewer/web/drive.js <dist dir or URL> <out dir>
//
// Needs Playwright with Chromium (scripts/runner-setup.sh installs it in
// ~/tools/drive; run with NODE_PATH=~/tools/drive/node_modules). Steps:
// load and wait for the first frame (the menu is up: the pointer is free),
// screenshot `start.png`; click the menu's Controls button (screenshot
// `menu-controls.png`; the pointer must stay free) and click it again;
// left-click the scene, off the menu, and report whether the page holds the
// pointer lock; move the mouse and screenshot `turned.png`; optionally hold
// keys from $KEYS, a JSON list of [key, ms], and screenshot `moved.png`.
// $VIEW ("x,y,z,yaw,pitch") starts the camera there (the page's `?view=`) and
// skips the turn; $ACTIONS, a JSON list run once the pointer is locked, drives
// the scene: ["click"] or ["rightclick"] at the crosshair, ["type", text],
// ["press", key], ["wait", ms], ["shot", "name.png"], ["locked"] (logs the lock).
// Then release the lock as Esc does (the menu must come back, `menu-back.png`),
// and click the menu's Resume button to lock again. No click may edit a block.
//
// The menu hangs 200 px down: the buttons are 260x40 with
// 10 px gaps under a title, so Resume and Controls sit at fixed places in a
// 1280x720 viewport (see start.png).
const { chromium } = require('playwright')
const http = require('http'), fs = require('fs'), path = require('path')
const [dir, out] = process.argv.slice(2)
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' }
const server = http.createServer((req, res) => {
  const file = req.url.split('?')[0]
  const p = path.join(dir, file === '/' ? 'index.html' : file)
  fs.readFile(p, (e, b) => {
    if (e) { res.statusCode = 404; return res.end() }
    res.setHeader('Content-Type', types[path.extname(p)] || 'application/octet-stream')
    res.end(b)
  })
}).listen(0)
const logs = []
const printLogs = () => console.log(logs.filter(l => !/GL Driver|GPU stall/.test(l)).slice(-15).join('\n'))

;(async () => {
  const browser = await chromium.launch({
    channel: process.env.CHANNEL || 'chromium',
    args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'],
  })
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } })
  page.on('console', m => logs.push(`[${m.type()}] ${m.text()}`.slice(0, 300)))
  page.on('pageerror', e => logs.push('[pageerror] ' + e.message.slice(0, 300)))
  // SwiftShader frames are slow; give screenshots time.
  const shot = name => page.screenshot({ path: path.join(out, name), timeout: 120000 })
  const locked = () => page.evaluate(() => document.pointerLockElement?.id ?? null)
  // Software-rendered frames take a while, so Bevy may need seconds to react.
  const waitLock = (want) => page.waitForFunction(w => (document.pointerLockElement !== null) === w, want, { timeout: 20000 }).catch(() => {})

  const t0 = Date.now()
  const view = process.env.VIEW ? `?view=${process.env.VIEW}` : ''
  const url = (/^https?:/.test(dir) ? dir : `http://localhost:${server.address().port}/`) + view
  await page.goto(url, { timeout: 120000 })
  await page.waitForFunction(() => !document.getElementById('loading'), null, { timeout: 180000 })
  await page.waitForTimeout(+(process.env.WAIT || 15000))
  console.log(`drawing after ${(Date.now() - t0) / 1000}s`)
  await shot('start.png')

  const RESUME = [640, 268], CONTROLS = [640, 368], SCENE = [200, 360]
  console.log('locked before click:', await locked())
  await page.mouse.click(...CONTROLS, { button: 'left' })
  await page.waitForTimeout(3000)
  await shot('menu-controls.png')
  console.log('locked after clicking Controls (must be null):', await locked())
  await page.mouse.click(...CONTROLS, { button: 'left' })
  await page.waitForTimeout(1000)
  console.log('locked after clicking Controls again (must be null):', await locked())
  // Get to the scene first: a jump of the mouse in the frame that locks the
  // pointer would turn the view.
  await page.mouse.move(...SCENE)
  await page.waitForTimeout(3000)
  await page.mouse.click(...SCENE, { button: 'left' })
  await waitLock(true)
  console.log('locked after left click:', await locked())
  // Turn right, with no button held.
  for (let i = 0; i < (process.env.VIEW ? 0 : 20); i++) { await page.mouse.move(640 + 20 * (i + 1), 360); await page.waitForTimeout(50) }
  await page.waitForTimeout(3000)
  await shot('turned.png')

  for (const [action, arg] of JSON.parse(process.env.ACTIONS || '[]')) {
    // Clicks stay where the mouse is: with the pointer locked only the aim
    // matters, and a jump of the mouse would turn the view.
    if (action === 'click') await page.mouse.click(...SCENE, { button: 'left' })
    else if (action === 'rightclick') await page.mouse.click(...SCENE, { button: 'right' })
    else if (action === 'type') await page.keyboard.type(arg, { delay: 100 })
    else if (action === 'press') await page.keyboard.press(arg)
    else if (action === 'wait') await page.waitForTimeout(arg)
    else if (action === 'shot') await shot(arg)
    else if (action === 'locked') console.log(`locked after ${JSON.stringify(arg ?? '')}:`, await locked())
    // Software-rendered frames are slow: let the viewer react to each step.
    await page.waitForTimeout(2500)
  }

  if (process.env.KEYS) {
    for (const [k, ms] of JSON.parse(process.env.KEYS)) {
      await page.keyboard.down(k); await page.waitForTimeout(ms); await page.keyboard.up(k)
    }
    await page.waitForTimeout(3000)
    await shot('moved.png')
  }
  await page.evaluate(() => document.exitPointerLock())
  await waitLock(false)
  await page.waitForTimeout(3000)
  await shot('menu-back.png')
  // A later frame must not re-take the lock without a new click.
  await page.waitForTimeout(3000)
  console.log('locked after exitPointerLock (what Esc does):', await locked())
  // The viewer must have noticed the release, or this click won't re-lock.
  await page.mouse.click(...RESUME, { button: 'left' })
  await waitLock(true)
  console.log('locked after clicking Resume:', await locked())
  console.log('block edits logged (must be 0):', logs.filter(l => /\b(broke|placed|toggled) /.test(l)).length)
  printLogs()
  await browser.close(); server.close()
})().catch(e => { console.error(String(e).split('\n')[0]); printLogs(); process.exit(1) })
