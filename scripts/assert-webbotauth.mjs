// assert-webbotauth.mjs — PLAN-08 R3 Web Bot Auth verification gate.
//
// Signs a request with the demo agent key (RFC 9421, covered set "@authority",
// ed25519, tag web-bot-auth) and proves the gate: (1) trust-upgrades a valid signature
// from the allowlisted key, (2) DENIES a forgery of that key, (3) audits both. Run
// against the compose/webbotauth overlay. Exit non-zero on any failure.
import crypto from 'node:crypto';
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

const EDGE = process.env.EDGE || 'https://127.0.0.1:8444';
const ADMIN = process.env.ADMIN || 'https://127.0.0.1:8445';
const OP = process.env.OP_TOKEN || 'e2e-operator-token';
const KID = 'test-agent-key-1';
// The demo private key seed (matches deployments/agentkeys/trusted.txt's public key).
const SEED_HEX = 'ce6027626c5586bfb674334ccd50f05e758162585cfa263ea7bcddd165409a74';

// Build an ed25519 signing key from the raw 32-byte seed via a PKCS8 DER wrapper.
const pkcs8 = Buffer.concat([Buffer.from('302e020100300506032b657004220420', 'hex'), Buffer.from(SEED_HEX, 'hex')]);
const KEY = crypto.createPrivateKey({ key: pkcs8, format: 'der', type: 'pkcs8' });

let failed = 0;
const check = (name, ok, detail) => { console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ' — ' + detail : ''}`); if (!ok) failed++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// authority is the Host the gate will see (host:port), lowercased — matching the
// verifier's buildSignatureBase.
function sign(authority) {
  const created = Math.floor(Date.now() / 1000);
  const inner = `("@authority");created=${created};keyid="${KID}";alg="ed25519";tag="web-bot-auth"`;
  const base = `"@authority": ${authority.toLowerCase()}\n"@signature-params": ${inner}`;
  const sig = crypto.sign(null, Buffer.from(base), KEY).toString('base64');
  return { 'Signature-Input': `sig1=${inner}`, Signature: `sig1=:${sig}:` };
}

async function admin(path) {
  const r = await fetch(ADMIN + path, { headers: { Authorization: 'Bearer ' + OP } });
  return r.ok ? r.json() : null;
}

async function main() {
  const authority = new URL(EDGE).host; // "127.0.0.1:8444"
  for (let i = 0; i < 30; i++) {
    if (await fetch(EDGE + '/', { redirect: 'manual' }).then((r) => r.status > 0).catch(() => false)) break;
    await sleep(1000);
  }

  // 1. Valid signature from the allowlisted key → trust-upgrade (forwarded, not denied).
  const good = sign(authority);
  const goodStatus = await fetch(EDGE + '/', { headers: good, redirect: 'manual' }).then((r) => r.status).catch(() => 0);
  check('valid-agent-signature-allowed', goodStatus === 200, `HTTP ${goodStatus} (trust-upgrade)`);

  // 2. Forge it: flip a byte in the signature but keep the allowlisted keyid → deny.
  const forged = { ...good };
  const b = Buffer.from(forged.Signature.slice(5, -1), 'base64');
  b[0] ^= 0xff;
  forged.Signature = `sig1=:${b.toString('base64')}:`;
  const forgedStatus = await fetch(EDGE + '/', { headers: forged, redirect: 'manual' }).then((r) => r.status).catch(() => 0);
  check('forged-agent-signature-denied', forgedStatus === 403, `HTTP ${forgedStatus} (expected 403 deny)`);

  // 3. The tamper-evident log recorded both outcomes.
  await sleep(300);
  const audit = await admin('/__hmn/admin/audit?limit=200');
  const types = (audit && audit.records ? audit.records : []).map((r) => r.event_type);
  check('audit-records-verified-and-forged',
    types.includes('agent.signature.verified') && types.includes('agent.signature.forged'),
    `verified=${types.includes('agent.signature.verified')} forged=${types.includes('agent.signature.forged')}`);

  console.log(`\n=== web-bot-auth verification: ${failed === 0 ? 'ALL PASS' : failed + ' FAILED'} ===`);
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((e) => { console.error('webbotauth test error:', e); process.exit(2); });
