import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import {
  EXTENSION_ID,
  FIXTURE_ORIGIN,
  FIXTURE_PATH,
  METHODS,
  PROTOCOL_VERSION,
  ReplayGuard,
  sanitizeResult,
  validateEnvelope,
} from './extension/protocol.mjs';

const NOW = 1_900_000_000_000;
const SEQUENCES = [
  '10000000-0000-4000-8000-000000000001',
  '10000000-0000-4000-8000-000000000002',
  '10000000-0000-4000-8000-000000000003',
  '10000000-0000-4000-8000-000000000004',
  '10000000-0000-4000-8000-000000000005',
];

function envelope(request, sequenceId = SEQUENCES[0]) {
  return {
    protocolVersion: PROTOCOL_VERSION,
    sequenceId,
    deadlineUnixMs: NOW + 1_000,
    request,
  };
}

test('the exact five typed methods validate without selectors, scripts, or mutation verbs', () => {
  const requests = [
    { method: 'snapshot' },
    { method: 'findByRole', role: 'button', nameToken: 'choice-correct' },
    { method: 'findByTextToken', token: 'synthetic-form' },
    { method: 'rectForNode', nodeId: 'opaque_1' },
    { method: 'visibleState', nodeId: 'opaque_1' },
  ];
  assert.deepEqual(METHODS, requests.map((request) => request.method));
  requests.forEach((request, index) => {
    assert.deepEqual(
      validateEnvelope(envelope(request, SEQUENCES[index]), { now: NOW }).request,
      request,
    );
  });
  for (const request of [
    { method: 'click', nodeId: 'opaque_1' },
    { method: 'snapshot', selector: '*' },
    { method: 'findByTextToken', token: 'operator-secret' },
    { method: 'findByRole', role: 'button', nameToken: 'choice-correct', evaluate: 'document.cookie' },
  ]) {
    assert.throws(() => validateEnvelope(envelope(request), { now: NOW }));
  }
});

test('deadlines, unknown envelope fields, and replayed sequence IDs fail closed', () => {
  assert.throws(
    () => validateEnvelope({ ...envelope({ method: 'snapshot' }), deadlineUnixMs: NOW - 1 }, { now: NOW }),
    /deadline/,
  );
  assert.throws(
    () => validateEnvelope({ ...envelope({ method: 'snapshot' }), deadlineUnixMs: NOW + 2_001 }, { now: NOW }),
    /deadline/,
  );
  assert.throws(
    () => validateEnvelope({ ...envelope({ method: 'snapshot' }), selector: '*' }, { now: NOW }),
    /unknown field/,
  );
  const guard = new ReplayGuard();
  guard.accept(SEQUENCES[0], NOW + 1_000, NOW);
  assert.throws(() => guard.accept(SEQUENCES[0], NOW + 1_000, NOW), /replayed/);
});

test('responses retain only bounded geometry/state and never raw DOM values', () => {
  const lookup = { method: 'findByTextToken', token: 'choice-correct' };
  const target = {
    token: 'choice-correct',
    rect: { x: 10, y: 20, width: 30, height: 40 },
    enabled: true,
    visible: true,
    nodeId: 'opaque_1',
  };
  assert.deepEqual(sanitizeResult(lookup, target), target);
  assert.throws(
    () => sanitizeResult(lookup, { ...target, value: 'typed secret' }),
    /unknown field/,
  );
  assert.throws(
    () => sanitizeResult(lookup, { ...target, token: 'challenge-action' }),
    /does not match/,
  );
  assert.throws(
    () => sanitizeResult(
      { method: 'visibleState', nodeId: 'opaque_1' },
      { visible: true, enabled: true },
    ),
    /incomplete/,
  );
  assert.throws(
    () => sanitizeResult(
      { method: 'snapshot' },
      Array.from({ length: 65 }, () => target),
    ),
    /node bound/,
  );
});

test('MV3 package has a stable ID, nativeMessaging only, and a single Core host match', async () => {
  const manifest = JSON.parse(await readFile(
    new URL('./extension/manifest.json', import.meta.url),
    'utf8',
  ));
  const publicKey = Buffer.from(manifest.key, 'base64');
  const digest = createHash('sha256').update(publicKey).digest().subarray(0, 16);
  const derivedId = [...digest]
    .map((byte) => String.fromCharCode(97 + (byte >> 4), 97 + (byte & 15)))
    .join('');
  assert.equal(manifest.manifest_version, 3);
  assert.equal(derivedId, EXTENSION_ID);
  assert.deepEqual(manifest.permissions, ['nativeMessaging']);
  assert.deepEqual(manifest.host_permissions, ['https://core/*']);
  assert.deepEqual(manifest.content_scripts[0].matches, ['https://core/*']);
  const nativeManifest = JSON.parse(await readFile(
    new URL('./native-host-manifest.json', import.meta.url),
    'utf8',
  ));
  assert.deepEqual(nativeManifest.allowed_origins, [`chrome-extension://${EXTENSION_ID}/`]);
});

test('content script has no DOM action primitive and hard-codes the fixed fixture location', async () => {
  const source = await readFile(new URL('./extension/content-script.js', import.meta.url), 'utf8');
  assert.match(source, new RegExp(FIXTURE_ORIGIN.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  assert.match(source, new RegExp(FIXTURE_PATH.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  assert.doesNotMatch(source, /\.click\s*\(/);
  assert.doesNotMatch(source, /dispatchEvent|insertAdjacent|innerHTML|outerHTML|eval\s*\(|new Function/);
  assert.doesNotMatch(source, /\.value\s*=|setAttribute\s*\(|removeAttribute\s*\(/);
  assert.match(source, /window\.screenX/);
  assert.match(source, /window\.outerHeight\s*-\s*window\.innerHeight/);
  assert.match(source, /rect\.bottom\s*<=\s*0/);
});
