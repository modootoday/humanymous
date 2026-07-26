// e2e.mjs — proxy-layer end-to-end: a demo upstream behind the humanymous proxy.
// Verifies: (1) HTML injection into upstream bytes, (2) control-plane scoring
// updates the sticky verdict, (3) a bot session is DENYed at the edge while the
// upstream is never contacted, (4) the automated baseline is challenged rather
// than misrepresented as a real human session. Local only.
import http from 'node:http';
import https from 'node:https';
import crypto from 'node:crypto';

process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';
const PROXY = process.env.HM_PROXY || 'https://127.0.0.1:8444';
const ADMIN = process.env.HM_ADMIN || 'https://127.0.0.1:8445'; // SEPARATE admin listener (SoT-28 WS1)
const UP_PORT = 9000;
const ORIGIN_KEY = process.env.HM_ORIGIN_KEY || 'demo-origin-secret';
const TOKEN_KEY = Buffer.from(process.env.HM_TOKEN_KEY_HEX || '', 'hex');
// Deterministic dev tokens (must match HMN_ADMIN_TOKENS the gate is started with).
const TOK = { auditor: 'e2e-auditor-token', operator: 'e2e-operator-token', approver: 'e2e-approver-token', dpo: 'e2e-dpo-token' };

// areq issues an authenticated request to the SEPARATE admin listener.
function areq(method, path, { token, body } = {}) {
  return new Promise((resolve, reject) => {
    const u = new URL(ADMIN + path);
    const opts = { method, hostname: u.hostname, port: u.port, path: u.pathname + u.search, headers: {} };
    if (token) opts.headers['Authorization'] = 'Bearer ' + token;
    if (body) opts.headers['Content-Type'] = 'application/json';
    const r = https.request(opts, (res) => { let d = ''; res.on('data', (c) => (d += c)); res.on('end', () => resolve({ status: res.statusCode, body: d })); });
    r.on('error', reject);
    if (body) r.write(JSON.stringify(body));
    r.end();
  });
}

// The origin validates the rotating cloaking token the proxy attaches (SoT-23
// §1, HR-24): originAuth = HMAC-SHA256(key, "hmny-origin|" + epoch). The Gate
// rotates the epoch hourly (internal/gate/guard.go: epoch = "e"+floor(unix/3600)),
// so the origin must derive the SAME time-bucketed epoch and accept the adjacent
// buckets as grace at the hour boundary (mirroring originEpochGrace). A hardcoded
// epoch would reject every forwarded request outside a single hour.
const ORIGIN_EPOCH_WINDOW = 3600; // seconds — must match guard.go originEpochWindow
function originEpoch(offsetBuckets = 0) {
  return 'e' + (Math.floor(Date.now() / 1000 / ORIGIN_EPOCH_WINDOW) + offsetBuckets);
}
function originAuth(epoch) {
  return crypto.createHmac('sha256', ORIGIN_KEY).update('hmny-origin|' + epoch).digest('hex');
}
function validOriginAuth(h) {
  return h === originAuth(originEpoch(0)) || h === originAuth(originEpoch(-1)) || h === originAuth(originEpoch(1));
}

let upstreamHits = 0;
let directHitsRejected = 0;
const upstream = http.createServer((req, res) => {
  // Origin cloaking: accept ONLY proxied traffic carrying a valid token.
  if (!validOriginAuth(req.headers['x-hmny-origin-auth'] || '')) {
    directHitsRejected++;
    res.statusCode = 421; // Misdirected Request — direct-hit bypass
    res.end('direct origin access denied');
    return;
  }
  upstreamHits++;
  if (req.url === '/health') { res.end('ok'); return; }
  res.setHeader('Content-Type', 'text/html; charset=utf-8');
  res.end('<html><head><title>Demo App</title></head><body><h1>Origin content</h1></body></html>');
});

