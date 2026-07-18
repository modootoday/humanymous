// Canonical 3-row humanymous Pass red/blue wargame (SoT-36 §7). Local target only.
//
// The puzzle is intentionally bot-solvable (accessibility → DOM-readable → a script
// can compute the aligning offsets). Security is the FUSED axes, not the puzzle:
//   ① a non-interactive crypto proof (nonce-bound PoW),
//   ② a real-event proof (the hardware fingerprint of the input),
//   ③ engine fusion (out of this harness's scope; L1–L7 + JA4 + rate/retry).
//
// Each red strategy self-labels via X-HM-Redteam so it lands in the wargame KPIs.
// `expected` encodes the CURRENT honest posture: which classes we block, and which
// remain a measured residual frontier. The promotion gate fails loudly if reality
// diverges from that posture (a supposedly-blocked class bypasses, or the human/
// accessible lane regresses).
import { createHash } from 'node:crypto';

const BASE = process.env.BASE || 'https://127.0.0.1:8443';
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

const zeroBits = bytes => {
  let count = 0;
  for (const byte of bytes) {
    if (byte === 0) { count += 8; continue; }
    for (let bit = 7; bit >= 0; bit--) {
      if ((byte & (1 << bit)) === 0) count++; else return count;
    }
  }
  return count;
};
function solvePoW(p) {
  const seed = Buffer.from(p.seed, 'hex');
  for (let i = 0; i < (1 << 24); i++) {
    const digest = createHash('sha256').update(seed).update(String(i)).digest();
    if (zeroBits(digest) >= p.difficulty) return String(i);
  }
  throw new Error('PoW search exhausted');
}
// The oracle: the aligning offsets are derivable from the public challenge — by
// design (a blind user's screen reader can read them, so a script can too).
const solve = ch => ch.rows.map(row => ((ch.center - row.keyIndex) % ch.n + ch.n) % ch.n);

// Per-run salt: the anti-replay registry persists ~10 min server-side. Fold a
// run-unique fractional into every trace so re-runs don't self-collide — but it is
// a MODULE constant, so a given `variant` is stable WITHIN a run (the replay test
// depends on reusing one captured trace verbatim).
const RUN = (Date.now() % 100000) * 0.0001;

class Client {
  constructor(strategy = '') { this.cookie = ''; this.strategy = strategy; }
  async api(path, opts = {}) {
    const headers = { ...(opts.headers || {}) };
    if (this.cookie) headers.Cookie = this.cookie;
    if (this.strategy) headers['X-HM-Redteam'] = this.strategy;
    const response = await fetch(BASE + path, { ...opts, headers });
    const setCookie = response.headers.get('set-cookie');
    if (setCookie) this.cookie = setCookie.split(';')[0];
    return response.json();
  }
  fresh() { return this.api('/api/pass/new'); }
  preflight(nw) {
    const p = nw.preflight.pow;
    return this.api('/api/pass/pow', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ bucket: p.bucket, nonce: solvePoW(p), challengeNonce: nw.challengeNonce }),
    });
  }
  submit(body) {
    return this.api('/api/pass/solve', {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
    });
  }
}

// A forged keyboard proof (the accessible lane a bot must mimic): no pointer
// microstructure, plausibly-irregular inter-key timing. `variant` perturbs the
// timing so each call is a distinct trace (the replay registry keys on the trace).
function keyboardProof(nw, variant = 0) {
  return {
    bucket: nw.bucket, challengeNonce: nw.challengeNonce, offsets: solve(nw.challenge), trusted: true,
    keys: 5, keyDurs: [137 + variant + RUN, 89, 211 + variant, 123],
    moves: 0, coalesced: 0, pathLen: 0, durations: [], rawT: [], pressures: [],
  };
}
// A forged pointer proof: plausible coalesced sub-samples + a monotone, jittered
// raw timestamp stream. `variant` perturbs the stream so it is a fresh trace.
function pointerProof(nw, variant = 0) {
  // RUN shifts the whole series uniformly: diffs (hence stddev + monotonicity) are
  // preserved, but the absolute timestamps — and thus the trace digest — are unique per run.
  const rawT = [1, 7, 15, 24, 34, 47, 55, 68, 82, 99, 117, 138].map((n, i) => n + (i % 3) * variant * 0.01 + RUN);
  return {
    bucket: nw.bucket, challengeNonce: nw.challengeNonce, offsets: solve(nw.challenge), trusted: true,
    moves: 12, coalesced: 31, pathLen: 143 + variant,
    durations: [10, 18, 9, 22, 15, 12 + variant * 0.1, 27, 11], rawT,
    pressures: [], keys: 0, keyDurs: [],
  };
}

