const { chromium } = require('playwright')
const http = require('http'), fs = require('fs'), path = require('path')
const dir = process.argv[2]
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' }
const server = http.createServer((req, res) => {
  const p = path.join(dir, req.url === '/' ? 'index.html' : req.url.split('?')[0])
  fs.readFile(p, (e, b) => { if (e) { res.statusCode = 404; return res.end() } res.setHeader('Content-Type', types[path.extname(p)] || 'application/octet-stream'); res.end(b) })
}).listen(8766)
;(async () => {
  const browser = await chromium.launch({ headless: true, channel: 'chrome', args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] })
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } })
  const logs = []
  page.on('console', m => logs.push(`[${m.type()}] ${m.text()}`.slice(0, 400)))
  page.on('pageerror', e => logs.push('[pageerror] ' + e.message.slice(0, 400)))
  const t0 = Date.now()
  await page.goto('http://localhost:8766/', { waitUntil: 'load', timeout: 120000 })
  await page.waitForFunction(() => !document.getElementById('loading'), null, { timeout: 120000 }).catch(() => logs.push('[timeout] loading never cleared'))
  logs.push(`[time] drawing after ${(Date.now() - t0) / 1000}s`)
  await page.waitForTimeout(+(process.env.WAIT || 20000))
  if (process.env.KEYS) {
    await page.click('#memcraft')
    for (const [k, ms] of JSON.parse(process.env.KEYS)) { await page.keyboard.down(k); await page.waitForTimeout(ms); await page.keyboard.up(k) }
    await page.waitForTimeout(8000)
  }
  await page.screenshot({ path: process.env.OUT + '/' + (process.env.NAME || 'viewer') + '.png' })
  console.log(logs.filter(l => !/GL Driver/.test(l)).slice(-40).join('\n'))
  await browser.close(); server.close()
})().catch(e => { console.error(e); process.exit(1) })
