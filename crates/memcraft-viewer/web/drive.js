// Drives the browser build headlessly and screenshots it, as a smoke test.
//
//   node crates/memcraft-viewer/web/drive.js <dist dir> <out dir>
//
// Needs Playwright with Chromium (scripts/runner-setup.sh installs it in
// ~/tools/drive; run with NODE_PATH=~/tools/drive/node_modules). Steps:
// load and wait for the first frame, screenshot `start.png`; left-click the
// canvas and report whether the page holds the pointer lock; move the mouse
// and screenshot `turned.png`; optionally hold keys from $KEYS, a JSON list
// of [key, ms], and screenshot `moved.png`.
const { chromium } = require('playwright')
const http = require('http'), fs = require('fs'), path = require('path')
const [dir, out] = process.argv.slice(2)
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' }
const server = http.createServer((req, res) => {
  const p = path.join(dir, req.url === '/' ? 'index.html' : req.url.split('?')[0])
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
  await page.goto(`http://localhost:${server.address().port}/`, { timeout: 120000 })
  await page.waitForFunction(() => !document.getElementById('loading'), null, { timeout: 180000 })
  await page.waitForTimeout(+(process.env.WAIT || 15000))
  console.log(`drawing after ${(Date.now() - t0) / 1000}s`)
  await shot('start.png')

  console.log('locked before click:', await locked())
  await page.mouse.click(640, 360, { button: 'left' })
  await waitLock(true)
  console.log('locked after left click:', await locked())
  // Turn right, with no button held.
  for (let i = 0; i < 20; i++) { await page.mouse.move(640 + 20 * (i + 1), 360); await page.waitForTimeout(50) }
  await page.waitForTimeout(3000)
  await shot('turned.png')

  if (process.env.KEYS) {
    for (const [k, ms] of JSON.parse(process.env.KEYS)) {
      await page.keyboard.down(k); await page.waitForTimeout(ms); await page.keyboard.up(k)
    }
    await page.waitForTimeout(3000)
    await shot('moved.png')
  }
  await page.evaluate(() => document.exitPointerLock())
  await waitLock(false)
  // A later frame must not re-take the lock without a new click.
  await page.waitForTimeout(3000)
  console.log('locked after exitPointerLock (what Esc does):', await locked())
  // The viewer must have noticed the release, or this click won't re-lock.
  await page.mouse.click(640, 360, { button: 'left' })
  await waitLock(true)
  console.log('locked after clicking again:', await locked())
  printLogs()
  await browser.close(); server.close()
})().catch(e => { console.error(String(e).split('\n')[0]); printLogs(); process.exit(1) })
