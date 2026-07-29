// assert-ml.mjs — SoT-42 Pillar A: prove the SIGNED behavioral-model bundle was ADMITTED and is
// SERVING (measured H4). Runs inside the bots image against the ml.yaml overlay's Core.
//
// This is a non-vacuous admission proof, not a "did it boot" check: the Core only reports
// ml.enabled with a real activeBundle if BundleManager.Stage passed the feature-schema pin AND the
// ed25519 signature verify, then Promote put it in front of traffic. A broken signature, a stale
// schema, or a missing key leaves the seam abstaining → ml.enabled:false → this FAILS.
//
//   docker compose -f deployments/compose.yaml -f deployments/compose/ml.yaml \
//       -f deployments/compose/assertions.yaml run --rm assert-ml
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0'; // self-signed lab edge cert

const CORE = process.env.CORE || 'https://127.0.0.1:8443';
const OP = process.env.OP_TOKEN || 'e2e-core-operator-token';

let failed = 0;
const check = (name, ok, detail = '') => {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ' — ' + detail : ''}`);
  if (!ok) failed++;
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function counters() {
  const res = await fetch(`${CORE}/api/counters`, { headers: { Authorization: 'Bearer ' + OP } });
  if (!res.ok) throw new Error(`/api/counters ${res.status}`);
  return res.json();
}

async function main() {
  // Readiness: the Core admits the model at startup, so once /api/counters answers 200 the
  // admission decision is already made.
  let c = null;
  for (let i = 0; i < 30; i++) {
    try { c = await counters(); break; } catch { await sleep(1000); }
  }
  if (!c) { console.error('core /api/counters never became ready'); process.exit(2); }

  const ml = c.ml || {};
  check('ml.enabled', ml.enabled === true,
    `enabled=${ml.enabled} (a failed signature/schema verify would leave this false)`);
  check('ml.activeBundle is a served bundle', typeof ml.activeBundle === 'string' && ml.activeBundle !== '' && ml.activeBundle !== 'none',
    `activeBundle=${JSON.stringify(ml.activeBundle)} (proves Stage→Promote installed the SIGNED bundle)`);
  check('ml.canaryState armed', ml.canaryState === 'armed',
    `canaryState=${JSON.stringify(ml.canaryState)} (the -ml-canary probation is active, not tripped)`);
  // The ml overlay has no -redis, so fleet correlation is per-process and must report zero fallbacks.
  const fleet = c.fleet || {};
  check('fleet.redisFallbackTotal present', typeof fleet.redisFallbackTotal === 'number',
    `redisFallbackTotal=${fleet.redisFallbackTotal}`);

  console.log(`\n=== assert-ml: ${failed === 0 ? 'ALL PASS' : failed + ' FAILED'} ===`);
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((e) => { console.error('assert-ml harness error:', e); process.exit(2); });
