// demo.js — the public "score your browser" demo. Reuses the exact same client
// detection pipeline as loader.js, but renders a polished, READ-ONLY verdict view
// (no launcher, no target field, self-scoring only). Public-safe by construction:
// it can only score THIS browser and show recorded bot fixtures.

import { guardSnapshot } from './injector.js';
import { startCollector, behaviorSummary } from './collector.js';
import { probeEnvironment } from './env.js';
import { collectAdvanced, computeFingerprintId } from './advanced.js';
import { postReport } from './transport.js';
import { initRIT } from './rit.js';
import { solveAndSubmit } from './pow.js';

// Begin observing behavior immediately so there is a human profile by the time
// the visitor presses Score.
startCollector();

const $ = (id) => document.getElementById(id);
const esc = (s) => String(s == null ? '' : s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

const LAYERS = [
  ['L1', 'Static client inspection', 'What your browser says about itself'],
  ['L2', 'Client fingerprinting', 'Rendering, audio, screen, and hardware characteristics'],
  ['L3', 'Client integrity checks', 'Whether browser functions appear modified'],
  ['L4', 'Interaction analysis', 'Pointer, keyboard, scroll, and event observations'],
  ['L5', 'Network and protocol inspection', 'Encrypted-connection, HTTP/2, and header observations'],
  ['L6', 'Consistency checks', 'Whether browser claims agree with server observations'],
  ['L7', 'Risk aggregation and verdict selection', 'How evidence becomes one verdict'],
];

// A recorded headless-bot fixture for the "watch a bot get caught" comparison.
// Static data — the page never launches anything.
const BOT_FIXTURE = {
  verdict: 'DENY', riskScore: 95, hardRuleFired: 'HR-7',
  why: 'A headless browser and another automation indicator produced a DENY verdict.',
};
const RULE_LABELS = {
  'HR-1': 'automation artifact',
  'HR-2': 'browser and network-engine mismatch',
  'HR-7': 'headless browser plus another automation indicator',
  'HR-8': 'stealth browser modification',
  'HR-9': 'browser-control leak plus automation evidence',
  'HR-11': 'hosting-network browser',
  'HR-12': 'no interaction observed',
  'HR-13': 'disabled developer console plus automation evidence',
  'HR-14': 'in-session protocol-fingerprint rotation',
  'HR-15': 'multi-axis identity rotation',
  'HR-16': 'request-body integrity failure',
  'HR-17': 'missing or replayed request-integrity token',
  'HR-18': 'browser claim without execution evidence',
  'HR-19': 'cross-session correlation',
  'HR-20': 'automated-browser interaction signature',
  'HR-21': 'HTTP/2 denial-of-service protection',
  'HR-22': 'advanced browser residual',
  'HR-23': 'Chromium-container residual',
  'HR-24': 'network residual policy',
};
function ruleLabel(id) { return RULE_LABELS[id] || 'enforcement rule'; }
function signalLabel(id) {
  return String(id || 'diagnostic signal')
    .replace(/^l[1-7]\./i, '')
    .replace(/^x\./i, '')
    .replace(/[._-]+/g, ' ');
}

async function loadWasm() {
  const go = new Go();
  const resp = await fetch('/static/detector.wasm');
  const bytes = await resp.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, go.importObject);
  go.run(instance); // registers window.__hmDetect
}

