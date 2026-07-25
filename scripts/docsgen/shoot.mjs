// shoot.mjs — render the real product surfaces (Ledger console, /demo, humanymous Pass)
// as HTML/CSS with representative stubbed data, screenshot them headlessly, and emit
// brand-framed WebP for the README + docs. No live engine, no stale raster: the pages
// ARE the repo's real HTML/CSS/JS —
//   • console.html + styles/*.css assembled exactly as internal/gate/console.go does,
//     with the admin API stubbed to canned fixtures (one capture per view + theme);
//   • demo.html with a canned verdict/lanes render (the same DOM demo.js builds), so
//     no WASM/scoring backend is needed;
//   • pass.html driven by a canned /api/pass/new (the CURRENT reel-alignment design).
// Each capture is wrapped in the macOS window chrome by frame.mjs → light + dark WebP.
//
// Run: cd scripts/docsgen && node shoot.mjs   (or `make docs-frames`). Uses the
// playwright-core + browser already present for the e2e harness (test/node_modules);
// falls back through Edge → bundled Chromium.

import { readFileSync, readdirSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname, resolve, extname } from 'node:path';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { frameContent, THEMES } from './frame.mjs';

const ROOT = resolve(join(dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1')), '..', '..'));
const OUT = join(ROOT, 'docs', 'assets', 'screenshots', 'framed');
const require = createRequire(import.meta.url);
const { chromium } = require(require.resolve('playwright-core', { paths: [join(ROOT, 'test', 'node_modules')] }));

// ── real page HTML, assembled the way the Go server serves it ────────────────
const consoleHTML = (() => {
  const html = readFileSync(join(ROOT, 'internal', 'gate', 'console.html'), 'utf8');
  const dir = join(ROOT, 'internal', 'gate', 'styles');
  const css = readdirSync(dir).filter((f) => f.endsWith('.css')).sort()
    .map((f) => readFileSync(join(dir, f), 'utf8')).join('\n');
  return html
    .replace('<style id="app-css"></style>', `<style id="app-css">${css}</style>`)
    .replace('<body>', `<body><script>window.__HMN_TOKEN='shot';</script>`);
})();

// demo.html without its live scripts; a canned verdict/lanes render is appended so the
// exact demo.js DOM is produced for a DENY/HR-7 fixture — no WASM, no scoring backend.
// (DEMO_RENDER is defined below; DEMO_RENDER_SRC is a hoisted function declaration.)
const DEMO_RENDER = `<script>(${DEMO_RENDER_SRC.toString()})()<\/script>`;
const demoHTML = readFileSync(join(ROOT, 'web', 'demo.html'), 'utf8')
  .replace(/<script src="\/static\/js\/wasm_exec\.js"><\/script>/, '')
  .replace(/<script type="module" src="\/static\/js\/demo\.js"><\/script>/, '') + DEMO_RENDER;

const passHTML = readFileSync(join(ROOT, 'web', 'pass.html'), 'utf8');
const playgroundHTML = readFileSync(join(ROOT, 'web', 'playground.html'), 'utf8'); // the Detection Observatory

// ── canned fixtures for the stubbed endpoints ────────────────────────────────
const now = 16;
const auditRecords = [
  ['admin.access', '—', 'none', [], 0, '—'],
  ['enforcement.deny', 'shop.example.com', 'deny', ['HR-7'], 95, 'enforce'],
  ['enforcement.allow', 'shop.example.com', 'allow', [], 4, 'enforce'],
  ['enforcement.challenge', 'app.example.com', 'challenge', ['HR-12'], 52, 'enforce'],
  ['enforcement.allow', 'app.example.com', 'allow', [], 6, 'enforce'],
  ['enforcement.deny', 'api.example.com', 'deny', ['HR-19'], 88, 'enforce'],
  ['enforcement.allow', 'shop.example.com', 'allow', [], 3, 'enforce'],
  ['ban.enforced', 'api.example.com', 'deny', ['HR-21'], 100, 'enforce'],
  ['enforcement.allow', 'app.example.com', 'allow', [], 8, 'enforce'],
  ['enforcement.challenge', 'shop.example.com', 'challenge', ['HR-12'], 44, 'enforce'],
  ['enforcement.allow', 'app.example.com', 'allow', [], 5, 'enforce'],
  ['enforcement.allow', 'shop.example.com', 'allow', [], 2, 'enforce'],
  ['enforcement.deny', 'login.example.com', 'deny', ['HR-1'], 91, 'enforce'],
  ['enforcement.allow', 'app.example.com', 'allow', [], 7, 'enforce'],
  ['enforcement.allow', 'shop.example.com', 'allow', [], 4, 'enforce'],
  ['instance.startup', '—', 'none', [], 0, '—'],
].map((r, i, a) => ({
  seq: a.length - i, event_type: r[0], host: r[1], verdict: r[2],
  triggered_rules: r[3], risk_score: r[4], enforcement_mode: r[5],
  incident: r[2] === 'deny' || r[2] === 'challenge' ? 'INC-' + (1000 + i) : '',
}));

const FIX = {
  '/whoami': { id: 'operator-1', role: 'operator' },
  '/audit': { count: now, records: auditRecords },
  '/integrity': { ok: true, records: now, checkpoints: 3, witnessed: true, node: 'gate-1', lastSTH: { treeSize: now, root: 'a1b2c3d4e5f6a7b8c9d0e1f2' } },
  '/bans': {
    count: 2, bans: [
      { key: 'ip:203.0.113.9', source: 'auto', reason: 'rate-limit flood (HR-21)', strike: 2, expiresInSec: 6 * 3600 },
      { key: 'fp:9f3c1a7e…', source: 'manual', reason: 'coordinated scraping', strike: 1, expiresInSec: 0 },
    ],
  },
  '/policy': {
    globalMonitor: false, effectiveMonitor: false, killSwitch: false,
    rateLimit: { hard: 120, soft: 60, windowSec: 10 },
    routes: [
      { prefix: '/', preset: 'balanced', enforce: true, failClosed: false, syncScore: false, inject: true },
      { prefix: '/login', preset: 'strict', enforce: true, failClosed: true, syncScore: true, inject: true },
      { prefix: '/checkout', preset: 'strict', enforce: true, failClosed: true, syncScore: true, inject: true },
      { prefix: '/api', preset: 'balanced', enforce: true, failClosed: true, syncScore: false, inject: false },
      { prefix: '/healthz', preset: 'off', enforce: false, failClosed: false, syncScore: false, inject: false },
    ],
  },
  '/approvals': {
    count: 2, pending: [
      { id: 'a1b2c3d4e5f6a7b8', kind: 'ban.permanent', needsRole: 'approver', params: { key: 'ip:198.51.100.7', reason: 'botnet source' } },
      { id: 'c9d0e1f2a3b4c5d6', kind: 'erasure', needsRole: 'dpo', params: { subject: 'psn_4f7a…' } },
    ],
  },
  '/erasures': { scheduled: [] },
};

// Detection Observatory: a scored session pushed over SSE + its server-truth ScoreTrace.
// A puppeteer-class headless bot → HR-7 DENY, mirroring the /demo fixture at engine depth.
const obsReport = {
  sessionId: 'obs-7f3a', source: 'launch:puppeteer',
  scoring: {
    riskScore: 82.4, verdict: 'DENY', hardRuleFired: 'HR-7', policyVersion: '1.0.0',
    topContributors: [{ id: 'l1.navigator.webdriver', score: 40 }, { id: 'l1.headless.newHeadless', score: 22 }, { id: 'x.ua_vs_gpu', score: 18 }],
  },
  client: { signals: [
    { layer: 'L1', id: 'l1.navigator.webdriver', verdict: 'BOT', score: 40, collected: 'wasm', notes: 'webdriver=true' },
    { layer: 'L1', id: 'l1.headless.newHeadless', verdict: 'BOT', score: 22, collected: 'js', notes: 'HeadlessChrome UA token' },
    { layer: 'L2', id: 'l2.webgl.swiftshader', verdict: 'SUSPICIOUS', score: 14, collected: 'js', notes: 'software renderer' },
    { layer: 'L2', id: 'l2.canvas.noise_absent', verdict: 'SUSPICIOUS', score: 8, collected: 'js' },
    { layer: 'L3', id: 'l3.native.toString_ok', verdict: 'OK', score: 0, collected: 'wasm' },
    { layer: 'L4', id: 'l4.behavior.no_interaction', verdict: 'SUSPICIOUS', score: 9, collected: 'js' },
  ] },
  network: { ja4: 't13d1516h2_8daaf6152771_b186095e22b6', ja4Engine: 'chrome',
    signals: [{ layer: 'L5', id: 'l5.h2.akamai_fp', verdict: 'OK', score: 0, collected: 'server' }] },
  crosschecks: [{ id: 'x.ua_vs_gpu', consistent: false }, { id: 'x.ua_vs_ja4', consistent: true }, { id: 'x.browser_no_js', consistent: true }],
};
const obsMeta = { policy: { version: '1.0.0', challengeAt: 30, denyAt: 70, layerCap: 60 } };
const obsExplain = {
  hardRuleEval: [
    { rule: 'HR-1', why: 'Selenium / cdc_ automation artifact', matched: false },
    { rule: 'HR-2', why: 'UA claims a browser but TLS+H2 resolve to another engine', matched: false },
    { rule: 'HR-7', why: 'Headless browser plus a second automation tell', matched: true, won: true, verdict: 'DENY' },
    { rule: 'HR-8', why: 'Patched native getter hiding webdriver', matched: false },
    { rule: 'HR-12', why: 'No interaction over the observation window', matched: true },
    { rule: 'HR-18', why: 'Browser UA but zero client-side JS evidence', matched: false },
  ],
  perLayer: [
    { layer: 'L1', rawProb: 0.62, cappedProb: 0.60, capHit: true },
    { layer: 'L2', rawProb: 0.21, cappedProb: 0.21, capHit: false },
    { layer: 'L4', rawProb: 0.09, cappedProb: 0.09, capHit: false },
    { layer: 'L6', rawProb: 0.18, cappedProb: 0.18, capHit: false },
  ],
  dedupDropped: [{ droppedId: 'l1.webdriver.present', keptId: 'l1.navigator.webdriver' }],
};

const passChallenge = {
  challenge: {
    n: 11, center: 5, rows: [
      { chars: ['A', 'K', 'M', 'H', 'U', 'N', 'Y', 'O', 'S', 'E', 'R'], keyIndex: 5 },
      { chars: ['Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', 'Z'], keyIndex: 3 },
      { chars: ['Z', 'X', 'C', 'V', 'B', 'N', 'M', 'L', 'K', 'J', 'H'], keyIndex: 7 },
    ],
  },
  bucket: 0, challengeNonce: 'shot-nonce', preflight: { pow: { seed: '00', difficulty: 0, bucket: 0 } },
};

// ── static server: real pages + stubbed JSON ─────────────────────────────────
const MIME = { '.svg': 'image/svg+xml', '.js': 'text/javascript', '.css': 'text/css' };
function jsonRes(res, obj) { res.writeHead(200, { 'Content-Type': 'application/json' }); res.end(JSON.stringify(obj)); }
function htmlRes(res, s) { res.writeHead(200, { 'Content-Type': 'text/html' }); res.end(s); }

const sseClients = new Set(); // open Observatory SSE responses, ended on teardown

const server = createServer((req, res) => {
  const u = new URL(req.url, 'http://x');
  const p = u.pathname;
  if (p === '/__hmn/admin/console') return htmlRes(res, consoleHTML);
  if (p === '/demo') return htmlRes(res, demoHTML);
  if (p === '/pass') return htmlRes(res, passHTML);
  if (p === '/playground') return htmlRes(res, playgroundHTML);
  // Observatory data plane: meta + server-truth ScoreTrace + a scored session over SSE.
  if (p === '/playground/meta') return jsonRes(res, obsMeta);
  if (p.startsWith('/playground/explain/')) return jsonRes(res, obsExplain);
  if (p === '/playground/events') {
    res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache', Connection: 'keep-alive' });
    res.write(':ok\n\n');
    sseClients.add(res);
    req.on('close', () => sseClients.delete(res));
    setTimeout(() => { if (!res.writableEnded) res.write('event: session.scored\ndata: ' + JSON.stringify({ sessionId: obsReport.sessionId, source: obsReport.source, report: obsReport }) + '\n\n'); }, 250);
    return;
  }
  // stubbed admin API (strip the /__hmn/admin prefix)
  for (const key of Object.keys(FIX)) {
    if (p === '/__hmn/admin' + key) return jsonRes(res, FIX[key]);
  }
  // stubbed Pass API
  if (p === '/api/pass/new') return jsonRes(res, passChallenge);
  if (p === '/api/pass/pow') return jsonRes(res, { ok: true });
  if (p === '/api/pass/attest') return jsonRes(res, { ok: true, token: 'shot' });
  if (p === '/api/pass/solve') return jsonRes(res, { ok: true, verdict: 'ALLOW' });
  // best-effort static assets from web/ (e.g. the pass favicon)
  if (p.startsWith('/static/')) {
    try {
      const body = readFileSync(join(ROOT, 'web', p.slice('/static/'.length)));
      res.writeHead(200, { 'Content-Type': MIME[extname(p)] || 'application/octet-stream' });
      return res.end(body);
    } catch { /* fall through */ }
  }
  res.writeHead(404); res.end('');
});

// ── asset manifest: each → one framed WebP per theme ─────────────────────────
const ASSETS = [
  { name: 'hero-ledger', page: 'console', view: 'overview', title: 'humanymous — Ledger', sel: '.app', w: 1600, perTheme: true, vp: { width: 1360, height: 900 } },
  { name: 'console-bans', page: 'console', view: 'bans', title: 'humanymous — Ledger', sel: '.app', w: 1600, perTheme: true, vp: { width: 1360, height: 900 } },
  { name: 'console-policy', page: 'console', view: 'policy', title: 'humanymous — Ledger', sel: '.app', w: 1600, perTheme: true, vp: { width: 1360, height: 980 } },
  { name: 'console-approvals', page: 'console', view: 'approvals', title: 'humanymous — Ledger', sel: '.app', w: 1600, perTheme: true, vp: { width: 1360, height: 760 } },
  { name: 'verdict-detail', page: 'demo', title: 'humanymous — /demo', sel: '.wrap', w: 1200, perTheme: false, vp: { width: 940, height: 900 } },
  // Pass: capture the body (its own gradient bg) in a viewport wider than the 640px
  // card so the card sits inset with breathing room; trim off (keeps the side margins).
  { name: 'pass-challenge', page: 'pass', title: 'humanymous — Pass', sel: 'body', w: 900, perTheme: false, trim: false, vp: { width: 800, height: 900 } },
  // Observatory: the launcher rail is far taller than the scored-session content, so
  // clip to the bottom of the last center panel instead of capturing the whole body.
  { name: 'observatory', page: 'observatory', title: 'humanymous — Detection Observatory', sel: 'body', w: 1500, perTheme: false, clipBottom: '.center', vp: { width: 1300, height: 1480 } },
];
const URLS = { console: '/__hmn/admin/console', demo: '/demo', pass: '/pass', observatory: '/playground' };

async function prep(page, a) {
  if (a.page === 'console') {
    await page.waitForSelector('.app', { timeout: 8000 });
    if (a.view !== 'overview') {
      await page.click(`.navitem[data-view="${a.view}"]`);
      await page.waitForSelector(`#view-${a.view}:not([hidden])`, { timeout: 8000 });
    }
    await page.waitForTimeout(500); // let the view's fetch paint
  } else if (a.page === 'demo') {
    await page.waitForSelector('#lanes .lane', { timeout: 8000 });
    await page.evaluate(() => {
      document.querySelectorAll('header.hero, footer, .card').forEach((el) => {
        if (el.id !== 'result' && el.id !== 'lanesSection') el.style.display = 'none';
      });
    });
  } else if (a.page === 'pass') {
    await page.waitForSelector('.reel', { timeout: 8000 });
    await page.evaluate(() => document.querySelectorAll('.note').forEach((n) => (n.style.display = 'none')));
    await page.waitForTimeout(200);
  } else if (a.page === 'observatory') {
    await page.waitForSelector('#lanes .lane', { timeout: 8000 });          // SSE session.scored rendered
    await page.waitForSelector('#explain:not([hidden])', { timeout: 8000 }).catch(() => {}); // ScoreTrace fetched
    await page.evaluate(() => { const c = document.querySelector('.rcard[data-p="puppeteer.mjs"]'); if (c) c.click(); }); // highlight the launched profile
    await page.waitForTimeout(300);
  }
}

async function run() {
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  const base = 'http://127.0.0.1:' + server.address().port;
  mkdirSync(OUT, { recursive: true });

  const channels = [{ channel: 'msedge' }, { channel: undefined }]; // Edge, then bundled Chromium
  let browser, lastErr;
  for (const c of channels) {
    try { browser = await chromium.launch({ ...c, headless: true }); break; } catch (e) { lastErr = e; }
  }
  if (!browser) throw lastErr;

  let n = 0;
  for (const a of ASSETS) {
    const themes = a.perTheme ? ['dark', 'light'] : ['dark'];
    const captured = {};
    for (const theme of themes) {
      const ctx = await browser.newContext({ viewport: a.vp, deviceScaleFactor: 2, colorScheme: theme, ignoreHTTPSErrors: true, locale: 'en-US' });
      const page = await ctx.newPage();
      await page.goto(base + URLS[a.page], { waitUntil: 'domcontentloaded' });
      if (a.page === 'console') await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), theme);
      await prep(page, a);
      if (a.clipBottom) {
        // clip from the top to the true content bottom (max bottom of the container's
        // direct children) — the container itself stretches to the tall rail, so its own
        // box would re-introduce the dead space we're trying to drop.
        const h = await page.evaluate((sel) => {
          const root = document.querySelector(sel);
          if (!root) return document.body.scrollHeight;
          let max = 0;
          root.querySelectorAll(':scope > *').forEach((k) => { const b = k.getBoundingClientRect().bottom; if (b > max) max = b; });
          return Math.ceil(max + 20);
        }, a.clipBottom);
        // clip only captures within the viewport, so the vp must be tall enough (below).
        captured[theme] = await page.screenshot({ clip: { x: 0, y: 0, width: a.vp.width, height: h } });
        captured[theme] = await page.screenshot({ clip: { x: 0, y: 0, width: a.vp.width, height: h } });
      } else {
        captured[theme] = await page.locator(a.sel).first().screenshot();
      }
      await ctx.close();
    }
    // Emit both chrome variants. per-theme pages give a theme-matched inner capture;
    // single-capture pages (dark app) reuse the dark inner under both chromes.
    for (const theme of ['dark', 'light']) {
      const inner = captured[theme] || captured.dark;
      const webp = await frameContent(inner, { title: a.title, w: a.w, theme, trim: a.trim !== false });
      writeFileSync(join(OUT, `${a.name}-${theme}.webp`), webp);
      n++;
    }
    process.stdout.write(`  ✓ ${a.name}\n`);
  }

  await browser.close();
  for (const c of sseClients) { try { c.end(); } catch { /* already gone */ } }
  await new Promise((r) => server.close(r));
  console.log(`shoot: ${n} framed WebP (${ASSETS.length} surfaces × 2 themes) → docs/assets/screenshots/framed/`);
}

