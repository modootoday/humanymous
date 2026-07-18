// loader.js — orchestrates the client detection flow (plan/02 §4.1):
//   1. injector.js has already run (imported first) → pristine refs + guards
//   2. start behavioral collector
//   3. instantiate the WASM detector, run L1-L3
//   4. probe environment (adblock/tracking)
//   5. after a short observation window, assemble + post the ClientReport
//   6. render the verdict

import { guardSnapshot } from './injector.js';
import { startCollector, behaviorSummary } from './collector.js';
import { probeEnvironment } from './env.js';
import { collectAdvanced, computeFingerprintId } from './advanced.js';
import { postReport } from './transport.js';
import { initRIT } from './rit.js';
import { solveAndSubmit } from './pow.js';

const out = document.getElementById('out');
const log = (o) => { if (out) out.textContent = typeof o === 'string' ? o : JSON.stringify(o, null, 2); };

startCollector();

async function loadWasm() {
  const go = new Go();
  const resp = await fetch('/static/detector.wasm');
  const bytes = await resp.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, go.importObject);
  go.run(instance); // registers window.__hmDetect
}

async function run() {
  log('running WASM detector…');
  await initRIT();          // fetch session + first RIT seed (SoT-07)
  await loadWasm();

  // L1-L3 from WASM.
  const clientJSON = window.__hmDetect ? window.__hmDetect() : '{}';
  const report = JSON.parse(clientJSON);

  // Environment probe (SoT-09).
  report.environment = await probeEnvironment();

  // Advanced fingerprint surfaces (SoT-14). Overall timeout so a hung probe
  // (e.g. in Firefox) can never stall the report.
  try {
    report.advanced = await Promise.race([
      collectAdvanced(),
      new Promise((r) => setTimeout(() => r({ probed: false }), 2500)),
    ]);
  } catch (_) {}

  // Stable per-device fingerprint id for cross-session correlation (SoT-15).
  try { report.fingerprintId = await computeFingerprintId(); } catch (_) {}

  // Runtime guard results (SoT-11).
  const g = guardSnapshot();
  report.guard = {
    reported: true,
    evalUsed: g.evalUsed,
    funcCtor: g.funcCtor,
    scriptInjected: g.scriptInjected,
    protoPolluted: g.protoPolluted,
    nativeHooked: g.nativeHooked,
    consoleDisabled: g.consoleDisabled,
  };

  // Observe behavior, then attach the summary and post. The window is long
  // enough to capture the LLM inference-loop cadence (multi-second pauses); a
  // production detector would observe over the session and re-score (SoT-16).
  log('observing behavior… move the mouse / type to build a human profile');
  await new Promise((r) => setTimeout(r, 6000));
  report.behavior = behaviorSummary();

  let verdict = await postReport(report);

  // If challenged, solve the proof-of-work to prove CPU work and upgrade trust
  // (SoT-13). A real browser solves in well under a second.
  if (verdict.verdict === 'CHALLENGE' && verdict._powChallenge) {
    log('solving proof-of-work challenge…');
    try {
      const upgraded = await solveAndSubmit(verdict._powChallenge);
      if (upgraded && upgraded.ok) verdict = { ...verdict, verdict: upgraded.verdict, riskScore: upgraded.riskScore, powUpgraded: true };
    } catch (_) {}
  }
  log(verdict);
}

run().catch((e) => log('error: ' + e.message));
