// assert-core-fleet.mjs — NG-detection Pillar C1: prove FLEET-WIDE (Redis) correlation velocity
// (measured H5). Runs inside the bots image against the core-fleet.yaml overlay (two Core nodes +
// Redis).
//
// The correlation key is fingerprintId|JA4, so every observation for one campaign must present an
// IDENTICAL JA4 to both nodes. We therefore drive all requests over a SINGLE keep-alive connection
// per origin (node:https, maxSockets:1) — undici's default fetch pool opens several connections that
// can negotiate different ALPN/JA4 and split the key, which would be a harness artifact, not a
// detection result. One socket per origin pins one JA4; both nodes see the same client JA4, so their
// counts share one Redis key.
//
// The proof is a split-campaign the per-process registry could not see:
//   1. Warm each node's per-node MAD baseline with organic single-subnet fingerprints (velocity≈1).
//   2. Drive 4 distinct /24s for ONE burst fingerprint at node A — below the per-node floor(8), so A
//      never fires ip_velocity (NEGATIVE control: a single node cannot see the whole campaign).
//   3. Drive 6 more distinct /24s for the SAME fingerprint at node B — B saw only 6 locally, yet the
//      FLEET count crosses the floor, so ip_velocity MUST fire on B. Only possible via shared Redis.
//   4. Both nodes report zero fleet fallbacks (they actually used Redis, not per-process degradation).
//
// l5.correlation.ip_velocity is weight-0 / audit-only, so this measures the AGGREGATION, not an
// enforcement change (detection stays frozen). It reads the raw report signals via the operator
// token, independent of the verdict.
//
//   docker compose -f deployments/compose.yaml -f deployments/compose/core-fleet.yaml \
//       -f deployments/compose/assertions.yaml run --rm assert-core-fleet
import https from 'node:https';

const A = process.env.A || 'https://127.0.0.1:8443';
const B = process.env.B || 'https://127.0.0.1:8449';
const OP = process.env.OP_TOKEN || 'e2e-core-operator-token';
const IP_VEL = 'l5.correlation.ip_velocity';

// One keep-alive socket per origin → one JA4 per node → the campaign shares one correlation key.
const agent = new https.Agent({ keepAlive: true, maxSockets: 1, rejectUnauthorized: false });

let failed = 0;
const check = (name, ok, detail = '') => {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ' — ' + detail : ''}`);
  if (!ok) failed++;
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function req(method, url, { headers = {}, body } = {}) {
  return new Promise((resolve, reject) => {
    const u = new URL(url);
    const r = https.request(
      { hostname: u.hostname, port: u.port, path: u.pathname + u.search, method, headers, agent },
      (res) => {
        let data = '';
        res.on('data', (c) => (data += c));
        res.on('end', () => resolve({ status: res.statusCode, json: () => JSON.parse(data || '{}') }));
      },
    );
    r.on('error', reject);
    if (body) r.write(body);
    r.end();
  });
}

// collect posts one observation for `fp` from the /24 of `xff` and returns the created session id.
async function collect(base, fp, xff) {
  const res = await req('POST', `${base}/api/collect`, {
    headers: { 'Content-Type': 'application/json', 'X-Forwarded-For': xff },
    body: JSON.stringify({ fingerprintId: fp }),
  });
  if (res.status >= 500) throw new Error(`collect ${base} ${res.status}`); // DENY(4xx) is fine; still scored+stored
  return res.json().sessionId;
}

async function reportSignalIDs(base, sid) {
  const res = await req('GET', `${base}/api/report/${sid}`, { headers: { Authorization: 'Bearer ' + OP } });
  if (res.status !== 200) throw new Error(`report ${base}/${sid} ${res.status}`);
  return (res.json().network?.signals || []).map((s) => s.id);
}

async function fleetFallback(base) {
  const res = await req('GET', `${base}/api/counters`, { headers: { Authorization: 'Bearer ' + OP } });
  if (res.status !== 200) throw new Error(`counters ${base} ${res.status}`);
  return res.json().fleet?.redisFallbackTotal;
}

async function waitReady(base) {
  for (let i = 0; i < 30; i++) {
    try { if ((await req('GET', `${base}/healthz`)).status === 200) return true; } catch { /* not up */ }
    await sleep(1000);
  }
  return false;
}

async function main() {
  if (!(await waitReady(A)) || !(await waitReady(B))) {
    console.error('one or both Core nodes never became ready');
    process.exit(2);
  }

  // 1. Warm each node's per-node MAD population baseline (≈30 organic, single-subnet fingerprints).
  for (let i = 0; i < 30; i++) {
    await collect(A, `warm-a-${i}`, `10.${i}.0.9`);
    await collect(B, `warm-b-${i}`, `11.${i}.0.9`);
  }

  const BURST = 'fleet-burst-fp';
  // 2. Node A: 4 distinct /24s for the burst fingerprint — below the per-node floor(8).
  let lastA;
  for (let i = 0; i < 4; i++) lastA = await collect(A, BURST, `203.0.${113 + i}.9`);
  const aSigs = await reportSignalIDs(A, lastA);
  check('node A alone does NOT fire ip_velocity', !aSigs.includes(IP_VEL),
    `A saw 4 /24s (< per-node floor 8); signals=[${aSigs.join(', ')}]`);

  // 3. Node B: 6 more distinct /24s for the SAME fingerprint — B saw only these locally, but the
  //    FLEET count (4 from A + 6 from B) crosses the floor via shared Redis.
  let lastB;
  for (let i = 0; i < 6; i++) lastB = await collect(B, BURST, `203.0.${117 + i}.9`);
  const bSigs = await reportSignalIDs(B, lastB);
  check('node B fires ip_velocity from the FLEET count', bSigs.includes(IP_VEL),
    `B saw only 6 /24s locally yet ip_velocity fired ⇒ the count was shared via Redis; signals=[${bSigs.join(', ')}]`);

  // 4. Both nodes genuinely used Redis (no silent per-process degradation).
  const fbA = await fleetFallback(A);
  const fbB = await fleetFallback(B);
  check('both nodes used Redis (0 fleet fallbacks)', fbA === 0 && fbB === 0,
    `A.redisFallbackTotal=${fbA} B.redisFallbackTotal=${fbB}`);

  console.log(`\n=== assert-core-fleet: ${failed === 0 ? 'ALL PASS' : failed + ' FAILED'} ===`);
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((e) => { console.error('assert-core-fleet harness error:', e); process.exit(2); });
