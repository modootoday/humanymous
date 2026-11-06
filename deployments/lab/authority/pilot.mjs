// Actual Rust child + installed Express/Hono adapters. No mocked decisions.
// Credentials/keys exist only in private pipes and this process's memory.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { randomBytes, randomUUID, createHash } from 'node:crypto';
import { mkdtemp, mkdir, stat, readdir, unlink, appendFile } from 'node:fs/promises';
import { createInterface } from 'node:readline';
import http from 'node:http';
import { once } from 'node:events';
import { createRequire } from 'node:module';
const require = createRequire('/opt/adapter/package.json');
const framework = require('express');
const { Hono } = require('hono');
const { express, hono, VERSION } = require('/opt/adapter/src/index.cjs');
const cases = [];
function record(id, detail = '') { cases.push({ id, status: 'passed', detail }); }
const privateRoot = await mkdtemp('/tmp/hm-authority-');
await mkdir(`${privateRoot}/ipc`, { mode: 0o700 });
await mkdir(`${privateRoot}/wal`, { mode: 0o700 });
const socketPath = `${privateRoot}/ipc/decision.sock`;
const noCapabilities = { raw_tls: false, original_h2: false, raw_header_order: false, collector: false, rit: false };
const config = {
  endpoint: socketPath,
  scope: { installation: 'owned-lab', tenant: 'synthetic', origin: 'origin', revision: '1', authority: 'owned.invalid' },
  peer: { kind: 'unix', uid: process.getuid() },
  routes: [
    { method: 'POST', path: '/observable', required: noCapabilities },
    { method: 'POST', path: '/protected', required: { ...noCapabilities, raw_tls: true, collector: true, rit: true } },
  ],
  audit: { directory: `${privateRoot}/wal`, node_id: 'owned-lab', hmac_key_hex: randomBytes(32).toString('hex'), signing_seed_hex: randomBytes(32).toString('hex'), allow_new: true, trusted_head: null },
  capacity: 8, replay_capacity: 128, rate_per_second: 100, timeout_ms: 2000, clock_skew_ms: 0,
};
let child, ready, appServer;
const privateValues = [config.audit.hmac_key_hex, config.audit.signing_seed_hex];
async function start() {
  child = spawn('/usr/local/bin/humanymous-authority', [], { stdio: ['pipe', 'pipe', 'pipe'] });
  // Drain stderr, deliberately discard rather than persist private process data.
  child.stderr.resume();
  const lines = createInterface({ input: child.stdout });
  child.stdin.end(JSON.stringify(config));
  const timer = setTimeout(() => child.kill('SIGKILL'), 10000);
  try {
    ready = await Promise.race([
      once(lines, 'line').then(([line]) => JSON.parse(line)),
      once(child, 'exit').then(([code]) => { throw Error(`authority exited before readiness: ${code}`); }),
    ]);
    assert.equal(ready.protocol, VERSION);
    assert.equal(ready.endpoint, socketPath);
    assert.match(ready.credential, /^[A-Za-z0-9_-]{32,256}$/);
    privateValues.push(ready.credential);
  } finally { clearTimeout(timer); lines.close(); }
}
async function stop() {
  if (child && child.exitCode === null && child.signalCode === null) {
    const exited = once(child, 'exit'); child.kill('SIGTERM'); await exited;
  }
}
function envelope(target = '/observable') {
  const body = Buffer.from([0, 255, 42]);
  return { protocol: VERSION, adapter: { id: 'humanymous.node', version: '0.1.0-experimental.1', installation: 'owned-lab' }, connectionId: null, capture: null, id: randomUUID(), tenant: 'synthetic', origin: 'origin', revision: '1', deadline: Date.now() + 15000, method: 'POST', target, authority: 'owned.invalid', peer: null, headers: [['host', 'owned.invalid'], ['content-length', String(body.length)]], headerMode: 'fetch-normalized', body: { layer: 'transfer-decoded-content-encoded', complete: true, length: body.length, sha256: createHash('sha256').update(body).digest('hex'), base64: body.toString('base64') }, capabilities: { rawTls: false, originalH2: false, rawHeaderOrder: false, completeBody: true }, stream: null };
}
async function ipc(message, token = ready.credential, path = '/adapter/v0.1/decision') {
  return new Promise((resolve, reject) => {
    const body = JSON.stringify(message);
    const req = http.request({ socketPath, method: 'POST', path, headers: { host: 'localhost', authorization: `Bearer ${token}`, 'content-type': 'application/json', 'content-length': Buffer.byteLength(body), connection: 'close' } }, res => {
      const chunks = []; res.on('data', x => chunks.push(x));
      res.on('end', () => resolve({ status: res.statusCode, body: Buffer.concat(chunks).toString() }));
      res.on('error', reject);
    });
    req.on('error', reject); req.setTimeout(3000, () => req.destroy(Error('timeout'))); req.end(body);
  });
}
async function expectStatus(id, request, expected, token, path) {
  const response = await ipc(request, token, path);
  assert.equal(response.status, expected, id); record(id, { status: expected }); return response;
}
let effects = 0;
try {
  await start(); record('actual-rust-readiness-private-pipe');
  assert.equal((await stat(socketPath)).mode & 0o777, 0o600); record('unix-socket-mode');
  const original = envelope();
  const decided = await expectStatus('actual-shared-engine-audit-decision', original, 200);
  const decision = JSON.parse(decided.body);
  assert.deepEqual(Object.keys(decision).sort(), ['expires', 'id', 'origin', 'protocol', 'revision', 'tenant', 'verdict']);
  assert.equal(decision.id, original.id);
  assert.ok(['ALLOW', 'DENY', 'CHALLENGE'].includes(decision.verdict));
  record('observed-application-http-verdict', { verdict: decision.verdict, missingBrowserEvidence: true });
  await expectStatus('nonce-replay', original, 409);
  await expectStatus('wrong-credential', envelope(), 401, 'A'.repeat(43));
  await expectStatus('installation-target-binding', { ...envelope(), adapter: { ...original.adapter, installation: 'foreign-lab' } }, 403);
  await expectStatus('wrong-tenant', { ...envelope(), tenant: 'foreign' }, 403);
  await expectStatus('unknown-target-refused', envelope('/unknown'), 409);
  await expectStatus('stale-config', { ...envelope(), revision: '0' }, 409);
  await expectStatus('required-observations-fail-closed', envelope('/protected'), 422);
  const corrupted = envelope(); corrupted.body.sha256 = '0'.repeat(64);
  await expectStatus('body-digest-binding', corrupted, 422);
  const admin = await ipc(envelope(), ready.credential, '/admin'); assert.notEqual(admin.status, 200); record('decision-token-no-admin', { status: admin.status });
  // Actual framework and adapter. No custom decide function or fixture evidence.
  const options = { installation: 'owned-lab', tenant: 'synthetic', origin: 'origin', revision: '1', routes: ['/protected', '/observable'], socketPath, credential: ready.credential };
  const app = framework(); app.use(express(options)); app.post(['/protected', '/observable'], (_req, res) => { effects++; res.end('origin-effect'); });
  appServer = app.listen(0, '127.0.0.1'); await once(appServer, 'listening');
  const appRequest = (path = '/protected') => new Promise((resolve, reject) => {
    const req = http.request({ host: '127.0.0.1', port: appServer.address().port, path, method: 'POST', headers: { host: 'owned.invalid', 'content-length': '0' } }, res => { res.resume(); res.on('end', () => resolve(res.statusCode)); });
    req.on('error', reject); req.end();
  });
  assert.equal(await appRequest(), 503); assert.equal(effects, 0); record('express-real-authority-missing-signals-no-effect');
  assert.equal(await appRequest('/observable'), 409); assert.equal(effects, 0); record('express-observable-challenge-is-not-allow');
  const h = new Hono(); h.post('/protected', c => { effects++; return c.text('origin-effect'); });
  const handler = hono(options, h);
  assert.equal((await handler(new Request('http://owned.invalid/protected', { method: 'POST' }))).status, 503);
  assert.equal(effects, 0); record('hono-real-authority-missing-signals-no-effect');
  const oldCredential = ready.credential;
  await stop();
  assert.equal(await appRequest(), 503); assert.equal(effects, 0); record('authority-loss-fail-closed');
  // A stopped Unix server leaves a path. The trusted owning supervisor removes it.
  await unlink(socketPath).catch(error => { if (error.code !== 'ENOENT') throw error; });
  config.audit.allow_new = false;
  await start();
  assert.notEqual(ready.credential, oldCredential); record('restart-credential-rotation');
  await expectStatus('old-epoch-refused-after-restart', envelope(), 401, oldCredential);
  await expectStatus('authenticated-wal-restart', envelope(), 200);
  const walFiles = await readdir(`${privateRoot}/wal`);
  assert.ok(walFiles.length > 0); record('actual-wal-created', { fileCount: walFiles.length });
  await stop();
  await unlink(socketPath).catch(error => { if (error.code !== 'ENOENT') throw error; });
  const segment = walFiles.find(name => /^audit-\d{10}\.log$/.test(name));
  assert.ok(segment);
  await appendFile(`${privateRoot}/wal/${segment}`, '{"corrupt":true}\n');
  await assert.rejects(start, /exited before readiness/);
  await assert.rejects(stat(socketPath), { code: 'ENOENT' });
  assert.equal(await appRequest(), 503); assert.equal(effects, 0);
  record('corrupt-wal-refuses-listener-and-origin-effect');
  // No raw envelopes, audit keys or credential values are returned as evidence.
} catch (error) {
  let detail = String(error.message);
  for (const secret of privateValues) detail = detail.replaceAll(secret, '[redacted]');
  cases.push({ id: 'execution', status: 'failed', detail: detail.slice(0, 300) });
  process.exitCode = 1;
} finally {
  appServer?.closeAllConnections();
  if (appServer) await new Promise(resolve => appServer.close(resolve));
  await stop();
  console.log(JSON.stringify({ kind: 'rust-authority-node-ipc', cases, effects, independentWitness: false, durableContainerRecreation: false, fullGate: false, releaseReady: false }));
}