async function scoreBrowser(onProgress) {
  onProgress && onProgress('Establishing a session…');
  await initRIT();
  onProgress && onProgress('Loading the detector…');
  await loadWasm();

  onProgress && onProgress('Inspecting client properties, fingerprints, and integrity…');
  const report = JSON.parse(window.__hmDetect ? window.__hmDetect() : '{}');
  report.environment = await probeEnvironment();
  try {
    report.advanced = await Promise.race([collectAdvanced(), new Promise((r) => setTimeout(() => r({ probed: false }), 2500))]);
  } catch (_) {}
  try { report.fingerprintId = await computeFingerprintId(); } catch (_) {}
  const g = guardSnapshot();
  report.guard = {
    reported: true, evalUsed: g.evalUsed, funcCtor: g.funcCtor, scriptInjected: g.scriptInjected,
    protoPolluted: g.protoPolluted, nativeHooked: g.nativeHooked, consoleDisabled: g.consoleDisabled,
  };

  onProgress && onProgress('Watching your behavior…');
  await new Promise((r) => setTimeout(r, 1500)); // short observation window
  report.behavior = behaviorSummary();

  onProgress && onProgress('Scoring across seven detection stages…');
  let verdict = await postReport(report);
  if (verdict.verdict === 'CHALLENGE' && verdict._powChallenge) {
    try {
      const up = await solveAndSubmit(verdict._powChallenge);
      if (up && up.ok) verdict = { ...verdict, verdict: up.verdict, riskScore: up.riskScore, powUpgraded: true };
    } catch (_) {}
  }

  // Fetch the full merged SessionReport for the read-only seven-stage view.
  let full = null;
  try {
    const rr = await fetch('/api/report/' + encodeURIComponent(verdict.sessionId), { credentials: 'same-origin' });
    if (rr.ok) full = await rr.json();
  } catch (_) {}
  return { verdict, full };
}

// ---- rendering ------------------------------------------------------------

function vclass(v) { return v === 'DENY' ? 'deny' : v === 'CHALLENGE' ? 'challenge' : v === 'ALLOW' ? 'allow' : 'unknown'; }
function vshape(v) {
  if (v === 'DENY') return '<svg class="vshape" viewBox="0 0 14 14" aria-hidden="true"><rect width="14" height="14" fill="currentColor"/></svg>';
  if (v === 'CHALLENGE') return '<svg class="vshape" viewBox="0 0 16 16" aria-hidden="true"><rect x="3" y="3" width="10" height="10" transform="rotate(45 8 8)" fill="currentColor"/></svg>';
  if (v === 'ALLOW') return '<svg class="vshape" viewBox="0 0 14 14" aria-hidden="true"><circle cx="7" cy="7" r="7" fill="currentColor"/></svg>';
  return '';
}
function sigClass(v) { return v === 'BOT' ? 'flag' : v === 'SUSPICIOUS' ? 'noted' : v === 'OK' ? 'clear' : 'unk'; }

function friendlyLine(sc) {
  if (sc.verdict === 'ALLOW') return 'The available browser and server observations were consistent enough for ALLOW.';
  if (sc.verdict === 'CHALLENGE') return 'Some observations require additional verification. The current reference does not provide a complete recovery flow on every route.';
  if (sc.hardRuleFired) return 'This session looks automated — the <b>' + esc(ruleLabel(sc.hardRuleFired)) + '</b> raised the verdict.';
  return 'This session scored as automated.';
}

