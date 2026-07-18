// shoot-console.mjs — screenshot of the Ledger admin console (SoT-26/28).
// Loads /__hmn/admin/console on the Gate admin listener; the dev build injects
// window.__HMN_TOKEN so the operator view auto-authenticates. Captures the
// Overview. Channel/args are env-driven (workstation Edge or container Chromium).
import { chromium } from 'playwright-core';

const OUT = process.argv[2];
const BASE = process.argv[3] || 'https://127.0.0.1:8445';
if (!OUT) { console.error('usage: node shoot-console.mjs <outdir> [adminBaseURL]'); process.exit(2); }

const chRaw = process.env.HM_LAUNCH_CHANNEL;
const channel = chRaw === undefined ? 'msedge' : (['chromium', 'bundled', ''].includes(chRaw) ? undefined : chRaw);
const args = (process.env.HM_LAUNCH_ARGS || '').split(',').map((s) => s.trim()).filter(Boolean);

const browser = await chromium.launch({ channel, headless: true, args });
const ctx = await browser.newContext({ ignoreHTTPSErrors: true, viewport: { width: 1360, height: 1400 }, deviceScaleFactor: 1.0 });
const p = await ctx.newPage();
const errs = [];
p.on('pageerror', (e) => errs.push('pageerror: ' + e.message));
p.on('console', (m) => { if (m.type() === 'error') errs.push('console.error: ' + m.text()); });

await p.goto(BASE + '/__hmn/admin/console', { waitUntil: 'domcontentloaded' });
// The SPA renders view containers (e.g. #view-overview) once the token is read.
await p.waitForSelector('#view-overview, .view, [data-view]', { timeout: 8000 }).catch(() => {});
await p.waitForTimeout(1200); // let the first SSE tick + fetches paint

const title = await p.title().catch(() => '');
await p.screenshot({ path: `${OUT}/console-overview.jpg`, type: 'jpeg', quality: 74, fullPage: false });

await browser.close();
console.log(JSON.stringify({ title, pageErrors: errs.slice(0, 5) }, null, 2));
if (errs.length) console.error('note: page errors present (see above)');
console.log('OK: captured console-overview.jpg');
