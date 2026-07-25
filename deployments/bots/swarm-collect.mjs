// swarm-collect.mjs — emit N minimal sessions carrying a shared fingerprintId.
// Each swarm container runs this from a distinct real subnet; the engine's
// cross-session correlation (SoT-15) then sees ONE fingerprint from THREE
// subnets and raises proxy_rotation / shared_fingerprint. Local target only.
import https from 'node:https';
import { appendFileSync } from 'node:fs';

const BASE = process.argv[2];
const FP = process.argv[3] || 'swarm-shared-fp-0001';
const N = Number(process.argv[4] || 5);
const agent = new https.Agent({ rejectUnauthorized: false, keepAlive: false });

function req(method, path, { cookie, body } = {}) {
  return new Promise((resolve, reject) => {
    const u = new URL(BASE + path);
    const headers = {};
    if (cookie) headers.Cookie = cookie;
    if (body) headers['Content-Type'] = 'application/json';
    const r = https.request(
      { hostname: u.hostname, port: u.port, path: u.pathname, method, headers, agent },
      (rs) => { let d = ''; rs.on('data', (c) => (d += c)); rs.on('end', () => resolve({ status: rs.statusCode, headers: rs.headers, body: d })); },
    );
    r.on('error', reject);
    if (body) r.write(body);
    r.end();
  });
}

let completed = 0;
for (let i = 0; i < N; i++) {
  try {
    const s = await req('GET', '/api/session');
    const cookie = (s.headers['set-cookie'] || ['']).map((c) => c.split(';')[0]).join('; ');
    const report = JSON.stringify({ userAgent: 'Mozilla/5.0 (swarm participant)', signals: [], fingerprintId: FP });
    const c = await req('POST', '/api/collect', { cookie, body: report });
    let v = {}; try { v = JSON.parse(c.body); } catch { /* non-JSON */ }
    const corr = (v.topContributors || []).map((t) => t.id).filter((id) => /corr|proxy|shared|ip\.|subnet/i.test(id));
    console.log(`[swarm] #${i} verdict=${v.verdict} risk=${v.riskScore} rule=${v.hardRuleFired || '-'} corr=[${corr.join(',') || '-'}]`);
    completed++;
    if (corr.includes('l5.correlation.proxy_rotation')) {
      appendFileSync('/artifacts/swarm-correlation.ok', `proxy_rotation ${new Date().toISOString()}\n`);
    }
  } catch (e) {
    console.error(`[swarm] #${i} error: ${e.message}`);
  }
}

if (completed !== N) {
  console.error(`FAIL: only ${completed}/${N} swarm requests completed`);
  process.exitCode = 1;
}