function render(verdict, full) {
  const sc = (full && full.scoring) || verdict || {};
  const v = sc.verdict || verdict.verdict || 'UNKNOWN';
  const risk = sc.riskScore != null ? sc.riskScore : (verdict.riskScore || 0);

  // gauge + banner
  const gauge = $('gauge');
  const cls = vclass(v);
  const pct = Math.max(0, Math.min(100, Number(risk)));
  gauge.style.setProperty('--pct', pct);
  gauge.className = 'gauge ' + cls;
  $('bignum').textContent = (typeof risk === 'number' ? risk.toFixed(1) : risk);
  $('bignum').className = 'bignum num ' + cls;
  gauge.setAttribute('aria-label', 'Risk ' + risk + ' of 100 — verdict ' + v);
  const vt = $('verdict');
  vt.className = 'vt-main ' + cls;
  vt.innerHTML = vshape(v) + esc(v);
  $('vband').textContent = sc.hardRuleFired
    ? 'enforcement rule · ' + ruleLabel(sc.hardRuleFired) + ' → ' + v
    : 'score band · ' + (v === 'ALLOW' ? 'ALLOW (0–29)' : v === 'CHALLENGE' ? 'CHALLENGE (30–69)' : v === 'DENY' ? 'DENY (70–100)' : '—');
  $('vsub').innerHTML = friendlyLine(sc) + ' <span class="faint">risk ' + esc(String(risk)) + ' / 100 · policy ' + esc(sc.policyVersion || '1.0.0') + '</span>';

  // Seven-stage lanes
  const byLayer = {};
  LAYERS.forEach((l) => (byLayer[l[0]] = []));
  const all = [].concat((full && full.client && full.client.signals) || [], (full && full.network && full.network.signals) || []);
  all.forEach((s) => { const L = s.layer || 'L1'; (byLayer[L] = byLayer[L] || []).push(s); });

  let html = '';
  LAYERS.forEach((l) => {
    const [L, name] = l;
    const sigs = (byLayer[L] || []).filter((s) => (s.score || 0) > 0 || s.verdict === 'BOT' || s.verdict === 'SUSPICIOUS');
    let chips = sigs.map((s) => '<span class="sig ' + sigClass(s.verdict) + '" title="' + esc(s.notes || '') + '">' + esc(signalLabel(s.id)) + (s.score > 0 ? ' <b>+' + Number(s.score).toFixed(0) + '</b>' : '') + '</span>').join('');
    if (L === 'L5' && full && full.network && full.network.ja4) {
      chips = '<span class="sig srv">connection fingerprint observed</span>' + chips;
    }
    if (L === 'L6' && full && full.crosschecks) {
      chips += full.crosschecks.map((x) => '<span class="xc ' + (x.consistent ? 'good' : 'bad') + '">' + esc(signalLabel(x.id)) + ' · ' + (x.consistent ? 'consistent' : 'inconsistent') + '</span>').join('');
    }
    if (L === 'L7') {
      chips += '<span class="sig srv">combined risk ' + (typeof risk === 'number' ? risk.toFixed(1) : risk) + ' → ' + esc(v) + '</span>';
      if (sc.hardRuleFired) chips += '<span class="sig flag">' + esc(ruleLabel(sc.hardRuleFired)) + '</span>';
    }
    const clear = !chips;
    const pill = clear ? '<span class="pill clear">clear</span>' : '';
    html += '<div class="lane"><span class="ln"><b>' + name + '</b></span><div class="sigs">' + (chips || '<span class="empty">clear — nothing suspicious</span>') + '</div>' + pill + '</div>';
  });
  $('lanes').innerHTML = html;

  // reveal + move focus, announce
  $('result').hidden = false;
  $('lanesSection').hidden = false;
  const h = $('verdictHeading');
  if (h) { h.setAttribute('tabindex', '-1'); h.focus(); }
  const live = $('live');
  if (live) live.textContent = 'Verdict: ' + v + '. Risk ' + risk + ' out of 100.';
}

function renderBotCompare() {
  $('botverdict').innerHTML = vshape(BOT_FIXTURE.verdict) + BOT_FIXTURE.verdict + ' · ' + BOT_FIXTURE.riskScore;
  $('botcatch').textContent = ruleLabel(BOT_FIXTURE.hardRuleFired) + ': ' + BOT_FIXTURE.why;
}

// ---- wiring ---------------------------------------------------------------

function setBusy(on, msg) {
  const btn = $('scorebtn');
  btn.disabled = on;
  $('status').textContent = msg || '';
  const region = $('result');
  if (region) region.setAttribute('aria-busy', on ? 'true' : 'false');
}

async function onScore() {
  setBusy(true, 'Scoring…');
  try {
    const { verdict, full } = await scoreBrowser((m) => ($('status').textContent = m));
    render(verdict, full);
    $('scorebtn').textContent = '↻ Re-run the scorer';
    setBusy(false, '');
  } catch (e) {
    setBusy(false, '');
    $('status').textContent = 'Could not complete scoring — a signal may have been blocked (a privacy extension, or you are offline). You can retry.';
  }
}

document.addEventListener('DOMContentLoaded', () => {
  renderBotCompare();
  $('scorebtn').addEventListener('click', onScore);
  const sid = $('sessionid');
  if (sid) sid.textContent = (document.cookie.match(/hsid=([a-f0-9]+)/) || [, '—'])[1].slice(0, 12) + '…';
});
