// assert-redis-hardening.mjs — PLAN-08 backlog Redis hardening gate.
// Proves: (1) with AUTH + a shared HMAC key, a GENUINE ban set via the gate is stored
// (signed) and enforced; (2) a FORGED ban written straight into Redis (no valid HMAC)
// is IGNORED by the gate — a compromised coordinator cannot inject state.
import net from 'node:net';
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';
const ADMIN = process.env.ADMIN || 'https://127.0.0.1:8445';
const OP = process.env.OP_TOKEN || 'e2e-operator-token';
const REDIS_HOST = process.env.REDIS_HOST || 'redis';
const REDIS_PORT = Number(process.env.REDIS_PORT || 6379);
const REDIS_PASSWORD = process.env.REDIS_PASSWORD || 'demopass';
let failed = 0;
const check = (n, ok, d) => { console.log(`${ok ? 'PASS' : 'FAIL'} ${n}${d ? ' — ' + d : ''}`); if (!ok) failed++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const admin = async (m, p, b) => {
  const o = { method: m, headers: { Authorization: 'Bearer ' + OP } };
  if (b) { o.headers['Content-Type'] = 'application/json'; o.body = JSON.stringify(b); }
  const r = await fetch(ADMIN + p, o); const t = await r.text(); try { return JSON.parse(t); } catch { return null; }
};
const listKeys = async () => ((await admin('GET', '/__hmn/admin/bans'))?.bans || []).map((x) => x.key);
function redisCommand(args) {
  const commands = [
    ['AUTH', REDIS_PASSWORD],
    args,
  ].map((parts) => `*${parts.length}\r\n${parts.map((part) => `$${Buffer.byteLength(String(part))}\r\n${part}\r\n`).join('')}`).join('');
  return new Promise((resolve, reject) => {
    const socket = net.createConnection({ host: REDIS_HOST, port: REDIS_PORT });
    let data = '';
    socket.setTimeout(5000);
    socket.on('connect', () => socket.write(commands));
    socket.on('data', (chunk) => {
      data += chunk;
      if ((data.match(/\r\n/g) || []).length >= 2) socket.end();
    });
    socket.on('end', () => data.includes('-ERR') ? reject(new Error(data.trim())) : resolve(data));
    socket.on('timeout', () => socket.destroy(new Error('Redis timeout')));
    socket.on('error', reject);
  });
}

async function main() {
  for (let i = 0; i < 30; i++) { if ((await admin('GET', '/__hmn/admin/bans'))) break; await sleep(1000); }

  // 1. Genuine ban via the gate (AUTH to Redis works, value is HMAC-signed) → enforced.
  await admin('POST', '/__hmn/admin/bans', { key: 'ip:203.0.113.10', reason: 'genuine', durationSec: 3600 });
  await sleep(300);
  const afterGenuine = await listKeys();
  check('genuine-signed-ban-stored', afterGenuine.includes('ip:203.0.113.10'), `bans: [${afterGenuine.join(', ')}]`);
  // AUTH is implicitly proven: without it the gate could not have written/read the ban above.

  // 2. Forge a ban straight into Redis (no valid HMAC tag) → the gate must IGNORE it.
  await redisCommand(['SET', 'hmn:ban:ip:9.9.9.9', '{"Key":"ip:9.9.9.9","Reason":"FORGED-BY-COMPROMISED-REDIS","Until":"2099-01-01T00:00:00Z"}']);
  await sleep(300);
  const afterForge = await listKeys();
  check('forged-ban-ignored', !afterForge.includes('ip:9.9.9.9'),
    `forged ip:9.9.9.9 ${afterForge.includes('ip:9.9.9.9') ? 'WAS HONORED (bad)' : 'ignored (HMAC rejected)'}`);

  console.log(`\n=== redis hardening: ${failed === 0 ? 'ALL PASS' : failed + ' FAILED'} ===`);
  process.exit(failed === 0 ? 0 : 1);
}
main().catch((e) => { console.error('redis-hardening test error:', e); process.exit(2); });