// browserHeaders mimic what a real Chrome fetch/navigation sends, so the
// header-based L5/L6 network analysis sees a browser (the Node client otherwise
// looks non-browser and is correctly penalized).
const browserHeaders = {
  'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36',
  'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8',
  'Accept-Language': 'en-US,en;q=0.9',
  'Accept-Encoding': 'gzip, deflate, br',
  'sec-ch-ua': '"Chromium";v="126", "Not.A/Brand";v="24"',
  'sec-ch-ua-mobile': '?0',
  'sec-ch-ua-platform': '"Windows"',
  'Sec-Fetch-Dest': 'document',
  'Sec-Fetch-Mode': 'navigate',
  'Sec-Fetch-Site': 'none',
};

function req(method, path, { cookie, body, json, headers } = {}) {
  return new Promise((resolve, reject) => {
    const u = new URL(PROXY + path);
    const opts = { method, hostname: u.hostname, port: u.port, path: u.pathname + u.search, headers: { ...(headers || {}) } };
    if (cookie) opts.headers['Cookie'] = cookie;
    if (json) { opts.headers['Content-Type'] = 'application/json'; }
    const r = https.request(opts, (res) => {
      let data = '';
      res.on('data', (d) => (data += d));
      res.on('end', () => resolve({ status: res.statusCode, headers: res.headers, body: data }));
    });
    r.on('error', reject);
    if (body) r.write(json ? JSON.stringify(body) : body);
    r.end();
  });
}

import tls from 'node:tls';

// rawRequest sends raw bytes over a TLS socket to the proxy and returns the raw
// response head — used to exercise request-framing (smuggling) that a normal
// HTTP client would never emit.
function rawRequest(raw) {
  return new Promise((resolve, reject) => {
    const u = new URL(PROXY);
    const sock = tls.connect({ host: u.hostname, port: u.port, rejectUnauthorized: false }, () => sock.write(raw));
    let data = '';
    sock.on('data', (d) => { data += d; if (data.includes('\r\n\r\n')) { sock.end(); } });
    sock.on('end', () => resolve(data));
    sock.on('error', () => resolve(data || 'ERROR'));
    setTimeout(() => { sock.destroy(); resolve(data || 'TIMEOUT'); }, 4000);
  });
}

const setCookie = (res) => (res.headers['set-cookie'] ? res.headers['set-cookie'][0].split(';')[0] : '');

// cookieVal extracts one cookie's name=value pair from a Set-Cookie array.
const cookieVal = (res, name) => {
  const hit = (res.headers['set-cookie'] || []).find((c) => c.startsWith(name + '='));
  return hit ? hit.split(';')[0] : '';
};

// Mint one explicit valid-ALLOW fixture token with the throwaway Gate's test key.
// The automated baseline below must remain CHALLENGE and therefore must not issue
// a token. Unit tests cover issuance on a genuine ALLOW result; this fixture keeps
// the edge fast-path and theft checks independent of pretending automation is human.
function mintVerdictToken(sid, headers, subnet = '127.0.0.0/24') {
  if (TOKEN_KEY.length < 16) throw new Error('HM_TOKEN_KEY_HEX must contain the throwaway Gate test key');
  const bindKey = crypto.createHash('sha256')
    .update(headers['User-Agent'] || '')
    .update('\0')
    .update(headers['Accept-Language'] || '')
    .update('\0')
    .update(headers['sec-ch-ua'] || '')
    .digest()
    .subarray(0, 12)
    .toString('base64url');
  const bind = crypto.createHash('sha256')
    .update(bindKey)
    .update('\0')
    .update(subnet)
    .digest()
    .subarray(0, 16)
    .toString('base64url');
  const epoch = 'e' + Math.floor(Date.now() / 1000 / (15 * 60));
  const expires = Math.floor(Date.now() / 1000) + 30 * 60;
  const payload = `${sid}|${bind}|${expires}|${epoch}`;
  const mac = crypto.createHmac('sha256', TOKEN_KEY).update(payload).digest('base64url');
  return Buffer.from(payload).toString('base64url') + '.' + mac;
}