const results = [];
async function run(name, fn, expected) {
  const outcome = await fn();
  const passed = !!outcome.ok;
  results.push({ name, passed, expected, reason: outcome.reason || '' });
  const tag = passed ? 'BYPASS ' : 'BLOCKED';
  const flag = passed === expected ? ' ' : '✗'; // ✗ = reality diverged from posture
  console.log(`${flag} ${tag}  ${name.padEnd(26)} ${outcome.reason || ''}`);
  return outcome;
}

console.log('=== humanymous Pass: 3-row red/blue wargame ===\n');

// ── The human/accessible floor FIRST ───────────────────────────────────────────
// Run it before any bot saturates the shared JA4|subnet velocity key, so the
// accessible lane is measured on a clean system (in this harness every node client
// shares one fingerprint — a real deployment separates human vs bot by JA4).
const human = new Client(); const humanNew = await human.fresh(); await human.preflight(humanNew);
const humanResult = await human.submit(keyboardProof(humanNew, 19));
console.log(`${humanResult.ok ? '  PASS   ' : '✗ FAIL   '} accessible-keyboard-human  risk=${humanResult.riskScore ?? '?'} ${humanResult.reason || ''}`);

// ── Axis ③ demonstration: volume-forge ─────────────────────────────────────────
// One session clears Pass repeatedly with forged keyboard proofs. A single forged
// solve IS indistinguishable and passes — but the flooding CADENCE is the tell: the
// engine folds l7.pass.velocity/flood into the score, so (a) the PoW CPU cost climbs
// (crypto axis, never the puzzle → accessibility safe) and (b) the returned verdict
// turns BOT. Solving Pass never launders mass automation. No lockout — nothing stalls.
const vol = new Client('volume-forge'); const volStats = [];
for (let i = 0; i < 8; i++) {
  const nw = await vol.fresh();
  const powDiff = nw.preflight.pow.difficulty;
  await vol.preflight(nw);
  const r = await vol.submit(keyboardProof(nw, 100 + i));
  volStats.push({ i, powDiff, ok: !!r.ok, risk: r.riskScore ?? 0, verdict: r.verdict || '-' });
}
console.log('\nvolume-forge (Pass-clears succeed, but PoW cost + engine risk climb):');
for (const s of volStats) console.log(`  #${s.i} pow=${s.powDiff}  clear=${s.ok}  engineRisk=${s.risk}  verdict=${s.verdict}`);
const powFirst = volStats[0].powDiff, powLast = volStats[volStats.length - 1].powDiff;
const riskPeak = Math.max(...volStats.map(s => s.risk));
const costEscalated = powLast > powFirst;
const engineCaught = riskPeak >= 45; // flood signal (weight 45) lifts risk into the CHALLENGE band
console.log(`  → PoW cost ${powFirst}→${powLast} (${costEscalated ? 'ESCALATED' : 'flat'}); peak engine risk ${riskPeak} (${engineCaught ? 'held at elevated risk, verdict ≠ ALLOW despite clearing' : 'not flagged'})\n`);

// ── Blocked classes (our current defense holds) ────────────────────────────────
await run('read-dom-forge-no-crypto', async () => {
  const c = new Client('read-dom-forge'); const nw = await c.fresh();
  return c.submit(pointerProof(nw, 1)); // never paid axis ①
}, false);

await run('no-attestation-or-pow', async () => {
  const c = new Client('no-attestation'); const nw = await c.fresh();
  return c.submit(keyboardProof(nw, 5)); // keyboard lane, but no crypto proof
}, false);

await run('dispatch-untrusted', async () => {
  const c = new Client('dispatch-untrusted'); const nw = await c.fresh(); await c.preflight(nw);
  return c.submit({ ...pointerProof(nw, 6), trusted: false });
}, false);

