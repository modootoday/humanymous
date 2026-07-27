import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import {
  CORE_SCORE_SCHEMA,
  evaluateScoreTrace,
  SCORE_RECEIPT_SCHEMA,
  SCORE_TRACE_SCHEMA,
} from './score-trace-evaluator.mjs';

const runId = 'run-1';
const profileId = 'external_input_virtual';
const detectorWasmSha256 = 'b'.repeat(64);
const reportSha256 = 'c'.repeat(64);

function coreReceipt(overrides = {}) {
  const value = {
    schemaVersion: CORE_SCORE_SCHEMA,
    runLabel: `external-input/${runId}/${profileId}`,
    runId,
    profileId,
    sessionRef: '0123456789abcdef',
    riskScore: 74.3,
    verdict: 'CHALLENGE',
    hardRuleFired: 'HR-12',
    policyVersion: '1.0.0',
    detectorPath: '/static/detector.wasm',
    detectorWasmSha256,
    engineVersion: 'wasm-test-v1',
    wasmSignalCount: 3,
    reportSha256,
    scoreTraceSha256: '',
    scoreRecomputed: false,
    serverAuthoritative: true,
    ...overrides,
  };
  const material = [
    SCORE_TRACE_SCHEMA,
    value.runLabel,
    value.sessionRef,
    Number(value.riskScore).toFixed(1),
    value.verdict,
    value.hardRuleFired,
    value.policyVersion,
    value.detectorWasmSha256,
    value.engineVersion,
    String(value.wasmSignalCount),
    value.reportSha256,
  ].join('\0');
  value.scoreTraceSha256 = createHash('sha256').update(material).digest('hex');
  return value;
}

async function fixture(overrides = {}) {
  const root = await mkdtemp(join(tmpdir(), 'hmn-score-trace-'));
  const paths = {
    coreReceiptPath: join(root, 'core.score.json'),
    resultPath: join(root, 'result.json'),
    destination: join(root, 'receipt.json'),
  };
  await writeFile(
    paths.coreReceiptPath,
    `${JSON.stringify(coreReceipt(overrides.coreReceipt))}\n`,
  );
  await writeFile(paths.resultPath, `${JSON.stringify({
    runId,
    profileId,
    measurement: {
      verdict: overrides.resultVerdict || 'CHALLENGE',
      finalFrameSha256: 'a'.repeat(64),
    },
  })}\n`);
  return paths;
}

test('independent evaluator binds the sole Core score to framebuffer evidence', async () => {
  const paths = await fixture();
  const receipt = await evaluateScoreTrace({ runId, profileId, ...paths });
  assert.equal(receipt.schemaVersion, SCORE_RECEIPT_SCHEMA);
  assert.equal(receipt.scorer, 'core');
  assert.equal(receipt.scoreRecomputed, false);
  assert.equal(receipt.riskScore, 74.3);
  assert.equal(receipt.verdict, 'CHALLENGE');
  assert.equal(receipt.detectorWasmSha256, detectorWasmSha256);
  assert.deepEqual(JSON.parse(await readFile(paths.destination, 'utf8')), receipt);
});

test('independent evaluator fails closed on framebuffer verdict drift', async () => {
  const paths = await fixture({ resultVerdict: 'ALLOW' });
  await assert.rejects(
    evaluateScoreTrace({ runId, profileId, ...paths }),
    /not bound/,
  );
});

test('independent evaluator fails closed on Core trace drift', async () => {
  const paths = await fixture();
  const receipt = JSON.parse(await readFile(paths.coreReceiptPath, 'utf8'));
  receipt.riskScore = 2;
  await writeFile(paths.coreReceiptPath, `${JSON.stringify(receipt)}\n`);
  await assert.rejects(
    evaluateScoreTrace({ runId, profileId, ...paths }),
    /hash differs/,
  );
});

test('Core receipt must be run-bound, WASM-backed, and server authoritative', async () => {
  for (const coreReceiptOverride of [
    { runLabel: `external-input/other/${profileId}` },
    { runId: 'other' },
    { profileId: 'other' },
    { detectorPath: '/tmp/detector.wasm' },
    { detectorWasmSha256: 'not-a-hash' },
    { wasmSignalCount: 0 },
    { riskScore: '74.3' },
    { scoreRecomputed: true },
    { serverAuthoritative: false },
  ]) {
    const paths = await fixture({ coreReceipt: coreReceiptOverride });
    await assert.rejects(
      evaluateScoreTrace({ runId, profileId, ...paths }),
      /not bound|fields are invalid/,
    );
  }
});

test('Core receipt rejects unknown and duplicate fields', async () => {
  const unknown = await fixture({ coreReceipt: { unknown: true } });
  await assert.rejects(
    evaluateScoreTrace({ runId, profileId, ...unknown }),
    /fields are invalid/,
  );

  const duplicate = await fixture();
  const valid = coreReceipt();
  const raw = JSON.stringify(valid).replace(
    `"runId":"${runId}"`,
    `"runId":"${runId}","runId":"other"`,
  );
  await writeFile(duplicate.coreReceiptPath, raw);
  await assert.rejects(
    evaluateScoreTrace({ runId, profileId, ...duplicate }),
    /duplicate key/,
  );
});

test('controller result rejects duplicate keys and invalid framebuffer hash', async () => {
  const duplicate = await fixture();
  await writeFile(
    duplicate.resultPath,
    `{"runId":"${runId}","runId":"other","profileId":"${profileId}",` +
      '"measurement":{"verdict":"CHALLENGE","finalFrameSha256":"' +
      `${'a'.repeat(64)}"}}`,
  );
  await assert.rejects(
    evaluateScoreTrace({ runId, profileId, ...duplicate }),
    /duplicate key/,
  );

  const invalidHash = await fixture();
  const result = JSON.parse(await readFile(invalidHash.resultPath, 'utf8'));
  result.measurement.finalFrameSha256 = 'invalid';
  await writeFile(invalidHash.resultPath, JSON.stringify(result));
  await assert.rejects(
    evaluateScoreTrace({ runId, profileId, ...invalidHash }),
    /not bound/,
  );
});
