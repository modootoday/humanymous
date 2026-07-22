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
  collect(report) { // establish the session's engine verdict via the telemetry pipeline
    return this.api('/api/collect', {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(report),
    });
  }
  attest(nw) { // axis ①: fetch a rate-limited attestation token for this instance
    return this.api('/api/pass/attest', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ challengeNonce: nw.challengeNonce }),
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
// A forgery with a MACHINE tell: sub-15ms inter-key latency (no human presses arrow
// keys that fast). The salt keeps it a fresh trace while staying < 15ms.
function machineProof(nw, variant = 0) {
  return {
    bucket: nw.bucket, challengeNonce: nw.challengeNonce, offsets: solve(nw.challenge), trusted: true,
    keys: 5, keyDurs: [7 + (RUN % 1) * 0.3, 5, 9, 6], // mean < 15ms + non-zero variance (passes the pre-filter)
    moves: 0, coalesced: 0, pathLen: 0, durations: [], rawT: [], pressures: [],
  };
}
// A mobile-claiming forgery: touch pointer + pressure samples but NO device-motion.
// A real phone always carries hand-tremor micro-motion; its absence is the tell.
function mobileProof(nw) {
  return {
    bucket: nw.bucket, challengeNonce: nw.challengeNonce, offsets: solve(nw.challenge), trusted: true,
    pointerType: 'touch', pressures: [0.41 + (RUN % 1) * 0.02, 0.5, 0.46, 0.52, 0.44],
    moves: 12, coalesced: 31, pathLen: 141,
    durations: [10.3, 18.1, 9.6, 22.4, 15.2, 12.8, 27.1, 11.5], // fractional → no integer-timing flag
    rawT: [1, 7, 15, 24, 34, 47, 55, 68, 82, 99, 117, 138].map((n) => n + RUN),
    keys: 0, keyDurs: [], motion: [], // NO device-motion → mobile inconsistency
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
    // Separate variants by a WHOLE ms so distinct traces stay distinct after the
    // server quantizes the digest to 1 ms (anti-replay CWE-294 fix); a sub-ms variant
    // would collide with another trace and be wrongly flagged as a replay.
    durations: [10, 18, 9, 22, 15, 12 + variant, 27, 11], rawT,
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

// NOTE ON ORDERING: in this harness every node client shares ONE JA4|subnet, so the
// velocity governor's fingerprint key is shared. Sessions whose VERDICT we assert
// (the human floor + the engine-fusion demos) run FIRST, on a clean fingerprint,
// before the floods raise the shared velocity into a hard band. A real deployment
// separates human vs bot by JA4, so this ordering is a harness artifact, not a gap.

// ── The human/accessible floor FIRST ───────────────────────────────────────────
const human = new Client(); const humanNew = await human.fresh(); await human.preflight(humanNew);
const humanResult = await human.submit(keyboardProof(humanNew, 19));
console.log(`${humanResult.ok ? '  PASS   ' : '✗ FAIL   '} accessible-keyboard-human  risk=${humanResult.riskScore ?? '?'} ${humanResult.reason || ''}`);

// ── Round 5/6/7: engine fusion LIVE (both directions) ──────────────────────────
// upgrade: a genuine SCORE-based CHALLENGE that solves Pass is UPGRADED to ALLOW —
// the point of the challenge (SoT-36 §3, mirroring PoW). no-laundering: a session
// already proven a bot (selenium+webdriver → HR-1 DENY) solves the puzzle (ok:true —
// it is bot-solvable by design) but the verdict STAYS DENY. Solving Pass is one signal:
// it never fabricates trust a genuine session hadn't earned, and never overrides
// conclusive independent bot evidence.
const chal = new Client('challenge-clears');
await chal.collect({ userAgent: 'myapp/1.0', signals: [] }); // score-based CHALLENGE, no hard rule
const chalNew = await chal.fresh(); await chal.preflight(chalNew);
const chalSolve = await chal.submit(keyboardProof(chalNew, 301));
const upgradeOK = chalSolve.ok === true && chalSolve.verdict === 'ALLOW';
console.log(`challenge-clears-to-allow:  solved=${chalSolve.ok} engineVerdict=${chalSolve.verdict} → ${upgradeOK ? 'UPGRADED to ALLOW (Pass cleared the challenge)' : 'NOT upgraded'}`);

const botUA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36';
const known = new Client('known-bot');
const knownNew = await known.fresh(); await known.preflight(knownNew);
await known.collect({ userAgent: botUA, signals: [
  { id: 'l1.artifact.selenium', verdict: 'BOT', confidence: 1 },
  { id: 'l1.navigator.webdriver', verdict: 'BOT', confidence: 1 },
] });
const knownSolve = await known.submit(keyboardProof(knownNew, 300));
const launderBlocked = knownSolve.ok === true && knownSolve.verdict === 'DENY';
console.log(`known-bot-solves-pass:      cleared=${knownSolve.ok} engineVerdict=${knownSolve.verdict} → ${launderBlocked ? 'NOT laundered (stays DENY)' : 'LAUNDERED — BUG'}\n`);

// ── Round 4: deeper real-event model (SOFT) — machine-speed tell ───────────────
// A forgery with a machine tell (<15ms inter-key). The accessible lane forbids
// BLOCKING on speed (AT/switch devices inject fast), so it still CLEARS — but the
// behavioral model folds l4.key.machine_speed into the score, so its risk sits above
// the human's 0, contributing to defense-in-depth even on a fresh identity.
const mach = new Client('machine-forge'); const mnw = await mach.fresh(); await mach.preflight(mnw);
const machR = await mach.submit(machineProof(mnw));
const machRisk = machR.riskScore ?? 0;
console.log(`machine-forge: clear=${machR.ok} risk=${machRisk} verdict=${machR.verdict} (behavioral tell folded in)`);

// ── Round 8: mobile sensor guard (SOFT) — a touch claim without device-motion ──
// The user-requested guard: a session claiming touch input but carrying NO device-
// motion is inconsistent (a real phone always has hand-tremor micro-jitter). SOFT —
// a mounted phone is low-motion too, so it raises risk (l2.adv.mobile_inconsistent),
// never blocks. A real mobile human sends motion samples and is not flagged.
const mob = new Client('mobile-forge'); const mobnw = await mob.fresh(); await mob.preflight(mobnw);
const mobR = await mob.submit(mobileProof(mobnw));
const mobRisk = mobR.riskScore ?? 0;
console.log(`mobile-forge:  clear=${mobR.ok} risk=${mobRisk} verdict=${mobR.verdict} (touch claim, no device-motion → flagged)\n`);

// ── Axis ③ (round 2) + axis ① identity gate (round 3): volume-forge ────────────
// One session clears Pass with forged keyboard proofs. Round 2 folds l7.pass.velocity
// /flood into the score (PoW cost climbs, verdict turns non-ALLOW). Round 3 goes
// further: once the session cadence is flagged, PoW alone no longer buys an attempt —
// a rate-limited attestation TOKEN is also required. A naive flood holds no token, so
// after the first few fresh attempts it is BLOCKED, not merely taxed. No lockout: the
// window self-clears; a fresh identity may try again — bounded by the issuance rate.
const vol = new Client('volume-forge'); const volStats = [];
for (let i = 0; i < 8; i++) {
  const nw = await vol.fresh();
  const powDiff = nw.preflight.pow.difficulty;
  const attReq = !!(nw.preflight && nw.preflight.attestRequired);
  await vol.preflight(nw);
  const r = await vol.submit(keyboardProof(nw, 100 + i)); // naive: never fetches a token
  volStats.push({ i, powDiff, attReq, ok: !!r.ok, risk: r.riskScore ?? 0, reason: r.reason || '' });
}
console.log('\nvolume-forge (naive flood — cost climbs, then the identity gate BLOCKS it):');
for (const s of volStats) console.log(`  #${s.i} pow=${s.powDiff} attestReq=${s.attReq} clear=${s.ok} risk=${s.risk} ${s.reason}`);
const volCleared = volStats.filter(s => s.ok).length;
const volBlocked = volStats.filter(s => !s.ok).length;
const volGate = volBlocked > 0 && volStats.some(s => /attestation/.test(s.reason));
console.log(`  → ${volCleared} cleared then ${volBlocked} BLOCKED (identity gate ${volGate ? 'engaged' : 'DID NOT engage'})\n`);

// ── Round 3: token-flood — a bot that DOES fetch attestation tokens ─────────────
// Smarter than volume-forge: when attestation is required it fetches a token. But
// issuance is rate-limited PER FINGERPRINT, so its throughput is hard-capped: after
// the per-fingerprint budget is spent the issuer denies, and the attempts are blocked.
// The crypto axis now costs an identity budget, not just CPU.
const tok = new Client('token-flood'); const tokStats = [];
for (let i = 0; i < 8; i++) {
  const nw = await tok.fresh();
  const attReq = !!(nw.preflight && nw.preflight.attestRequired);
  await tok.preflight(nw);
  let token = '';
  if (attReq) { const a = await tok.attest(nw); token = a.ok ? a.token : ''; }
  const r = await tok.submit({ ...keyboardProof(nw, 200 + i), attestToken: token });
  tokStats.push({ i, attReq, gotToken: !!token, ok: !!r.ok, reason: r.reason || '' });
}
console.log('token-flood (fetches tokens — throughput capped by per-fingerprint issuance):');
for (const s of tokStats) console.log(`  #${s.i} attestReq=${s.attReq} token=${s.gotToken} clear=${s.ok} ${s.reason}`);
const tokCleared = tokStats.filter(s => s.ok).length;
const tokBlocked = tokStats.filter(s => !s.ok).length;
const tokCap = tokBlocked > 0; // issuance budget exhausted at least once
console.log(`  → ${tokCleared} cleared then ${tokBlocked} BLOCKED (issuance budget ${tokCap ? 'capped throughput' : 'NOT capped'})\n`);

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
const identityGateOK = volGate && tokCap; // naive flood blocked + token flood throughput-capped
const behaviorOK = machRisk >= 10 && machRisk > (humanResult.riskScore ?? 0); // soft tell folded in, above the human's floor
const mobileOK = mobR.ok === true && mobRisk >= 10; // touch-without-motion flagged (soft), still clears
const engineFusionOK = launderBlocked && upgradeOK; // bot→DENY despite clearing; genuine CHALLENGE→ALLOW
const promotion = postureHeld && humanFloorOK && a11yOK && identityGateOK && behaviorOK && mobileOK && engineFusionOK;
const blocked = results.filter(r => !r.passed).length;
console.log(`\nblocked ${blocked}/${results.length} single-shot classes · posture ${postureHeld ? 'held' : 'DIVERGED'} · human floor ${humanFloorOK ? 'ok' : 'REGRESSED'} · identity-gate ${identityGateOK ? 'engaged' : 'FAILED'} · behavioral ${behaviorOK ? `folded in (machine ${machRisk} vs human 0)` : 'FAILED'} · mobile-guard ${mobileOK ? `flagged (risk ${mobRisk})` : 'FAILED'} · engine-fusion ${engineFusionOK ? 'held (bot→DENY, genuine CHALLENGE→ALLOW)' : 'FAILED'}`);
console.log(`promotion gate: ${promotion ? 'PASS' : 'FAIL'}`);
console.log('rounds 1-8 in force: ① nonce-bound PoW + anti-replay · ③ velocity→PoW-cost + engine risk (no lockout) · ① rate-limited attestation identity-gate · ④ SOFT motor-trace scoring + ⑧ mobile device-motion consistency (both never gate the accessible lane) · ⑤/⑥/⑦ engine fusion verified LIVE both ways — a genuine CHALLENGE→ALLOW, a known bot→DENY despite clearing the puzzle.');
console.log('honest floor: a PERFECT single forgery from a FRESH identity with human-like dynamics still clears the puzzle — it cannot be blocked at the trace level without excluding real AT users. It is now bounded by attestation issuance rate + folded engine risk, and the engine still DENIES it the moment any independent bot signal (JA4/L1-L7/correlation) appears. Remaining wins are engine-side, in the full real-browser-vs-bot redteam suite — not this node puzzle harness.');
if (!promotion) process.exitCode = 1;
