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
// Then release the lock as Esc does (the menu must come back, `menu-back.png`),
// and click the menu's Resume button to lock again. No click may edit a block.
//
// With $CAM (`x,y,z,yaw,pitch`: the start camera, see src/debug_view.rs) the
// page starts there; with $LEVER also set, the drive then right-clicks the
// crosshair once (screenshots `lever-before.png`, `lever-after.png`), which
// must flip a lever or press a button, and that one edit is the only one
// expected.
//
// With $CAM and $SAVE (use it instead of $LEVER) the drive instead breaks the
// block in the crosshair, opens the menu and clicks its download button
// (screenshots `save-menu.png`, then `saved.png` with the on-screen feedback),
// stores the download as `world.zip` in the out dir, prints `SAVE_EDIT=x,y,z
// old=id` for the block it broke, and resumes. The zip is then checked with
// `cargo test -p memcraft-viewer -- --ignored drives_download` (see
// src/download.rs), which wants the unzipped folder and that line's x,y,z.
// Against a running memcraft-server (pass its URL, e.g. http://127.0.0.1:8080/;
// see the README) also set $HOSTED: the button then saves to the server
// (the drive waits for the "Saved N columns to the server" log and screenshots
// it), no zip is stored, and at the end the page is reloaded (`reloaded.png`).
// The same ignored test, with DRIVE_WORLD the served world directory, checks
// that the edit is on disk.
// With $PICKER also set (and $CAM aimed at open air or a flat surface), the
// drive then presses B (the picker opens: the pointer lock must drop and the
// menu stay down; `picker-open.png`), types $PICKER as the search
// (`picker-search.png` shows the filtered grid), hovers the second block in the
// grid (`picker-hover.png`: its name in a tooltip), clicks it
// (`picker-picked.png`: the selected hotbar slot now holds it), presses Esc
// (the picker closes; the pointer stays free), clicks the scene to lock again
// and right-clicks the crosshair to place the picked block
// (`picker-placed.png`). Exactly one block edit is expected. A middle click
// then picks the block under the crosshair into the same slot
// (`picker-pickblock.png`; the log says which).
//
// With $CAM and $ORIENT (instead of the stages above), the drive places blocks
// the way a player would and screenshots each (`orient-0.png`, ...). $ORIENT
// is a JSON list of steps `[search, turnX, turnY, cells]`: pick the first block
// the picker finds for `search` (null keeps the slot's block), turn the camera by
// the mouse deltas (pixels; 0.2 degrees each), right-click the crosshair, and
// expect `cells` block edits (2 for a door; default 1; 0 for a step that only
// looks, no click) after holding the keys of an optional fifth element, a JSON
// list of [key, ms] like $KEYS. Orientation comes from
// the face the crosshair meets and where the camera looks (see src/orient.rs).
//
// With $CAM aimed at a chest (or another container) and $CHEST also set, the
// drive right-clicks the crosshair: the container's window opens (the pointer
// lock must drop and the menu stay down; `chest-before.png`, `chest-open.png`).
// E closes it and the pointer locks again (`chest-closed.png`); a second right
// click and Esc close it leaving the pointer free (`chest-esc.png`: the menu),
// and a click on the scene locks again. The stage edits no block (the final
// count may still include the lock click, which sometimes acts on the block
// it aims at before the view turns). The sample world's
// chest `CAM=90.5,77.8,110.5,270,5` looks at holds eleven stacks (the camera is
// checked by a test in src/targeting.rs).
//
// The menu hangs 200 px down: the buttons are 260x40 with
// 10 px gaps under a title, so Resume and Controls sit at fixed places in a
// 1280x720 viewport (see start.png).
const { chromium } = require('playwright')
const http = require('http'), fs = require('fs'), path = require('path')
const [dir, out] = process.argv.slice(2)
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' }
const server = http.createServer((req, res) => {
  const url = req.url.split('?')[0]
  const p = path.join(dir, url === '/' ? 'index.html' : url)
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
  // Resolves to the first log line matching `re` (or undefined after `ms`).
  const waitLog = async (re, ms) => {
    for (const end = Date.now() + ms; Date.now() < end; await page.waitForTimeout(250)) {
      const found = logs.find(l => re.test(l))
      if (found) return found
    }
  }
  // Software-rendered frames take a while, so Bevy may need seconds to react.
  const waitLock = (want) => page.waitForFunction(w => (document.pointerLockElement !== null) === w, want, { timeout: 20000 }).catch(() => {})

  const t0 = Date.now()
  const base = /^https?:/.test(dir) ? dir : `http://localhost:${server.address().port}/`
  const url = process.env.CAM ? `${base}?cam=${process.env.CAM}` : base
  await page.goto(url, { timeout: 120000 })
  await page.waitForFunction(() => !document.getElementById('loading'), null, { timeout: 180000 })
  await page.waitForTimeout(+(process.env.WAIT || 15000))
  console.log(`drawing after ${(Date.now() - t0) / 1000}s`)
  await shot('start.png')

  const RESUME = [640, 268], SAVE = [640, 318], CONTROLS = [640, 368], SCENE = [200, 360]
  console.log('locked before click:', await locked())
  await page.mouse.click(...CONTROLS, { button: 'left' })
  await page.waitForTimeout(3000)
  await shot('menu-controls.png')
  console.log('locked after clicking Controls (must be null):', await locked())
  await page.mouse.click(...CONTROLS, { button: 'left' })
  await page.waitForTimeout(1000)
  console.log('locked after clicking Controls again (must be null):', await locked())
  // Move there first and let frames pass: motion that arrives with the lock
  // would turn the view (440 px is 28 degrees), and a lever drive aims by $CAM.
  await page.mouse.move(...SCENE)
  await page.waitForTimeout(3000)
  await page.mouse.click(...SCENE, { button: 'left' })
  await waitLock(true)
  console.log('locked after left click:', await locked())
  if (process.env.LEVER) {
    await page.waitForTimeout(3000)
    await shot('lever-before.png')
    await page.mouse.click(...SCENE, { button: 'right' })
    await page.waitForTimeout(500)
    await shot('lever-after.png')
    console.log('toggles logged after the right click:', logs.filter(l => /\btoggled /.test(l)))
  }
  if (process.env.SAVE) {
    await page.waitForTimeout(3000)
    await page.mouse.click(...SCENE, { button: 'left' })
    await page.waitForTimeout(1000)
    const broke = logs.map(l => /\bbroke \((\d+), \d+\) at \[(-?\d+), (-?\d+), (-?\d+)\]/.exec(l)).find(Boolean)
    if (!broke) throw new Error('no block was broken')
    console.log(`SAVE_EDIT=${broke.slice(2).join(',')} old=${broke[1]}`)
    await page.evaluate(() => document.exitPointerLock())
    await waitLock(false)
    await page.waitForTimeout(3000)
    await shot('save-menu.png')
    if (process.env.HOSTED) {
      // Behind memcraft-server the button saves to the server instead.
      await page.mouse.click(...SAVE, { button: 'left' })
      const saved = await waitLog(/Saved \d+ columns? to the server/, 60000)
      console.log('saved:', saved)
      if (!saved) throw new Error('the server did not report the save')
    } else {
      const [download] = await Promise.all([
        page.waitForEvent('download', { timeout: 60000 }),
        page.mouse.click(...SAVE, { button: 'left' }),
      ])
      await download.saveAs(path.join(out, 'world.zip'))
      console.log('downloaded:', download.suggestedFilename())
    }
    await shot('saved.png')
    await page.waitForTimeout(6000)
    await shot('saved-later.png') // the feedback has gone again
    await page.mouse.click(...RESUME, { button: 'left' })
    await waitLock(true)
    console.log('locked after resuming from the save menu:', await locked())
  }
  if (process.env.PICKER) {
    await page.waitForTimeout(3000)
    await shot('picker-before.png')
    await page.keyboard.press('b')
    await waitLock(false)
    await page.waitForTimeout(3000)
    console.log('locked with the picker open (must be null):', await locked())
    await shot('picker-open.png')
    await page.keyboard.type(process.env.PICKER, { delay: 100 })
    await page.waitForTimeout(3000)
    await shot('picker-search.png')
    // The second cell of the grid (Orange Wool for "wool"): cells are 38 px
    // with 4 px gaps, the first at x = 431, y = 141 in a 1280x720 viewport
    // (see picker-search.png).
    const CELL = [431 + 42 + 19, 141 + 19]
    await page.mouse.move(...CELL)
    await page.waitForTimeout(3000)
    await shot('picker-hover.png')
    await page.mouse.click(...CELL, { button: 'left' })
    await page.waitForTimeout(3000)
    await shot('picker-picked.png')
    console.log('picks logged:', logs.filter(l => /\bpicked /.test(l)))
    await page.keyboard.press('Escape')
    await page.waitForTimeout(3000)
    console.log('locked after Esc closed the picker (must be null):', await locked())
    await shot('picker-closed.png')
    await page.mouse.move(...SCENE)
    await page.waitForTimeout(2000)
    await page.mouse.click(...SCENE, { button: 'left' })
    await waitLock(true)
    console.log('locked after the next click:', await locked())
    await page.waitForTimeout(3000)
    await page.mouse.click(...SCENE, { button: 'right' })
    await page.waitForTimeout(3000)
    await shot('picker-placed.png')
    console.log('block edits logged (the one placement):', logs.filter(l => /\b(broke|placed|toggled) /.test(l)))
    await page.mouse.click(...SCENE, { button: 'middle' })
    await page.waitForTimeout(3000)
    await shot('picker-pickblock.png')
    console.log('picks logged:', logs.filter(l => /\bpicked /.test(l)))
  }
  let orientCells = 0
  if (process.env.ORIENT) {
    await page.waitForTimeout(3000)
    const FIRST = [431 + 19, 141 + 19] // the first cell of the picker's grid
    let [mx, my] = SCENE
    const steps = JSON.parse(process.env.ORIENT)
    for (const [i, [search, dx = 0, dy = 0, cells = 1, keys = []]] of steps.entries()) {
      if (search) {
        // The picker's frames are slow to arrive in software rendering, so a
        // keystroke or click is sometimes lost: try again until it picks.
        const picked = () => logs.filter(l => /\bpicked /.test(l)).length
        for (let attempt = 0, before = picked(); attempt < 4 && picked() === before; attempt++) {
          if (attempt) {
            // The pointer is free after Esc: lock it again, as the player would.
            await page.mouse.move(...SCENE)
            await page.waitForTimeout(1000)
            await page.mouse.click(...SCENE, { button: 'left' })
            await waitLock(true)
            await page.waitForTimeout(2000)
          }
          await page.keyboard.press('b')
          await waitLock(false)
          await page.waitForTimeout(3000)
          await page.keyboard.type(search, { delay: 150 })
          await page.waitForTimeout(4000)
          await shot(`orient-${i}-picker${attempt || ''}.png`)
          await page.mouse.click(...FIRST, { button: 'left' })
          await page.waitForTimeout(3000)
          await page.keyboard.press('Escape')
          await page.waitForTimeout(1500)
        }
        await page.mouse.move(...SCENE)
        await page.waitForTimeout(1000)
        await page.mouse.click(...SCENE, { button: 'left' })
        await waitLock(true)
        ;[mx, my] = SCENE
        await page.waitForTimeout(2000)
      }
      // Turn in small steps, one frame's worth each.
      const n = Math.max(1, Math.ceil(Math.max(Math.abs(dx), Math.abs(dy)) / 20))
      for (let k = 0; k < n && (dx || dy); k++) {
        mx += dx / n; my += dy / n
        await page.mouse.move(mx, my)
        await page.waitForTimeout(80)
      }
      for (const [k, ms] of keys) {
        await page.keyboard.down(k); await page.waitForTimeout(ms); await page.keyboard.up(k)
      }
      await page.waitForTimeout(2500)
      if (cells > 0) await page.mouse.click(...SCENE, { button: 'right' })
      await page.waitForTimeout(2500)
      await shot(`orient-${i}.png`)
      orientCells += cells
    }
    console.log('placed:', logs.filter(l => /\bplaced /.test(l)))
  }
  if (process.env.CHEST) {
    // A chest has to be looked at exactly. The lock click on SCENE turned the
    // view by the 440 px it is left of the centre (a locked pointer's motion is
    // measured from the centre), so move back to the centre, which turns it
    // back, before looking.
    const CENTRE = [640, 360]
    const edits = () => logs.filter(l => /\b(broke|placed|toggled) /.test(l)).length
    const editsBefore = edits()
    await page.mouse.move(...CENTRE)
    await page.waitForTimeout(3000)
    await shot('chest-before.png')
    await page.mouse.click(...CENTRE, { button: 'right' })
    await waitLock(false)
    await page.waitForTimeout(3000)
    console.log('locked with the chest open (must be null):', await locked())
    await shot('chest-open.png')
    await page.keyboard.press('e')
    await waitLock(true)
    await page.waitForTimeout(3000)
    console.log('locked after E closed the chest (must be the canvas):', await locked())
    await shot('chest-closed.png')
    await page.mouse.click(...CENTRE, { button: 'right' })
    await waitLock(false)
    await page.waitForTimeout(3000)
    await page.keyboard.press('Escape')
    await page.waitForTimeout(3000)
    console.log('locked after Esc closed the chest (must be null):', await locked())
    await shot('chest-esc.png')
    await page.mouse.move(...SCENE)
    await page.waitForTimeout(2000)
    await page.mouse.click(...SCENE, { button: 'left' })
    await waitLock(true)
    console.log('locked after the next click:', await locked())
    console.log('block edits made by the chest stage (must be 0):', edits() - editsBefore)
  }
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
  await page.waitForTimeout(3000)
  await shot('menu-back.png')
  // A later frame must not re-take the lock without a new click.
  await page.waitForTimeout(3000)
  console.log('locked after exitPointerLock (what Esc does):', await locked())
  // The viewer must have noticed the release, or this click won't re-lock.
  await page.mouse.click(...RESUME, { button: 'left' })
  await waitLock(true)
  console.log('locked after clicking Resume:', await locked())
  console.log(`block edits logged (must be ${(process.env.LEVER ? 1 : 0) + (process.env.SAVE ? 1 : 0) + (process.env.PICKER ? 1 : 0) + orientCells}):`, logs.filter(l => /\b(broke|placed|toggled) /.test(l)).length)
  if (process.env.HOSTED && process.env.SAVE) {
    // A reload loads the saved world from the server: the broken block stays
    // broken (`reloaded.png`; the same camera as `save-menu.png`).
    await page.reload({ timeout: 120000 })
    await page.waitForFunction(() => !document.getElementById('loading'), null, { timeout: 180000 })
    await page.waitForTimeout(+(process.env.WAIT || 15000))
    await shot('reloaded.png')
  }
  printLogs()
  await browser.close(); server.close()
})().catch(e => { console.error(String(e).split('\n')[0]); printLogs(); process.exit(1) })