let replayTrace;
await run('replay-trace-first-use', async () => {
  const c = new Client('replay-trace'); const nw = await c.fresh(); await c.preflight(nw);
  replayTrace = pointerProof(nw, 4); return c.submit(replayTrace);
}, true); // first use looks human + pays PoW → passes (this is the captured trace)
await run('replay-trace-second-use', async () => {
  const c = new Client('replay-trace'); const nw = await c.fresh(); await c.preflight(nw);
  // reuse the SAME motor trace wrapped around a fresh challenge → registry collision
  return c.submit({ ...replayTrace, bucket: nw.bucket, challengeNonce: nw.challengeNonce, offsets: solve(nw.challenge) });
}, false);

// ── Residual frontier (measured, not yet closed by axes ①/②) ───────────────────
// A bot that PAYS the PoW and forges plausible dynamics still passes: keystroke/
// pointer microstructure is forgeable and the accessible lane forbids gating on
// motor richness. These are pushed down by axis ③ (engine fusion + rate/retry) and
// by rotating the issuer/input axis — out of this harness's scope. Kept honest.
await run('keyboard-forge-with-pow', async () => {
  const c = new Client('keyboard-forge'); const nw = await c.fresh(); await c.preflight(nw);
  return c.submit(keyboardProof(nw, 2));
}, true);
await run('cdp-inject-with-pow', async () => {
  const c = new Client('cdp-inject'); const nw = await c.fresh(); await c.preflight(nw);
  return c.submit(pointerProof(nw, 3));
}, true);

// ── Automated accessibility check on the served page ───────────────────────────
const page = await (await fetch(BASE + '/pass')).text();
const a11yChecks = {
  keyboardGroup: /role="group"/.test(page) && /tabindex="0"/.test(page),
  screenReaderLive: /aria-live="polite"/.test(page),
  rowSliders: /setAttribute\('role','slider'\)/.test(page),
  noCanvas: !/<canvas/i.test(page),
  noAnimationLoop: !/requestAnimationFrame|setInterval/.test(page),
  noTimeLimitCopy: /No timer|no timer/i.test(page),
};
const a11yOK = Object.values(a11yChecks).every(Boolean);
console.log(`\nA11Y ${a11yOK ? 'PASS' : 'FAIL'} ${JSON.stringify(a11yChecks)}`);

// ── Blue KPI scoreboard ────────────────────────────────────────────────────────
const kpi = await (await fetch(BASE + '/api/pass/kpi')).json();
console.log('\n=== blue KPI scoreboard ===');
console.log(`bypass-rate: ${(kpi.bypassRate * 100).toFixed(1)}% over ${kpi.botAttempts} red/bot attempts`);
console.log('per-strategy:', kpi.perStrategy);
console.log('per-difficulty:', kpi.perDifficulty);
console.log(`human pass floor: ${(kpi.humanPassRate * 100).toFixed(1)}% over ${kpi.humanAttempts} attempts`);

// ── Promotion gate ─────────────────────────────────────────────────────────────
const postureHeld = results.every(r => r.passed === r.expected);
const humanFloorOK = humanResult.ok && (kpi.humanAttempts === 0 || kpi.humanPassRate === 1);
const axis3OK = costEscalated && engineCaught; // velocity taxed cost + flipped the verdict
const promotion = postureHeld && humanFloorOK && a11yOK && axis3OK;
const blocked = results.filter(r => !r.passed).length;
console.log(`\nblocked ${blocked}/${results.length} single-shot classes · posture ${postureHeld ? 'held' : 'DIVERGED'} · human floor ${humanFloorOK ? 'ok' : 'REGRESSED'} · axis③ ${axis3OK ? 'engaged (cost↑ + risk↑, verdict ≠ ALLOW)' : 'FAILED'}`);
console.log(`promotion gate: ${promotion ? 'PASS' : 'FAIL'}`);
console.log('round 2 closed: mass forgery no longer launders trust — velocity escalates PoW cost + lifts the engine risk so a flooding session is never cleared to ALLOW, with zero lockout (wargame stays iterable).');
console.log('frontier: a single forged solve from a fresh identity still clears Pass; next rotate the ISSUER axis (rate-limited PAT/attestation token) so each attempt costs an identity, not just CPU.');
if (!promotion) process.exitCode = 1;
