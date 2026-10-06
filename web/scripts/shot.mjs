// Screenshot the running UI.
//
//   node scripts/shot.mjs <url> <out.png> [--click <selector>]... [--key <key>]... [--wait <ms>]
//
// Replaces a hand-rolled DevTools-protocol script that kept returning blank
// or one-pixel-tall captures. Playwright owns the viewport, waits for the
// page instead of sleeping blind, and runs the WebGL stage on a real GPU
// path, so what comes back is what a person would see. Keys are pressed in
// order after the clicks, so a shot can show the full-size view or a mark.
import { chromium } from 'playwright'

const args = process.argv.slice(2)
const url = args.shift()
const out = args.shift()
if (!url || !out) {
  console.error('usage: shot.mjs <url> <out.png> [--click <selector>]... [--key <key>]... [--wait <ms>]')
  process.exit(2)
}
const clicks = []
const keys = []
let wait = 1500
while (args.length) {
  const flag = args.shift()
  if (flag === '--click') clicks.push(args.shift())
  else if (flag === '--key') keys.push(args.shift())
  else if (flag === '--wait') wait = Number(args.shift())
  else throw new Error(`unknown flag ${flag}`)
}

const browser = await chromium.launch({ args: ['--use-angle=metal', '--enable-gpu'] })
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } })
  page.on('pageerror', (e) => console.error('page error:', e.message))
  await page.goto(url, { waitUntil: 'networkidle' })
  // The grid is drawn once the store's index and images arrive.
  await page.waitForSelector('.cell img', { timeout: 30000 }).catch(() => {})
  for (const sel of clicks) {
    await page.click(sel)
    await page.waitForLoadState('networkidle')
  }
  for (const k of keys) {
    await page.keyboard.press(k)
    await page.waitForTimeout(150)
  }
  await page.waitForTimeout(wait)
  await page.screenshot({ path: out })
  console.log('wrote', out)
} finally {
  await browser.close()
}
