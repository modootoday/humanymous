// assert-audit-clickhouse.mjs — PLAN-08 R6 durable audit projection gate.
//
// Proves the gate mirrors its tamper-evident audit records into ClickHouse and that
// the projected rows are cross-checkable against the signed chain: a record's
// record_hash in ClickHouse must equal the same record's record_hash reported by the
// gate admin API. Run against the compose/audit-ch overlay. Exit non-zero on failure.
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

const EDGE = process.env.EDGE || 'https://127.0.0.1:8444';
const ADMIN = process.env.ADMIN || 'https://127.0.0.1:8445';
const CH = process.env.CH || 'http://127.0.0.1:8123';
const OP = process.env.OP_TOKEN || 'e2e-operator-token';

let failed = 0;
const check = (name, ok, detail) => { console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ' — ' + detail : ''}`); if (!ok) failed++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const chQuery = async (sql) => (await fetch(CH + '/?query=' + encodeURIComponent(sql)).then((r) => r.text()).catch(() => '')).trim();

async function main() {
  // Wait for ClickHouse + the table.
  for (let i = 0; i < 60; i++) {
    if ((await chQuery('SELECT 1')) === '1' && (await chQuery('EXISTS TABLE audit_log')) === '1') break;
    await sleep(1000);
  }
  check('clickhouse-table-ready', (await chQuery('EXISTS TABLE audit_log')) === '1', 'audit_log table present');

  // Generate edge traffic → the gate emits verdict records → projected to ClickHouse.
  for (let i = 0; i < 30; i++) await fetch(EDGE + '/', { redirect: 'manual' }).catch(() => {});
  // The projection flushes on a ~1s ticker; give it a couple cycles.
  await sleep(3000);

  const count = parseInt(await chQuery('SELECT count() FROM audit_log'), 10) || 0;
  check('records-projected-to-clickhouse', count > 0, `${count} rows in audit_log`);

  // Cross-check: take a seq that is DEFINITELY projected (the max in ClickHouse), read
  // its record_hash from ClickHouse AND from the gate admin, and assert they match —
  // proving the projection is faithful and each row is cross-checkable against the
  // signed chain (record_hash is the Merkle leaf). Anchoring on max(seq) avoids racing
  // the ~1s projection flush.
  const maxSeq = parseInt(await chQuery('SELECT max(seq) FROM audit_log'), 10) || 0;
  const chHash = await chQuery(`SELECT record_hash FROM audit_log WHERE seq = ${maxSeq} LIMIT 1`);
  const audit = await fetch(ADMIN + `/__hmn/admin/audit?limit=200`, { headers: { Authorization: 'Bearer ' + OP } })
    .then((r) => (r.ok ? r.json() : null)).catch(() => null);
  const gateRec = ((audit && audit.records) || []).find((r) => r.seq === maxSeq);
  check('projected-record_hash-matches-chain', !!gateRec && chHash !== '' && chHash === gateRec.record_hash,
    `seq ${maxSeq}: ClickHouse=${chHash.slice(0, 16)}… gate=${((gateRec && gateRec.record_hash) || '').slice(0, 16)}…`);

  console.log(`\n=== durable audit projection: ${failed === 0 ? 'ALL PASS' : failed + ' FAILED'} ===`);
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((e) => { console.error('audit-clickhouse test error:', e); process.exit(2); });