// Canned verdict/lanes render appended to demo.html — reproduces demo.js render() for a
// recorded DENY/HR-7 fixture (see web/js/demo.js). Kept in sync with that DOM.
function DEMO_RENDER_SRC() {
  const verdict = { verdict: 'DENY', riskScore: 75.2, hardRuleFired: 'HR-7', sessionId: 'demo' };
  const full = {
    scoring: { verdict: 'DENY', riskScore: 75.2, hardRuleFired: 'HR-7', policyVersion: '1.0.0' },
    client: { signals: [
      { layer: 'L1', id: 'l1.navigator.webdriver', score: 40, verdict: 'BOT', notes: 'webdriver=true' },
      { layer: 'L1', id: 'l1.headless.newHeadless', score: 18, verdict: 'BOT', notes: 'HeadlessChrome UA token' },
      { layer: 'L2', id: 'l2.webgl.swiftshader', score: 12, verdict: 'SUSPICIOUS', notes: 'software renderer' },
      { layer: 'L3', id: 'l3.native.toString', score: 0, verdict: 'OK' },
      { layer: 'L4', id: 'l4.behavior.no_interaction', score: 8, verdict: 'SUSPICIOUS' },
    ] },
    network: { ja4: 't13d1516h2_8daaf6152771_b186095e22b6', ja4Engine: 'chrome',
      signals: [{ layer: 'L5', id: 'l5.h2.akamai_fp', score: 0, verdict: 'OK' }] },
    crosschecks: [{ id: 'x.ua_vs_gpu', consistent: false }, { id: 'x.ua_vs_tls', consistent: true }],
  };
  const LAYERS = [['L1', 'static'], ['L2', 'fingerprint'], ['L3', 'integrity'], ['L4', 'behavioral'], ['L5', 'network'], ['L6', 'cross-check'], ['L7', 'scoring']];
  const esc = (s) => String(s == null ? '' : s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
  const $ = (id) => document.getElementById(id);
  const vclass = (v) => v === 'DENY' ? 'deny' : v === 'CHALLENGE' ? 'challenge' : v === 'ALLOW' ? 'allow' : 'unknown';
  const vshape = (v) => v === 'DENY' ? '<svg class="vshape" viewBox="0 0 14 14"><rect width="14" height="14" fill="currentColor"/></svg>'
    : v === 'ALLOW' ? '<svg class="vshape" viewBox="0 0 14 14"><circle cx="7" cy="7" r="7" fill="currentColor"/></svg>' : '';
  const sigClass = (v) => v === 'BOT' ? 'flag' : v === 'SUSPICIOUS' ? 'noted' : v === 'OK' ? 'clear' : 'unk';
  const sc = full.scoring, v = sc.verdict, risk = sc.riskScore;
  const gauge = $('gauge'); gauge.style.setProperty('--pct', risk); gauge.className = 'gauge ' + vclass(v);
  $('bignum').textContent = risk.toFixed(1); $('bignum').className = 'num ' + vclass(v);
  const vt = $('verdict'); vt.className = 'vt-main ' + vclass(v); vt.innerHTML = vshape(v) + esc(v);
  $('vband').textContent = 'hard-rule override · ' + sc.hardRuleFired + ' → ' + v;
  $('vsub').innerHTML = 'This session looks automated — hard rule <b>' + sc.hardRuleFired + '</b> overrode the score. <span class="faint">risk ' + risk + ' / 100 · policy ' + sc.policyVersion + '</span>';
  const byLayer = {}; LAYERS.forEach((l) => (byLayer[l[0]] = []));
  full.client.signals.concat(full.network.signals).forEach((s) => (byLayer[s.layer] = byLayer[s.layer] || []).push(s));
  let html = '';
  LAYERS.forEach(([L, name]) => {
    const sigs = (byLayer[L] || []).filter((s) => (s.score || 0) > 0 || s.verdict === 'BOT' || s.verdict === 'SUSPICIOUS');
    let chips = sigs.map((s) => '<span class="sig ' + sigClass(s.verdict) + '">' + esc(s.id) + (s.score > 0 ? ' <b>+' + s.score.toFixed(0) + '</b>' : '') + '</span>').join('');
    if (L === 'L5') chips = '<span class="sig srv">ja4=' + esc(full.network.ja4Engine + ' · ' + full.network.ja4).slice(0, 40) + '</span>' + chips;
    if (L === 'L6') chips += full.crosschecks.map((x) => '<span class="xc ' + (x.consistent ? 'good' : 'bad') + '">' + esc(x.id) + ' · ' + (x.consistent ? 'consistent' : 'inconsistent') + '</span>').join('');
    if (L === 'L7') chips += '<span class="sig srv">Σ ' + risk.toFixed(1) + ' → ' + esc(v) + '</span><span class="sig flag">' + esc(sc.hardRuleFired) + ' fired</span>';
    html += '<div class="lane"><span class="ln">' + L + ' · <b>' + name + '</b></span><div class="sigs">' + (chips || '<span class="empty">clear — nothing suspicious</span>') + '</div>' + (chips ? '' : '<span class="pill clear">clear</span>') + '</div>';
  });
  $('lanes').innerHTML = html; $('result').hidden = false; $('lanesSection').hidden = false;
  $('botverdict').innerHTML = vshape('DENY') + 'DENY · 95';
  $('botcatch').textContent = 'HR-7 fired: headless browser + navigator.webdriver=true → hard DENY';
}

run().catch((e) => { console.error(e); process.exit(1); });
