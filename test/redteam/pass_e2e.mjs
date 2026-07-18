// pass_e2e.mjs — humanymous Pass (3-row alignment, SoT-36) functional e2e.
// Fetches a challenge, computes the aligning offsets (the answer is derivable — that
// is by design for accessibility), submits with a valid keyboard real-event proof and
// expects a pass; then confirms the real-event pre-filter rejects synthetic cases.
const BASE = process.env.BASE || 'https://127.0.0.1:8443';
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

let cookie = '';
async function api(path, opts = {}) {
  const h = { ...(opts.headers || {}) }; if (cookie) h.Cookie = cookie;
  const r = await fetch(BASE + path, { ...opts, headers: h });
  const sc = r.headers.get('set-cookie'); if (sc) cookie = sc.split(';')[0];
  return r.json();
}
const solve = (ch) => ch.rows.map(row => ((ch.center - row.keyIndex) % ch.n + ch.n) % ch.n);
const keyProof = () => ({ trusted: true, keys: 8, keyDurs: [140, 90, 220, 110, 300, 80, 160], moves: 0, coalesced: 0, pathLen: 0, durations: [], rawT: [], pressures: [] });

const nw = await api('/api/pass/new');
if (!nw.challenge) { console.error('FAIL: no challenge', nw); process.exit(1); }

const r1 = await api('/api/pass/solve', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ bucket: nw.bucket, offsets: solve(nw.challenge), ...keyProof() }) });
console.log('[1] aligned + keyboard proof ->', JSON.stringify(r1));
if (!r1.ok) { console.error('FAIL: valid keyboard solve rejected'); process.exit(1); }

cookie = ''; const nw2 = await api('/api/pass/new');
const r2 = await api('/api/pass/solve', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ bucket: nw2.bucket, offsets: solve(nw2.challenge), ...keyProof(), trusted: false }) });
console.log('[2] untrusted ->', JSON.stringify(r2));
if (r2.ok) { console.error('FAIL: untrusted accepted'); process.exit(1); }

cookie = ''; const nw3 = await api('/api/pass/new');
const r3 = await api('/api/pass/solve', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ bucket: nw3.bucket, offsets: solve(nw3.challenge), trusted: true, keys: 8, keyDurs: [50, 50, 50, 50, 50, 50], moves: 0, coalesced: 0, pathLen: 0, durations: [], rawT: [], pressures: [] }) });
console.log('[3] uniform key timing ->', JSON.stringify(r3));
if (r3.ok) { console.error('FAIL: uniform key timing accepted'); process.exit(1); }

cookie = ''; const nw4 = await api('/api/pass/new');
const bad = solve(nw4.challenge); bad[0] = (bad[0] + 1) % nw4.challenge.n;
const r4 = await api('/api/pass/solve', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ bucket: nw4.bucket, offsets: bad, ...keyProof() }) });
console.log('[4] misaligned ->', JSON.stringify(r4));
if (r4.ok) { console.error('FAIL: misaligned accepted'); process.exit(1); }

console.log('\n=== ALL PASS-E2E CHECKS OK: alignment verify + real-event pre-filter (keyboard lane) ===');
