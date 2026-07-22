// assert-privacypass.mjs — PLAN-08 R2 Privacy Pass PAT verification gate.
//
// Mints a valid RFC 9578 Private Access Token (token type 0x0002) with the demo issuer
// key and proves the gate trust-upgrades it and records privacypass.token.verified —
// while a tampered token is NOT verified. Run against the compose/privacypass overlay.
import crypto from 'node:crypto';
import fs from 'node:fs';
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

const EDGE = process.env.EDGE || 'https://127.0.0.1:8444';
const ADMIN = process.env.ADMIN || 'https://127.0.0.1:8445';
const OP = process.env.OP_TOKEN || 'e2e-operator-token';
const PRIV_PEM = process.env.PRIV_PEM || 'deployments/patissuers/issuer-priv.pem';

if (!fs.existsSync(PRIV_PEM)) {
  console.error(`missing ${PRIV_PEM} — run: go run ./scripts/gen-demo-keys`);
  process.exit(2);
}
const priv = crypto.createPrivateKey(fs.readFileSync(PRIV_PEM));
const pub = crypto.createPublicKey(priv);
const keyID = crypto.createHash('sha256').update(pub.export({ type: 'spki', format: 'der' })).digest(); // token_key_id (32B)

// Build a token: type(0x0002) || nonce[32] || challenge_digest[32] || token_key_id[32] || authenticator.
function mintToken() {
  const head = Buffer.concat([Buffer.from([0x00, 0x02]), crypto.randomBytes(32), Buffer.alloc(32), keyID]); // random nonce (single-use)
  const sig = crypto.sign('sha384', head, { key: priv, padding: crypto.constants.RSA_PKCS1_PSS_PADDING, saltLength: 48 });
  return Buffer.concat([head, sig]).toString('base64');
}

let failed = 0;
const check = (name, ok, detail) => { console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ' — ' + detail : ''}`); if (!ok) failed++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const verifiedCount = async () => {
  const a = await fetch(ADMIN + '/__hmn/admin/audit?limit=200', { headers: { Authorization: 'Bearer ' + OP } })
    .then((r) => (r.ok ? r.json() : null)).catch(() => null);
  return ((a && a.records) || []).filter((r) => r.event_type === 'privacypass.token.verified').length;
};

async function main() {
  for (let i = 0; i < 30; i++) {
    if (await fetch(EDGE + '/', { redirect: 'manual' }).then((r) => r.status > 0).catch(() => false)) break;
    await sleep(1000);
  }

  // 1. A valid PAT from the trusted issuer → trust-upgrade (forwarded).
  const token = mintToken();
  const st = await fetch(EDGE + '/', { headers: { Authorization: `PrivateToken token="${token}"` }, redirect: 'manual' })
    .then((r) => r.status).catch(() => 0);
  check('valid-pat-trust-upgraded', st === 200, `HTTP ${st}`);

  await sleep(300);
  const afterValid = await verifiedCount();
  check('audit-records-pat-verified', afterValid >= 1, `${afterValid} privacypass.token.verified event(s)`);

  // 1b. Replaying the SAME valid token is a double-spend → no new verified event.
  await fetch(EDGE + '/', { headers: { Authorization: `PrivateToken token="${token}"` }, redirect: 'manual' }).catch(() => {});
  await sleep(300);
  const afterReplay = await verifiedCount();
  check('pat-double-spend-rejected', afterReplay === afterValid, `verified count stayed ${afterReplay} (replay rejected)`);

  // 2. A tampered token (flip a signature byte) must NOT verify — no new verified event.
  const raw = Buffer.from(token, 'base64');
  raw[raw.length - 1] ^= 0xff;
  await fetch(EDGE + '/', { headers: { Authorization: `PrivateToken token="${raw.toString('base64')}"` }, redirect: 'manual' }).catch(() => {});
  await sleep(300);
  const afterTamper = await verifiedCount();
  check('tampered-pat-not-verified', afterTamper === afterValid, `verified count stayed ${afterTamper} (tampered token rejected)`);

  console.log(`\n=== privacy-pass PAT verification: ${failed === 0 ? 'ALL PASS' : failed + ' FAILED'} ===`);
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((e) => { console.error('privacypass test error:', e); process.exit(2); });
