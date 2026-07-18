// shoot-demo.mjs — live smoke test + screenshot of the public /demo page.
// Loads /demo in a real browser (Edge), presses "Score this browser", waits for
// the verdict to render, asserts a verdict appeared, and captures the page.
import { chromium } from 'playwright-core';

const OUT = process.argv[2];
const BASE = process.argv[3] || 'https://127.0.0.1:8443';
if (!OUT) { console.error('usage: node shoot-demo.mjs <outdir> [baseURL]'); process.exit(2); }

const browser = await chromium.launch({ channel: 'msedge', headless: true });
const ctx = await browser.newContext({ ignoreHTTPSErrors: true, viewport: { width: 1100, height: 1500 }, deviceScaleFactor: 1.0 });
const p = await ctx.newPage();
const errs = [];
p.on('pageerror', (e) => errs.push('pageerror: ' + e.message));
p.on('console', (m) => { if (m.type() === 'error') errs.push('console.error: ' + m.text()); });

await p.goto(BASE + '/demo', { waitUntil: 'domcontentloaded' });
await p.waitForSelector('#scorebtn', { timeout: 8000 });

// Simulate a little human movement so behavioral signals have data.
await p.mouse.move(200, 300); await p.mouse.move(500, 600); await p.mouse.move(300, 900);
await p.click('#scorebtn');

// Wait for the result region to be revealed (hidden attribute removed).
await p.waitForFunction(() => { const r = document.getElementById('result'); return r && !r.hidden; }, { timeout: 20000 });
await p.waitForTimeout(600);

const verdict = await p.$eval('#verdict', (el) => el.textContent.trim()).catch(() => '(none)');
const risk = await p.$eval('#bignum', (el) => el.textContent.trim()).catch(() => '?');
const lanes = await p.$$eval('#lanes .lane', (els) => els.length).catch(() => 0);

await p.screenshot({ path: `${OUT}/demo-full.jpg`, type: 'jpeg', quality: 72, fullPage: true });
for (const [sel, name] of [['#result', 'verdict'], ['#lanes', 'lanes']]) {
  const el = await p.$(sel);
  if (el) await el.screenshot({ path: `${OUT}/demo-${name}.jpg`, type: 'jpeg', quality: 72 }).catch(() => {});
}

await browser.close();
console.log(JSON.stringify({ verdict, risk, lanes, pageErrors: errs }, null, 2));
if (verdict === '(none)' || verdict === '—') { console.error('FAIL: no verdict rendered'); process.exit(1); }
console.log('OK: /demo scored this browser →', verdict, risk);
