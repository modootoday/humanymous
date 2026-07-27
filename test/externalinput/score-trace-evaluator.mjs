import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { parseStrictJson } from './vusb/strict-json.mjs';

export const CORE_SCORE_SCHEMA = 'humanymous.external-input-core-score/v1';
export const SCORE_TRACE_SCHEMA = 'humanymous.external-input-score-trace/v1';
export const SCORE_RECEIPT_SCHEMA = 'humanymous.external-input-score-receipt/v1';

const CORE_SCORE_FIELDS = Object.freeze([
  'schemaVersion',
  'runLabel',
  'runId',
  'profileId',
  'sessionRef',
  'riskScore',
  'verdict',
  'hardRuleFired',
  'policyVersion',
  'detectorPath',
  'detectorWasmSha256',
  'engineVersion',
  'wasmSignalCount',
  'reportSha256',
  'scoreTraceSha256',
  'scoreRecomputed',
  'serverAuthoritative',
]);
const SHA256 = /^[a-f0-9]{64}$/;
const SESSION_REF = /^[a-f0-9]{16}$/;
const VERDICTS = new Set(['ALLOW', 'CHALLENGE', 'DENY']);
const MAX_CORE_RECEIPT_BYTES = 16 * 1024;
const MAX_RESULT_BYTES = 2 * 1024 * 1024;
const MAX_TEXT_BYTES = 256;
const MAX_WASM_SIGNALS = 10_000;

function required(value, label) {
  if (!value) throw new TypeError(`${label} is required`);
  return value;
}

async function boundedRead(path, label, maximumBytes) {
  const value = await readFile(path);
  if (value.length > maximumBytes) {
    throw new TypeError(`${label} exceeds its byte bound`);
  }
  return value.toString('utf8');
}

function exactObject(value, fields, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  const actual = Object.keys(value).sort();
  const expected = [...fields].sort();
  if (actual.length !== expected.length ||
      actual.some((field, index) => field !== expected[index])) {
    throw new TypeError(`${label} fields are invalid`);
  }
}

function boundedText(value, { allowEmpty = false } = {}) {
  return typeof value === 'string' &&
    (allowEmpty || value.length > 0) &&
    Buffer.byteLength(value) <= MAX_TEXT_BYTES;
}

function traceDigest(receipt) {
  const material = [
    SCORE_TRACE_SCHEMA,
    receipt.runLabel,
    receipt.sessionRef,
    Number(receipt.riskScore).toFixed(1),
    receipt.verdict,
    receipt.hardRuleFired,
    receipt.policyVersion,
    receipt.detectorWasmSha256,
    receipt.engineVersion,
    String(receipt.wasmSignalCount),
    receipt.reportSha256,
  ].join('\0');
  return createHash('sha256').update(material).digest('hex');
}

function validateCoreReceipt(receipt, runId, profileId) {
  exactObject(receipt, CORE_SCORE_FIELDS, 'Core score receipt');
  const expectedLabel = `external-input/${runId}/${profileId}`;
  if (receipt.schemaVersion !== CORE_SCORE_SCHEMA ||
      receipt.runLabel !== expectedLabel ||
      receipt.runId !== runId ||
      receipt.profileId !== profileId) {
    throw new TypeError('Core score receipt is not bound to this run and profile');
  }

  const riskScore = receipt.riskScore;
  if (typeof riskScore !== 'number' ||
      !Number.isFinite(riskScore) || riskScore < 0 || riskScore > 100 ||
      !VERDICTS.has(receipt.verdict) ||
      !SESSION_REF.test(receipt.sessionRef || '') ||
      !boundedText(receipt.hardRuleFired, { allowEmpty: true }) ||
      !boundedText(receipt.policyVersion) ||
      receipt.detectorPath !== '/static/detector.wasm' ||
      !SHA256.test(receipt.detectorWasmSha256 || '') ||
      !boundedText(receipt.engineVersion) ||
      !Number.isSafeInteger(receipt.wasmSignalCount) ||
      receipt.wasmSignalCount < 1 ||
      receipt.wasmSignalCount > MAX_WASM_SIGNALS ||
      !SHA256.test(receipt.reportSha256 || '') ||
      !SHA256.test(receipt.scoreTraceSha256 || '') ||
      receipt.scoreRecomputed !== false ||
      receipt.serverAuthoritative !== true) {
    throw new TypeError('Core score receipt fields are invalid');
  }
  if (traceDigest(receipt) !== receipt.scoreTraceSha256) {
    throw new TypeError('Core score receipt hash differs');
  }
  return riskScore;
}

function validateFramebufferResult(result, runId, profileId, verdict) {
  if (!result || typeof result !== 'object' || Array.isArray(result) ||
      result.runId !== runId ||
      result.profileId !== profileId ||
      !result.measurement ||
      typeof result.measurement !== 'object' ||
      Array.isArray(result.measurement) ||
      result.measurement.verdict !== verdict ||
      !SHA256.test(result.measurement.finalFrameSha256 || '')) {
    throw new TypeError('Core score receipt and framebuffer result are not bound');
  }
  return result.measurement.finalFrameSha256;
}

export async function evaluateScoreTrace({
  runId,
  profileId,
  coreReceiptPath,
  resultPath,
  destination,
}) {
  required(runId, 'runId');
  required(profileId, 'profileId');
  const coreReceipt = parseStrictJson(
    await boundedRead(
      coreReceiptPath,
      'Core score receipt',
      MAX_CORE_RECEIPT_BYTES,
    ),
    'Core score receipt',
  );
  const riskScore = validateCoreReceipt(coreReceipt, runId, profileId);
  const result = parseStrictJson(
    await boundedRead(resultPath, 'controller result', MAX_RESULT_BYTES),
    'controller result',
  );
  const framebufferSha256 = validateFramebufferResult(
    result,
    runId,
    profileId,
    coreReceipt.verdict,
  );

  const receipt = Object.freeze({
    schemaVersion: SCORE_RECEIPT_SCHEMA,
    runId,
    profileId,
    scorer: 'core',
    detectorPath: coreReceipt.detectorPath,
    detectorWasmSha256: coreReceipt.detectorWasmSha256,
    scoreRecomputed: false,
    riskScore,
    verdict: coreReceipt.verdict,
    hardRuleFired: coreReceipt.hardRuleFired,
    policyVersion: coreReceipt.policyVersion,
    sessionRef: coreReceipt.sessionRef,
    scoreTraceSha256: coreReceipt.scoreTraceSha256,
    framebufferSha256,
  });
  await writeFile(destination, `${JSON.stringify(receipt, null, 2)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o600,
  });
  return receipt;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  evaluateScoreTrace({
    runId: required(process.env.HM_EXTERNAL_RUN_ID, 'HM_EXTERNAL_RUN_ID'),
    profileId: required(process.env.HM_EXTERNAL_MODE, 'HM_EXTERNAL_MODE'),
    coreReceiptPath: required(
      process.env.HM_EXTERNAL_CORE_SCORE_RECEIPT,
      'HM_EXTERNAL_CORE_SCORE_RECEIPT',
    ),
    resultPath: required(process.env.HM_EXTERNAL_RESULT_PATH, 'HM_EXTERNAL_RESULT_PATH'),
    destination: required(process.env.HM_EXTERNAL_SCORE_RECEIPT, 'HM_EXTERNAL_SCORE_RECEIPT'),
  }).catch((error) => {
    process.stderr.write(`score receipt evaluation failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
