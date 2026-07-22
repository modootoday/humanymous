// assert-proxyproto.mjs — PLAN-08 R4 L4-passthrough consistency gate.
//
// Runs against the compose/proxyproto overlay (HAProxy mode tcp + send-proxy-v2 in
// front of the gate). Proves the gate recovers the REAL client IP from the PROXY v2
// header (not the balancer's) while still terminating TLS itself. Exit non-zero on
// any failure.
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0'; // gate self-signed cert, passed through by HAProxy

const VIA_LB = process.env.VIA_LB || 'https://127.0.0.1:8460'; // HAProxy L4 frontend
const HAPROXY_IP = process.env.HAPROXY_IP || '172.40.0.5'; // pinned balancer IP
const REAL_CLIENT_IP = process.env.REAL_CLIENT_IP || '172.40.0.1'; // edge bridge gateway (the client via SNAT)

let failed = 0;
const check = (name, ok, detail) => { console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ' — ' + detail : ''}`); if (!ok) failed++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function main() {
  // Wait for the passthrough path to come up.
  for (let i = 0; i < 30; i++) {
    const ok = await fetch(VIA_LB + '/', { redirect: 'manual' }).then((r) => r.status > 0).catch(() => false);
    if (ok) break;
    await sleep(1000);
  }

  // 1. TLS is still terminated by the gate through the L4 passthrough (a normal page
  //    loads → the real ClientHello reached the gate → JA3/JA4 capture is unaffected).
  const pageStatus = await fetch(VIA_LB + '/', { redirect: 'manual' }).then((r) => r.status).catch(() => 0);
  check('tls-terminated-through-passthrough', pageStatus === 200, `HTTP ${pageStatus}`);

  // 2. The gate recovered the REAL client IP (the origin echoes the gate's authoritative
  //    X-Forwarded-For == the RemoteAddr it saw). It must equal the real client, NOT the
  //    balancer — otherwise every IP-keyed ban/rate/correlation would collapse onto the LB.
  const seen = (await fetch(VIA_LB + '/whoami').then((r) => r.text()).catch(() => '')).trim();
  check('real-client-ip-recovered', seen === REAL_CLIENT_IP, `gate saw "${seen}" (want the client ${REAL_CLIENT_IP})`);
  check('not-the-balancer-ip', seen !== HAPROXY_IP && seen !== '', `gate saw "${seen}" (must NOT be HAProxy ${HAPROXY_IP})`);

  console.log(`\n=== proxy-protocol L4 passthrough: ${failed === 0 ? 'ALL PASS' : failed + ' FAILED'} ===`);
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((e) => { console.error('proxyproto test error:', e); process.exit(2); });
