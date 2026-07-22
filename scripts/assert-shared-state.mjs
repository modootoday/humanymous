// assert-shared-state.mjs — PLAN-08 R1 cross-node consistency gate.
//
// Proves that with -redis, security state is shared across a gate fleet: a ban
// raised on node A is visible AND enforced on node B, a lift on A clears on B, and
// an auto-ban triggered by a flood on A blocks the same source on B. Run against the
// compose/redis-ha overlay (2 gate nodes + 1 Redis). Exit non-zero on any failure.
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0'; // self-signed edge certs in dev

const A_EDGE = process.env.A_EDGE || 'https://127.0.0.1:8444';
const A_ADMIN = process.env.A_ADMIN || 'https://127.0.0.1:8445';
const B_EDGE = process.env.B_EDGE || 'https://127.0.0.1:8446';
const B_ADMIN = process.env.B_ADMIN || 'https://127.0.0.1:8447';
const OP = process.env.OP_TOKEN || 'e2e-operator-token';

let failed = 0;
function check(name, ok, detail) {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ' — ' + detail : ''}`);
  if (!ok) failed++;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function admin(base, method, path, body) {
  const opts = { method, headers: { Authorization: 'Bearer ' + OP } };
  if (body) { opts.headers['Content-Type'] = 'application/json'; opts.body = JSON.stringify(body); }
  const res = await fetch(base + path, opts);
  const text = await res.text();
  let json; try { json = JSON.parse(text); } catch { json = null; }
  return { status: res.status, json, text };
}
async function listBans(base) {
  const r = await admin(base, 'GET', '/__hmn/admin/bans');
  return (r.json && r.json.bans ? r.json.bans : []).map((b) => b.key);
}

async function main() {
  const KEY = 'ip:203.0.113.7'; // TEST-NET-3 (never a real client)

  // Wait for both admin planes to answer.
  for (let i = 0; i < 30; i++) {
    const a = await admin(A_ADMIN, 'GET', '/__hmn/admin/bans').catch(() => ({ status: 0 }));
    const b = await admin(B_ADMIN, 'GET', '/__hmn/admin/bans').catch(() => ({ status: 0 }));
    if (a.status === 200 && b.status === 200) break;
    await sleep(1000);
  }

  // 1. A ban raised on node A is visible on node B (Add -> Redis -> List).
  await admin(A_ADMIN, 'POST', '/__hmn/admin/bans', { key: KEY, reason: 'r1-consistency', durationSec: 3600 });
  await sleep(300);
  const onB = await listBans(B_ADMIN);
  check('ban-add-on-A-visible-on-B', onB.includes(KEY), `node B bans: [${onB.join(', ')}]`);

  // 2. A lift on node A clears the ban on node B (Lift -> Redis DEL -> List).
  await admin(A_ADMIN, 'POST', '/__hmn/admin/bans/lift?key=' + encodeURIComponent(KEY));
  await sleep(300);
  const onB2 = await listBans(B_ADMIN);
  check('ban-lift-on-A-clears-on-B', !onB2.includes(KEY), `node B bans: [${onB2.join(', ')}]`);

  // 3. SPLIT-FLOOD aggregate: hard threshold is 120 req / 10s. Fire 80 concurrent
  //    requests at EACH node — 80/node is below the per-node hard limit, but 160
  //    aggregate is well over it. A per-node counter would never ban (neither node
  //    alone breaches); the SHARED sliding-window counter aggregates across nodes, so
  //    whichever node crosses 120 auto-bans, and the ban propagates fleet-wide. This
  //    proves both aggregate counting (R1 phase 2) AND cross-node enforcement.
  const controlB = await fetch(B_EDGE + '/', { redirect: 'manual' }).then((r) => r.status).catch(() => 0);
  check('edge-B-open-before-flood', controlB !== 403, `status ${controlB}`);

  const N = 80; // per node; 2N = 160 > hard(120), but N < hard so no per-node breach
  const burst = (base) =>
    Promise.all(Array.from({ length: N }, () => fetch(base + '/', { redirect: 'manual' }).then((r) => r.status).catch(() => 0)));
  const [aCodes, bCodes] = await Promise.all([burst(A_EDGE), burst(B_EDGE)]);
  const banned403 = aCodes.includes(403) || bCodes.includes(403);
  check('split-flood-aggregate-bans', banned403,
    `${N}/node (<120 each) → 403 seen: A=${aCodes.includes(403)} B=${bCodes.includes(403)} (shared counter aggregated 2×${N})`);

  await sleep(300);
  const aBlocked = await fetch(A_EDGE + '/', { redirect: 'manual' }).then((r) => r.status).catch(() => 0);
  const bBlocked = await fetch(B_EDGE + '/', { redirect: 'manual' }).then((r) => r.status).catch(() => 0);
  check('aggregate-ban-enforced-on-both', aBlocked === 403 && bBlocked === 403,
    `node A=${aBlocked} node B=${bBlocked} (both expected 403 via shared ban)`);

  console.log(`\n=== shared-state consistency: ${failed === 0 ? 'ALL PASS' : failed + ' FAILED'} ===`);
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((e) => { console.error('shared-state test error:', e); process.exit(2); });
