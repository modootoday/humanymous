// assert-webauthn.mjs — PLAN-08 R2 WebAuthn assertion verification gate.
// A Node ECDSA authenticator signs a WebAuthn get() assertion with the demo credential
// key; the gate must verify possession → trust-upgrade + audit, and reject a replay.
import crypto from 'node:crypto';
import fs from 'node:fs';
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

const EDGE = process.env.EDGE || 'https://127.0.0.1:8444';
const ADMIN = process.env.ADMIN || 'https://127.0.0.1:8445';
const OP = process.env.OP_TOKEN || 'e2e-operator-token';
const CRED_ID = 'demo-credential-1';
const priv = crypto.createPrivateKey(fs.readFileSync(process.env.PRIV_PEM || 'deployments/webauthncreds/cred-priv.pem'));

function assertion(counter) {
  const authData = Buffer.alloc(37); authData[32] = 0x05; authData.writeUInt32BE(counter, 33); // rpIdHash|flags|count
  const clientData = Buffer.from(JSON.stringify({ type: 'webauthn.get', challenge: 'Y2g', origin: 'https://example.com' }));
  const cdHash = crypto.createHash('sha256').update(clientData).digest();
  const sig = crypto.sign('sha256', Buffer.concat([authData, cdHash]), priv); // ES256 DER over SHA256(authData||cdHash)
  const j = JSON.stringify({ credentialId: CRED_ID, authenticatorData: authData.toString('base64'),
    clientDataJSON: clientData.toString('base64'), signature: sig.toString('base64') });
  return Buffer.from(j).toString('base64');
}

let failed = 0;
const check = (n, ok, d) => { console.log(`${ok ? 'PASS' : 'FAIL'} ${n}${d ? ' — ' + d : ''}`); if (!ok) failed++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const verifiedCount = async () => {
  const a = await fetch(ADMIN + '/__hmn/admin/audit?limit=200', { headers: { Authorization: 'Bearer ' + OP } })
    .then((r) => (r.ok ? r.json() : null)).catch(() => null);
  return ((a && a.records) || []).filter((r) => r.event_type === 'webauthn.assertion.verified').length;
};

async function main() {
  for (let i = 0; i < 30; i++) { if (await fetch(EDGE + '/', { redirect: 'manual' }).then((r) => r.status > 0).catch(() => false)) break; await sleep(1000); }

  const a1 = assertion(10);
  const st = await fetch(EDGE + '/', { headers: { 'Webauthn-Assertion': a1 }, redirect: 'manual' }).then((r) => r.status).catch(() => 0);
  check('valid-assertion-trust-upgraded', st === 200, `HTTP ${st}`);
  await sleep(300);
  const n1 = await verifiedCount();
  check('audit-records-webauthn-verified', n1 >= 1, `${n1} webauthn.assertion.verified event(s)`);

  // Replay the SAME assertion (counter 10, not advanced) → must NOT verify again.
  await fetch(EDGE + '/', { headers: { 'Webauthn-Assertion': a1 }, redirect: 'manual' }).catch(() => {});
  await sleep(300);
  const n2 = await verifiedCount();
  check('replayed-assertion-rejected', n2 === n1, `verified count stayed ${n2} (replay rejected by counter)`);

  console.log(`\n=== webauthn assertion verification: ${failed === 0 ? 'ALL PASS' : failed + ' FAILED'} ===`);
  process.exit(failed === 0 ? 0 : 1);
}
main().catch((e) => { console.error('webauthn test error:', e); process.exit(2); });
