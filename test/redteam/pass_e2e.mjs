// Functional API e2e for the canonical 3-row humanymous Pass (with axis ① crypto
// preflight). Local target only. Run against a live core on 127.0.0.1:8443.
import { createHash } from 'node:crypto';

const BASE = process.env.BASE || 'https://127.0.0.1:8443';
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

let cookie = '';
async function api(path, opts = {}) {
  const headers = { ...(opts.headers || {}) };
  if (cookie) headers.Cookie = cookie;
  const response = await fetch(BASE + path, { ...opts, headers });
  const setCookie = response.headers.get('set-cookie');
  if (setCookie) cookie = setCookie.split(';')[0];
  return response.json();
}

// Per-run salt: the anti-replay registry persists ~10 min server-side, so fold a
// run-unique fractional into every trace to keep the suite idempotent across re-runs.
const RUN = (Date.now() % 100000) * 0.0001;
const offsets = ch => ch.rows.map(row => ((ch.center - row.keyIndex) % ch.n + ch.n) % ch.n);
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
// axis ①: pay the nonce-bound crypto proof before the puzzle solve is admissible.
async function preflight(nw) {
  const p = nw.preflight.pow;
  return api('/api/pass/pow', {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ bucket: p.bucket, nonce: solvePoW(p), challengeNonce: nw.challengeNonce }),
  });
}
// A plausible accessible-keyboard proof (the blind/AT lane): no pointer
// microstructure, irregular inter-key timing, correct alignment. `variant` perturbs
// the timing so each case is a DISTINCT trace (the anti-replay registry keys on the
// trace, so identical timings across cases would collide and mask later checks).
const keyProof = (nw, variant = 0) => ({
  bucket: nw.bucket, challengeNonce: nw.challengeNonce, offsets: offsets(nw.challenge),
  trusted: true, keys: 5, keyDurs: [141 + variant + RUN, 93, 219 + variant, 118],
  moves: 0, coalesced: 0, pathLen: 0, durations: [], rawT: [], pressures: [],
});
async function fresh() { cookie = ''; return api('/api/pass/new'); }
async function submit(body, strategy = '') {
  const headers = { 'Content-Type': 'application/json' };
  if (strategy) headers['X-HM-Redteam'] = strategy;
  return api('/api/pass/solve', { method: 'POST', headers, body: JSON.stringify(body) });
}

const nw1 = await fresh();
if (!nw1.challenge || !(await preflight(nw1)).ok) throw new Error('challenge or PoW preflight unavailable');
const valid = await submit(keyProof(nw1, 1));
console.log('[1] aligned + nonce-bound PoW + keyboard trace ->', JSON.stringify(valid));
if (!valid.ok) throw new Error('valid accessible keyboard solve rejected');

const nw2 = await fresh();
const noCrypto = await submit(keyProof(nw2, 2), 'e2e-no-crypto'); // deliberately skip preflight
console.log('[2] no cryptographic preflight ->', JSON.stringify(noCrypto));
if (noCrypto.ok) throw new Error('solve without cryptographic preflight accepted');

const nw3 = await fresh(); await preflight(nw3);
const uniform = await submit({ ...keyProof(nw3), keyDurs: [50, 50, 50, 50] }, 'e2e-uniform');
console.log('[3] uniform key timing ->', JSON.stringify(uniform));
if (uniform.ok) throw new Error('uniform key timing accepted');

const nw4 = await fresh(); await preflight(nw4);
const bad = keyProof(nw4, 4); bad.offsets[0] = (bad.offsets[0] + 1) % nw4.challenge.n;
const misaligned = await submit(bad, 'e2e-path-mismatch');
console.log('[4] path/offset mismatch ->', JSON.stringify(misaligned));
if (misaligned.ok) throw new Error('path/offset mismatch accepted');

const nw5 = await fresh(); await preflight(nw5);
const wrongNonce = await submit({ ...keyProof(nw5, 5), challengeNonce: 'replayed-nonce' }, 'e2e-wrong-nonce');
console.log('[5] wrong challenge nonce ->', JSON.stringify(wrongNonce));
if (wrongNonce.ok) throw new Error('wrong challenge nonce accepted');

console.log('\nPASS E2E OK: crypto preflight, alignment, event trace, and nonce binding');