async function main() {
  await new Promise((r) => upstream.listen(UP_PORT, r));
  const results = [];
  const check = (name, cond, detail) => { results.push({ name, ok: !!cond, detail }); console.log(`${cond ? 'PASS' : 'FAIL'} ${name}${detail ? ' — ' + detail : ''}`); };

  // 1. First GET / (no session yet) — injection into upstream HTML, fail-open.
  const r1 = await req('GET', '/');
  check('html-injection', r1.body.includes('/__hmn/loader.js') && r1.body.includes('Origin content'),
    'injected bundle + preserved origin bytes');
  check('no-store-on-html', (r1.headers['cache-control'] || '').includes('no-store'), r1.headers['cache-control']);

  // 2. Bot beacon: webdriver=true -> scores DENY. The control-plane /collect
  // issues the hsid cookie (as a browser fetch(credentials:include) would).
  const botRep = { userAgent: 'Mozilla/5.0 HeadlessChrome/126', engineVersion: 'x', signals: [
    { id: 'l1.navigator.webdriver', layer: 'L1', value: true, verdict: 'BOT', weight: 40, score: 40, confidence: 0.95, collected: 'js' },
    { id: 'l1.ua.headless_token', layer: 'L1', value: true, verdict: 'BOT', weight: 35, score: 35, confidence: 0.95, collected: 'js' },
    { id: 'l1.window.outer_eq_inner', layer: 'L1', value: true, verdict: 'SUSPICIOUS', weight: 20, score: 20, confidence: 0.8, collected: 'js' },
  ], behavior: { mouse: { samples: 0 }, events: { totalEvents: 0 } } };
  const rb = await req('POST', '/__hmn/collect?label=bot:webdriver', { body: botRep, json: true });
  const cookie = setCookie(rb); // hsid the bot session is scored under
  const bv = JSON.parse(rb.body);
  check('bot-scored-deny', bv.verdict === 'DENY', `verdict=${bv.verdict} rule=${bv.hardRuleFired}`);

  // 3. Next GET / on the bot session -> blocked at edge, upstream NOT hit.
  const hitsBefore = upstreamHits;
  const r3 = await req('GET', '/', { cookie });
  check('bot-blocked-at-edge', r3.status === 403, `status=${r3.status}`);
  check('origin-not-contacted', upstreamHits === hitsBefore, `hits delta=${upstreamHits - hitsBefore}`);

  // 4. The "human" catalog fixture is still driven by Node automation. It is a
  // synthetic friction baseline, not evidence of a person, so CHALLENGE is the
  // expected safe result. The contract is "not denied", never "must ALLOW".
  const humanRep = { userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0 Safari/537', engineVersion: 'x',
    signals: [], behavior: { mouse: { samples: 40 }, events: { totalEvents: 30 } }, environment: { probed: true }, advanced: { probed: true } };
  const rh = await req('POST', '/__hmn/collect', { body: humanRep, json: true, headers: browserHeaders });
  const hc = setCookie(rh);
  const hv = JSON.parse(rh.body);
  const r5 = await req('GET', '/', { cookie: hc, headers: browserHeaders });
  check('synthetic-baseline-challenged', hv.verdict === 'CHALLENGE' && r5.status === 401,
    `verdict=${hv.verdict} status=${r5.status}`);

  // === Red/Blue v2: attacks on the proxy topology (SoT-23/25) ===

  // 5. Origin direct-hit (HR-24): bypass the proxy, hit the origin directly.
  const direct = await new Promise((resolve) => {
    const rr = http.request({ host: '127.0.0.1', port: UP_PORT, path: '/', method: 'GET' }, (x) => {
      let d = ''; x.on('data', (c) => (d += c)); x.on('end', () => resolve({ status: x.statusCode, body: d }));
    });
    rr.end();
  });
  check('origin-direct-hit-blocked', direct.status === 421, `status=${direct.status} (cloaking rejects unproxied)`);

  // 6. Header spoof (HR-27b): client forges X-Forwarded-For to poison source IP.
  const spoofXff = await req('GET', '/', { headers: { ...browserHeaders, 'X-Forwarded-For': '1.2.3.4' } });
  check('spoof-xff-blocked', spoofXff.status === 403, `status=${spoofXff.status}`);

  // 7. Header spoof (HR-27b): client forges our internal origin-auth to impersonate the trusted channel.
  const spoofAuth = await req('GET', '/', { headers: { ...browserHeaders, 'X-Hmny-Origin-Auth': originAuth('e1') } });
  check('spoof-origin-auth-blocked', spoofAuth.status === 403, `status=${spoofAuth.status}`);

  // 8. STRICT route fail-closed without a prior beacon / score (SoT-21 §2):
  //    a fresh session hitting /login must be challenged, not passed through.
  //    (HR-25 "no beacon" hard rule is retired and not shipped.)
  const strict = await req('GET', '/login', { headers: browserHeaders });
  check('strict-route-fail-closed', strict.status === 401, `status=${strict.status} (unscored strict -> challenge)`);

  // === Round 2: verdict-token & proof-replay family (SoT-21 §3-4) ===

  // A challenged baseline must not receive an ALLOW trust token.
  const challengedToken = cookieVal(rh, 'hmn_vt');
  check('challenge-does-not-issue-verdict-token', challengedToken === '', challengedToken ? 'unexpected token' : 'none issued');

  // 9. Exercise the trusted fast-path with an explicit valid-ALLOW fixture token,
  // independently of the automated baseline's correct CHALLENGE verdict.
  const sidCookie = cookieVal(rh, 'hsid');
  const sid = sidCookie.slice('hsid='.length);
  const vtCookie = 'hmn_vt=' + mintVerdictToken(sid, browserHeaders);
  const fixtureCookies = sidCookie + '; ' + vtCookie;
  const tokPass = await req('GET', '/', { cookie: fixtureCookies, headers: browserHeaders });
  check('token-fastpath-allow', tokPass.status === 200 && tokPass.body.includes('Origin content'),
    `status=${tokPass.status} explicit fixture`);

  // 10. Token theft: a bot lifts the fixture token and replays it from a
  //     DIFFERENT fingerprint (bot UA). The binding mismatch means the token is NOT
  //     honored — it is CLEARED and the request re-scored on the bot's own fingerprint.
  //     (A binding mismatch is deliberately NOT a terminal 403: that would lock out a
  //     returning human whose fingerprint/subnet legitimately changed — deep-review
  //     false-403 fix.) The security property is that the stolen token grants no
  //     trusted fast-path — proven by the edge clearing it — not that the request 403s.
  const cleared = (r) => (r.headers['set-cookie'] || []).some((c) => c.startsWith('hmn_vt=;'));
  const botHeaders = { 'User-Agent': 'HeadlessChrome/126', 'Accept': '*/*' };
  const stolen = await req('GET', '/', { cookie: vtCookie, headers: botHeaders });
  check('token-theft-not-honored', cleared(stolen), `cleared=${cleared(stolen)} status=${stolen.status} (binding mismatch -> cleared + re-scored)`);

  // 11. Token FORGERY (HR-28): a fabricated token (no server key) fails the signature
  //     check and is likewise cleared, not honored (never a terminal 403).
  const forged = await req('GET', '/', { cookie: 'hmn_vt=Zm9yZ2Vk.Zm9yZ2Vk', headers: browserHeaders });
  check('token-forgery-not-honored', cleared(forged), `cleared=${cleared(forged)} status=${forged.status} (bad sig -> cleared)`);

  // 12. Beacon REPLAY (HR-29): capture a beacon's single-use nonce and replay it.
  const nonce = 'nonce-' + Math.random().toString(36).slice(2);
  const b1 = await req('POST', '/__hmn/collect', { body: humanRep, json: true, headers: { ...browserHeaders, 'X-Hmn-Nonce': nonce } });
  const b2 = await req('POST', '/__hmn/collect', { body: humanRep, json: true, headers: { ...browserHeaders, 'X-Hmn-Nonce': nonce } });
  check('beacon-replay-rejected', b1.status === 200 && b2.status === 409, `first=${b1.status} replay=${b2.status}`);

  // === Round 3: smuggling / upgrade-tunnel / sweep (SoT-23/21/25) ===

  // 13. Request smuggling (HR-23): a framing-ambiguous request (conflicting
  //     duplicate Content-Length) is rejected end-to-end (Go stdlib normalization
  //     + the HR-23 guard as auditable defense-in-depth; guard logic unit-tested).
  const smug = await rawRequest([
    'POST / HTTP/1.1', 'Host: 127.0.0.1', 'Content-Length: 5', 'Content-Length: 6', '', 'hello',
  ].join('\r\n'));
  check('smuggling-blocked', /^HTTP\/1\.1 40[03]/.test(smug), smug.split('\r\n')[0]);

  // 14. Upgrade tunnel (HR-26): WebSocket upgrade with no prior verdict token.
  const ws = await req('GET', '/', { headers: { ...browserHeaders, 'Upgrade': 'websocket', 'Connection': 'Upgrade', 'Sec-WebSocket-Key': 'x', 'Sec-WebSocket-Version': '13' } });
  check('upgrade-tunnel-blocked', ws.status === 403, `status=${ws.status} (no prior verdict)`);

  // 15. Decision-probing sweep (HR-30): one fingerprint spins up many DISTINCT
  //     sessions to binary-search the decision. Establish 12 fresh sessions
  //     (same fingerprint) then probe — the sweep detector trips (>8/fp/30s).
  let sweptBlocked = false;
  for (let i = 0; i < 12; i++) {
    const sess = await req('GET', '/__hmn/session', { headers: browserHeaders });
    const sc = setCookie(sess);
    const s = await req('GET', '/', { cookie: sc, headers: browserHeaders });
    if (s.status === 403) sweptBlocked = true;
  }
  check('decision-sweep-blocked', sweptBlocked, 'many sessions/one fingerprint -> DENY');

  // WS5: a bare mutation (POST) with no session/beacon fails CLOSED (401) on a
  // balanced route while a safe GET fails open — origin never contacted. Use a
  // distinct fingerprint (unique UA) so the earlier sweep test doesn't taint it.
  const ws5Headers = { ...browserHeaders, 'User-Agent': 'Mozilla/5.0 (Macintosh) AppleWebKit/605 Safari/605 Chrome/125.0 WS5probe' };
  const bareGet = await req('GET', '/cart', { headers: ws5Headers });
  const barePost = await req('POST', '/cart', { headers: ws5Headers, body: 'x=1' });
  check('unsafe-method-fail-closed', bareGet.status === 200 && barePost.status === 401,
    `GET=${bareGet.status} POST=${barePost.status}`);

  // === Round 4: authenticated admin plane (SoT-28 WS1/WS2) ===

  // 16. WS1: the admin plane is NOT on the public edge; and unauthenticated
  //     admin requests on the admin listener are 404 (deny-by-default).
  const pubAdmin = await req('GET', '/__hmn/admin/audit');
  const unauth = await areq('GET', '/__hmn/admin/audit');
  check('admin-off-public-and-authed', pubAdmin.status === 404 && unauth.status === 404,
    `public=${pubAdmin.status} unauth=${unauth.status}`);

  // 17. Operator adds a temporary ban directly (authenticated).
  const addBan = await areq('POST', '/__hmn/admin/bans', { token: TOK.operator,
    body: { Key: 'ip:203.0.113.5', Reason: 'coordinated scraping', DurationSec: 3600, Incident: 'INC-9' } });
  check('console-add-ban', addBan.status === 200, `status=${addBan.status}`);

  // 18. Auditor CANNOT ban (RBAC 403); Auditor CAN list.
  const auditorBan = await areq('POST', '/__hmn/admin/bans', { token: TOK.auditor, body: { Key: 'ip:1.1.1.1', DurationSec: 60 } });
  const listBans = await areq('GET', '/__hmn/admin/bans', { token: TOK.auditor });
  check('rbac-auditor-readonly', auditorBan.status === 403 && listBans.body.includes('203.0.113.5'),
    `auditorBan=${auditorBan.status}`);

  // 19. Real dual-control: a permanent ban is PENDING, not applied; a DISTINCT
  //     approver commits; the requester cannot self-approve.
  const permReq = await areq('POST', '/__hmn/admin/bans', { token: TOK.operator, body: { Key: 'ip:203.0.113.7', DurationSec: 0, Reason: 'abuse' } });
  const approvalId = JSON.parse(permReq.body).approvalId;
  const selfApprove = await areq('POST', '/__hmn/admin/approvals/' + approvalId, { token: TOK.operator });
  const commit = await areq('POST', '/__hmn/admin/approvals/' + approvalId, { token: TOK.approver });
  check('dual-control-two-principal', permReq.body.includes('approvalId') && selfApprove.status === 403 && commit.status === 200,
    `pending=${permReq.status} self=${selfApprove.status} commit=${commit.status}`);

  // 19b. Wargame r01: body Operator/Approver must be ignored — dual-control is
  //      bearer-token only. Spoofed Approver must still leave ban PENDING only.
  const spoofBody = await areq('POST', '/__hmn/admin/bans', {
    token: TOK.operator,
    body: {
      Key: 'ip:198.51.100.77', DurationSec: 0, Reason: 'wargame-r01-body-approver',
      Operator: 'evil-op', Approver: 'already-approved',
    },
  });
  let spoofParsed = {};
  try { spoofParsed = JSON.parse(spoofBody.body); } catch { /* keep empty */ }
  check('permanent-ban-ignores-body-approver',
    spoofBody.status === 200 && spoofParsed.pending === true && !!spoofParsed.approvalId && spoofParsed.needsRole === 'approver',
    `status=${spoofBody.status} body=${spoofBody.body.slice(0, 200)}`);

  // 19c. Wargame r11: broad CIDR key always dual-control even with temporary DurationSec.
  const cidrReq = await areq('POST', '/__hmn/admin/bans', {
    token: TOK.operator,
    body: { Key: 'cidr:198.51.100.0/24', DurationSec: 3600, Reason: 'wargame-r11-cidr' },
  });
  let cidrParsed = {};
  try { cidrParsed = JSON.parse(cidrReq.body); } catch { /* keep */ }
  check('cidr-ban-dual-control',
    cidrReq.status === 200 && cidrParsed.pending === true && !!cidrParsed.approvalId,
    `status=${cidrReq.status} body=${cidrReq.body.slice(0, 200)}`);

  // 19d. Wargame r12: bulk permanent (DurationSec 0) must be rejected (not applied).
  const bulkPerm = await areq('POST', '/__hmn/admin/bans/bulk', {
    token: TOK.operator,
    body: { Keys: ['ip:198.51.100.88'], DurationSec: 0, Reason: 'wargame-r12-bulk-perm' },
  });
  check('bulk-permanent-rejected', bulkPerm.status === 400,
    `status=${bulkPerm.status} body=${bulkPerm.body.slice(0, 120)}`);

  // 19e. Wargame r14: over-cap duration forces dual-control (overflow/permanent ladder).
  const overCap = await areq('POST', '/__hmn/admin/bans', {
    token: TOK.operator,
    body: { Key: 'ip:198.51.100.99', DurationSec: 400 * 24 * 3600, Reason: 'wargame-r14-overcap' },
  });
  let overParsed = {};
  try { overParsed = JSON.parse(overCap.body); } catch { /* keep */ }
  check('overcap-ban-dual-control',
    overCap.status === 200 && overParsed.pending === true && !!overParsed.approvalId,
    `status=${overCap.status} body=${overCap.body.slice(0, 200)}`);

  // 19f. Wargame r21: auditor must not commit permanent-ban approvals (wrong role).
  const auditorCommit = await areq('POST', '/__hmn/admin/approvals/' + spoofParsed.approvalId, { token: TOK.auditor });
  check('auditor-cannot-approve-ban', auditorCommit.status === 403,
    `status=${auditorCommit.status} body=${auditorCommit.body.slice(0, 120)}`);

  // 19g. Wargame r22: negative DurationSec rejected (not reinterpreted as permanent).
  const negDur = await areq('POST', '/__hmn/admin/bans', {
    token: TOK.operator,
    body: { Key: 'ip:198.51.100.66', DurationSec: -1, Reason: 'wargame-r22-neg' },
  });
  check('negative-duration-rejected', negDur.status === 400,
    `status=${negDur.status} body=${negDur.body.slice(0, 120)}`);

  // 19h. Wargame r51+: empty/garbage ban keys rejected (validBanKey harden).
  const emptyIP = await areq('POST', '/__hmn/admin/bans', {
    token: TOK.operator, body: { Key: 'ip:', DurationSec: 60, Reason: 'wargame-empty-ip' },
  });
  const garbageCIDR = await areq('POST', '/__hmn/admin/bans', {
    token: TOK.operator, body: { Key: 'cidr:not-a-cidr', DurationSec: 0, Reason: 'wargame-bad-cidr' },
  });
  check('invalid-ban-key-rejected', emptyIP.status === 400 && garbageCIDR.status === 400,
    `emptyIP=${emptyIP.status} garbageCIDR=${garbageCIDR.status}`);

  // 20. Self-lift closed: unauthenticated lift is 404; operator lift works.
  const unauthLift = await areq('POST', '/__hmn/admin/bans/lift?key=ip:203.0.113.5');
  const lift = await areq('POST', '/__hmn/admin/bans/lift?key=ip:203.0.113.5', { token: TOK.operator });
  check('self-lift-closed', unauthLift.status === 404 && lift.status === 200, `unauth=${unauthLift.status} op=${lift.status}`);

  // === Round 5: live console read APIs (authenticated) ===

  const integ = await areq('GET', '/__hmn/admin/integrity', { token: TOK.auditor });
  const iv = JSON.parse(integ.body);
  check('console-integrity-verifies', iv.ok === true && iv.records > 0, `ok=${iv.ok} records=${iv.records}`);

  // WS8: the chain carries valid independent-witness co-signatures.
  check('integrity-witnessed', iv.witnessed === true, `witnessed=${iv.witnessed}`);

  const stream = await areq('GET', '/__hmn/admin/audit?limit=30', { token: TOK.auditor });
  const sv = JSON.parse(stream.body);
  check('console-audit-stream', sv.count > 0 && Array.isArray(sv.records), `count=${sv.count}`);

  // Privacy (SoT-18 §5): subject identifiers are HMAC-hex pseudonyms, never raw.
  const psns = [...stream.body.matchAll(/"(?:id_pseudonym|session_pseudonym|ja4_pseudonym)":"([^"]+)"/g)].map(m => m[1]);
  const allHashed = psns.length > 0 && psns.every(p => /^[0-9a-f]{64}$/.test(p) || p.startsWith('shredded:'));
  check('console-pseudonymized', allHashed, `${psns.length} subject ids, all pseudonymized`);

  // Meta-audit: the admin reads above are themselves recorded (SoT-28 §9).
  check('admin-reads-meta-audited', stream.body.includes('admin.access'), 'admin.access events present');

  // === Round 6: console SPA + two-phase erasure ===

  const consoleRes = await areq('GET', '/__hmn/admin/console');
  check('console-serves', consoleRes.status === 200 && consoleRes.body.includes('Ledger') && consoleRes.body.includes('view-overview'),
    'live console HTML served + dev token injected');

  const policy = await areq('GET', '/__hmn/admin/policy', { token: TOK.auditor });
  const pv = JSON.parse(policy.body);
  check('console-policy', Array.isArray(pv.routes) && pv.routes.length > 0 && pv.rateLimit.hard > 0,
    `${pv.routes.length} routes, hard=${pv.rateLimit.hard}`);

  // SoT-39 freeze regression: Docker runs Gate with a real Settings store, but
  // its volume starts empty. The effective posture must explicitly report the
  // empty overlay while all proxy conformance checks above remain unchanged.
  const settingsEffective = await areq('GET', '/__hmn/admin/settings/effective', { token: TOK.auditor });
  let settingsState = {};
  try { settingsState = JSON.parse(settingsEffective.body); } catch { /* checked below */ }
  check('settings-empty-overlay-freeze',
    settingsEffective.status === 200 && settingsState.storeAttached === true &&
      settingsState.effective?.emptyOverlay === true && !!settingsState.effective?.configVersion,
    `status=${settingsEffective.status} attached=${settingsState.storeAttached} empty=${settingsState.effective?.emptyOverlay}`);

  // Mutating Settings requests remain server-governed: Auditor is read-only,
  // stale CAS is rejected, and integrity demotion needs the typed confirmation.
  const cfgVersion = settingsState.effective?.configVersion || '';
  const settingsAuditorPost = await areq('POST', '/__hmn/admin/settings/overlays', {
    token: TOK.auditor,
    body: { parentConfigVersion: cfgVersion, scoring: { challengeAt: 20 } },
  });
  const settingsStale = await areq('POST', '/__hmn/admin/settings/overlays', {
    token: TOK.operator,
    body: { parentConfigVersion: 'cfg-stale', scoring: { challengeAt: 20 } },
  });
  const settingsClassCNoConfirm = await areq('POST', '/__hmn/admin/settings/overlays', {
    token: TOK.operator,
    body: { parentConfigVersion: cfgVersion, hardRules: { 'HR-18': 'monitor' } },
  });
  check('settings-mutation-guards',
    settingsAuditorPost.status === 403 && settingsStale.status === 409 && settingsClassCNoConfirm.status === 400,
    `auditor=${settingsAuditorPost.status} stale=${settingsStale.status} classC=${settingsClassCNoConfirm.status}`);

  // Erasure is two-phase + DPO-gated: request → pending; non-DPO approver rejected; DPO commits.
  const eraseReq = await areq('POST', '/__hmn/admin/erasure', { token: TOK.operator, body: { Subject: 'demo-subject-1', LegalBasis: 'GDPR Art.17' } });
  const eraseId = JSON.parse(eraseReq.body).approvalId;
  const nonDpo = await areq('POST', '/__hmn/admin/approvals/' + eraseId, { token: TOK.approver });
  const dpoCommit = await areq('POST', '/__hmn/admin/approvals/' + eraseId, { token: TOK.dpo });
  check('console-erasure-dual-control', eraseReq.body.includes('approvalId') && nonDpo.status === 403 && dpoCommit.status === 200,
    `pending=${eraseReq.status} nonDpo=${nonDpo.status} dpo=${dpoCommit.status}`);

  const finalInteg = JSON.parse((await areq('GET', '/__hmn/admin/integrity', { token: TOK.auditor })).body);
  check('console-chain-still-intact', finalInteg.ok === true, `ok=${finalInteg.ok} records=${finalInteg.records}`);

  // WS9: kill switch — dual-control global-monitor. Operator requests → distinct
  // approver commits → effectiveMonitor true. (Run last; suppresses enforcement.)
  const ksReq = await areq('POST', '/__hmn/admin/killswitch', { token: TOK.operator, body: { On: true } });
  const ksId = JSON.parse(ksReq.body).approvalId;
  const ksSelf = await areq('POST', '/__hmn/admin/approvals/' + ksId, { token: TOK.operator });
  const ksCommit = await areq('POST', '/__hmn/admin/approvals/' + ksId, { token: TOK.approver });
  const polAfter = JSON.parse((await areq('GET', '/__hmn/admin/policy', { token: TOK.auditor })).body);
  check('killswitch-dual-control', ksReq.body.includes('approvalId') && ksSelf.status === 403 && ksCommit.status === 200 && polAfter.effectiveMonitor === true,
    `req=${ksReq.status} self=${ksSelf.status} commit=${ksCommit.status} monitor=${polAfter.effectiveMonitor}`);

  upstream.close();
  const failed = results.filter((r) => !r.ok);
  console.log(`\n=== ${results.length - failed.length}/${results.length} passed ===`);
  process.exit(failed.length ? 1 : 0);
}
main().catch((e) => { console.error(e); process.exit(2); });
