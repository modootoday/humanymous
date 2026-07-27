import { lstat, readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';

import {
  canonicalJson,
  exactObject,
  MODEL_ID,
  RUN_ID,
  SHA256,
  sha256,
} from '../vusb/common.mjs';
import { RUNTIME_IMAGE_FIELDS } from '../vusb/catalog.mjs';
import { parseStrictJson } from '../vusb/strict-json.mjs';
export {
  cellRuntimeImageKeys,
  exactRuntimeImageArguments,
} from './runtime-images.mjs';

export const KERNEL_RUNNER_OUTER_VERSION =
  'humanymous.kernel-runner-outer-receipt/v1';
export const KERNEL_RUNNER_IDENTITY_VERSION =
  'humanymous.kernel-runner-identity/v1';

const OUTER_FIELDS = Object.freeze([
  'schemaVersion',
  'kind',
  'status',
  'runId',
  'modelId',
  'authority',
  'canonical',
  'baselineEligible',
  'releaseAttested',
  'evidenceClass',
  'physicalUsb',
  'runnerImageDigest',
  'seedSha256',
  'imageArchiveSha256',
  'runnerReceiptSha256',
  'guestTerminalSha256',
  'measurementTerminalSha256',
  'browserWasmSha256',
  'cell',
  'strategy',
  'coreMeasurement',
]);

const MAXIMUM_RECEIPT_BYTES = 2 * 1024 * 1024;
const MAXIMUM_LOG_BYTES = 64 * 1024 * 1024;
const EXPECTED_MEASUREMENT_FILES = Object.freeze([
  'console.log',
  'docker-load.log',
  'dockerd.log',
  'guest-console.log',
  'guest-terminal.json',
  'measurement-result.json',
  'measurement-score.json',
  'measurement-terminal.json',
  'runner.json',
]);
const IDENTITY_FIELDS = Object.freeze([
  'schemaVersion',
  'qemuVersion',
  'qemuBinarySha256',
  'guestBaseSha256',
  'guestBaseFormat',
  'guestBaseVirtualMiB',
  'guestBaseAllocatedBytes',
  'kernelSha256',
  'initramfsSha256',
]);
const RUNNER_RECEIPT_FIELDS = Object.freeze([
  'schemaVersion',
  'kind',
  'status',
  'accelerator',
  'qemuVersion',
  'runnerIdentitySha256',
  'guestTerminalSha256',
  'guestTerminal',
  'consoleLog',
]);
const GUEST_TERMINAL_FIELDS = Object.freeze([
  'schemaVersion',
  'kind',
  'status',
  'runId',
  'runNonce',
  'modelId',
  'authority',
  'canonical',
  'baselineEligible',
  'releaseAttested',
  'evidenceClass',
  'physicalUsb',
  'seedSha256',
  'imageArchiveSha256',
  'sourceBundleSha256',
  'cell',
  'strategy',
  'guestBootId',
  'kernelRelease',
  'composeConfigSha256',
  'resultSha256',
  'terminalSha256',
  'scoreReceiptSha256',
  'browserWasmSha256',
  'framebufferVerdict',
  'usbTopologyReceiptSha256',
  'teardownResidueSha256',
]);
const SCORE_FIELDS = Object.freeze([
  'schemaVersion',
  'runId',
  'profileId',
  'scorer',
  'detectorPath',
  'detectorWasmSha256',
  'scoreRecomputed',
  'riskScore',
  'verdict',
  'hardRuleFired',
  'policyVersion',
  'sessionRef',
  'scoreTraceSha256',
  'framebufferSha256',
]);
const VERDICTS = new Set(['ALLOW', 'CHALLENGE', 'DENY']);
const HEX_SHA256 = /^[a-f0-9]{64}$/;

export function validateLocalImageIdentity(expectedDigest, inspected) {
  if (!SHA256.test(expectedDigest || '') ||
      !inspected ||
      typeof inspected !== 'object' ||
      Array.isArray(inspected)) {
    throw new TypeError('local image inspection is invalid');
  }
  const descriptorDigest = inspected.Descriptor?.digest;
  if (descriptorDigest !== expectedDigest && inspected.Id !== expectedDigest) {
    throw new TypeError('local image identity does not match its immutable digest');
  }
  if (inspected.Os !== 'linux' || inspected.Architecture !== 'amd64') {
    throw new TypeError('local image platform must be linux/amd64');
  }
  return true;
}

export function validateEmbeddedRunnerIdentity(identity) {
  exactObject(identity, IDENTITY_FIELDS, 'embedded kernel runner identity');
  if (identity.schemaVersion !== KERNEL_RUNNER_IDENTITY_VERSION ||
      !/^[0-9]+\.[0-9]+\.[0-9]+$/.test(identity.qemuVersion || '') ||
      identity.guestBaseFormat !== 'qcow2' ||
      !Number.isSafeInteger(identity.guestBaseVirtualMiB) ||
      identity.guestBaseVirtualMiB < 512 ||
      identity.guestBaseVirtualMiB > 4096 ||
      !Number.isSafeInteger(identity.guestBaseAllocatedBytes) ||
      identity.guestBaseAllocatedBytes < 1 ||
      identity.guestBaseAllocatedBytes > 192 * 1024 * 1024) {
    throw new TypeError('embedded kernel runner identity is invalid');
  }
  for (const field of [
    'qemuBinarySha256',
    'guestBaseSha256',
    'kernelSha256',
    'initramfsSha256',
  ]) {
    if (!SHA256.test(identity[field] || '')) {
      throw new TypeError(`embedded kernel runner ${field} is invalid`);
    }
  }
  return Object.freeze(structuredClone(identity));
}

export function validateOuterReceipt(receipt) {
  exactObject(receipt, OUTER_FIELDS, 'kernel outer receipt');
  if (receipt.schemaVersion !== KERNEL_RUNNER_OUTER_VERSION ||
      receipt.kind !== 'kernel-runner-outer' ||
      receipt.status !== 'PASS' ||
      !RUN_ID.test(receipt.runId || '') ||
      !MODEL_ID.test(receipt.modelId || '') ||
      receipt.authority !== 'seed-bound-development' ||
      receipt.canonical !== false ||
      receipt.baselineEligible !== false ||
      receipt.releaseAttested !== false ||
      receipt.evidenceClass !== 'kernel-emulated-usb' ||
      receipt.physicalUsb !== false) {
    throw new TypeError('kernel outer receipt authority is invalid');
  }
  for (const field of [
    'runnerImageDigest',
    'seedSha256',
    'imageArchiveSha256',
    'runnerReceiptSha256',
    'guestTerminalSha256',
    'measurementTerminalSha256',
    'browserWasmSha256',
  ]) {
    if (!SHA256.test(receipt[field] || '')) {
      throw new TypeError(`kernel outer receipt ${field} is invalid`);
    }
  }
  exactObject(receipt.cell, ['browser', 'sequence', 'profileId'], 'kernel outer cell');
  if (!['chromium', 'firefox'].includes(receipt.cell.browser) ||
      ![3, 4].includes(receipt.cell.sequence) ||
      receipt.cell.profileId !== (
        receipt.cell.sequence === 3
          ? 'external_input_vusb'
          : 'external_input_dom_vusb'
      )) {
    throw new TypeError('kernel outer cell is invalid');
  }
  exactObject(receipt.strategy, ['variant', 'seed', 'oracleFeedback'], 'kernel outer strategy');
  if (receipt.strategy.variant !== 'mixed-input' ||
      typeof receipt.strategy.seed !== 'string' ||
      receipt.strategy.oracleFeedback !== false) {
    throw new TypeError('kernel outer strategy is invalid or oracle-enabled');
  }
  exactObject(
    receipt.coreMeasurement,
    ['scorer', 'scoreRecomputed', 'riskScore', 'verdict'],
    'kernel outer Core measurement',
  );
  const riskScore = Number(receipt.coreMeasurement.riskScore);
  if (receipt.coreMeasurement.scorer !== 'core' ||
      receipt.coreMeasurement.scoreRecomputed !== false ||
      !Number.isFinite(riskScore) || riskScore < 0 || riskScore > 100 ||
      !VERDICTS.has(receipt.coreMeasurement.verdict)) {
    throw new TypeError('kernel outer Core measurement is invalid');
  }
  return Object.freeze(structuredClone(receipt));
}

export function createOuterReceipt({
  runId,
  modelId,
  runnerImageDigest,
  seedSha256,
  imageArchiveSha256,
  runnerReceiptSha256,
  guestTerminalSha256,
  measurementTerminalSha256,
  browserWasmSha256,
  cell,
  strategy,
  coreMeasurement,
}) {
  return validateOuterReceipt({
    schemaVersion: KERNEL_RUNNER_OUTER_VERSION,
    kind: 'kernel-runner-outer',
    status: 'PASS',
    runId,
    modelId,
    authority: 'seed-bound-development',
    canonical: false,
    baselineEligible: false,
    releaseAttested: false,
    evidenceClass: 'kernel-emulated-usb',
    physicalUsb: false,
    runnerImageDigest,
    seedSha256,
    imageArchiveSha256,
    runnerReceiptSha256,
    guestTerminalSha256,
    measurementTerminalSha256,
    browserWasmSha256,
    cell: structuredClone(cell),
    strategy: structuredClone(strategy),
    coreMeasurement: structuredClone(coreMeasurement),
  });
}

async function boundedRegularFile(path, maximumBytes = MAXIMUM_RECEIPT_BYTES) {
  const stat = await lstat(path);
  if (!stat.isFile() || stat.isSymbolicLink() ||
      stat.size < 1 || stat.size > maximumBytes) {
    throw new TypeError(`${path} is not a bounded regular evidence file`);
  }
  return readFile(path);
}

function validateMeasurementRunnerReceipt(receipt) {
  exactObject(receipt, RUNNER_RECEIPT_FIELDS, 'kernel measurement runner receipt');
  if (receipt.schemaVersion !== 'humanymous.kernel-runner-receipt/v1' ||
      receipt.kind !== 'kernel-runner' ||
      receipt.status !== 'PASS' ||
      !['kvm', 'tcg'].includes(receipt.accelerator) ||
      !/^[0-9]+\.[0-9]+\.[0-9]+$/.test(receipt.qemuVersion || '') ||
      !SHA256.test(receipt.runnerIdentitySha256 || '') ||
      !SHA256.test(receipt.guestTerminalSha256 || '') ||
      receipt.guestTerminal !== 'guest-terminal.json' ||
      receipt.consoleLog !== 'console.log') {
    throw new TypeError('kernel measurement runner receipt is invalid');
  }
}

function validateGuestTerminal(receipt, seed, seedSha256) {
  exactObject(receipt, GUEST_TERMINAL_FIELDS, 'kernel guest terminal');
  if (receipt.schemaVersion !== 'humanymous.kernel-guest-terminal/v1' ||
      receipt.kind !== 'kernel-guest-measurement' ||
      receipt.status !== 'PASS' ||
      receipt.runId !== seed.runId ||
      receipt.runNonce !== seed.runNonce ||
      receipt.modelId !== seed.modelId ||
      receipt.authority !== 'seed-bound-development' ||
      receipt.canonical !== false ||
      receipt.baselineEligible !== false ||
      receipt.releaseAttested !== false ||
      receipt.evidenceClass !== 'kernel-emulated-usb' ||
      receipt.physicalUsb !== false ||
      receipt.seedSha256 !== seedSha256 ||
      receipt.imageArchiveSha256 !== seed.imageArchive.sha256 ||
      receipt.sourceBundleSha256 !== seed.sourceBundle.sha256) {
    throw new TypeError('kernel guest terminal authority or seed binding is invalid');
  }
  exactObject(receipt.cell, ['browser', 'sequence', 'profileId'], 'kernel guest cell');
  if (canonicalJson(receipt.cell) !== canonicalJson({
    browser: seed.innerCompose.browser,
    sequence: seed.innerCompose.sequence,
    profileId: seed.innerCompose.profileId,
  })) {
    throw new TypeError('kernel guest terminal cell differs from the seed');
  }
  exactObject(receipt.strategy, ['variant', 'seed', 'oracleFeedback'], 'kernel guest strategy');
  if (canonicalJson(receipt.strategy) !== canonicalJson(seed.strategy)) {
    throw new TypeError('kernel guest strategy differs from the no-oracle seed');
  }
  if (!/^[a-f0-9-]{36}$/.test(receipt.guestBootId || '') ||
      typeof receipt.kernelRelease !== 'string' ||
      receipt.kernelRelease.length < 3 || receipt.kernelRelease.length > 128 ||
      !VERDICTS.has(receipt.framebufferVerdict)) {
    throw new TypeError('kernel guest runtime identity or verdict is invalid');
  }
  for (const field of [
    'composeConfigSha256',
    'resultSha256',
    'terminalSha256',
    'scoreReceiptSha256',
    'browserWasmSha256',
    'usbTopologyReceiptSha256',
    'teardownResidueSha256',
  ]) {
    if (!SHA256.test(receipt[field] || '')) {
      throw new TypeError(`kernel guest terminal ${field} is invalid`);
    }
  }
}

function validateScoreReceipt(receipt) {
  exactObject(receipt, SCORE_FIELDS, 'Core score receipt');
  const riskScore = Number(receipt.riskScore);
  if (receipt.schemaVersion !== 'humanymous.external-input-score-receipt/v1' ||
      receipt.scorer !== 'core' ||
      receipt.detectorPath !== '/static/detector.wasm' ||
      !HEX_SHA256.test(receipt.detectorWasmSha256 || '') ||
      receipt.scoreRecomputed !== false ||
      !Number.isFinite(riskScore) || riskScore < 0 || riskScore > 100 ||
      !VERDICTS.has(receipt.verdict) ||
      !/^[a-f0-9]{16}$/.test(receipt.sessionRef || '') ||
      !HEX_SHA256.test(receipt.scoreTraceSha256 || '') ||
      !HEX_SHA256.test(receipt.framebufferSha256 || '')) {
    throw new TypeError('Core score receipt is invalid');
  }
}

export async function validateOuterOutputDirectory(directory, {
  seed,
  seedSha256,
} = {}) {
  if (!seed || !SHA256.test(seedSha256 || '')) {
    throw new TypeError('outer validation requires the exact seed and file hash');
  }
  const entries = (await readdir(directory)).sort();
  if (canonicalJson(entries) !== canonicalJson(EXPECTED_MEASUREMENT_FILES)) {
    throw new TypeError('kernel runner output contains missing or unexpected files');
  }
  const [
    guestRaw,
    runnerRaw,
    resultRaw,
    terminalRaw,
    scoreRaw,
  ] = await Promise.all([
    boundedRegularFile(join(directory, 'guest-terminal.json')),
    boundedRegularFile(join(directory, 'runner.json')),
    boundedRegularFile(join(directory, 'measurement-result.json')),
    boundedRegularFile(join(directory, 'measurement-terminal.json')),
    boundedRegularFile(join(directory, 'measurement-score.json')),
    boundedRegularFile(join(directory, 'console.log'), MAXIMUM_LOG_BYTES),
    boundedRegularFile(join(directory, 'guest-console.log'), MAXIMUM_LOG_BYTES),
    boundedRegularFile(join(directory, 'dockerd.log'), MAXIMUM_LOG_BYTES),
    boundedRegularFile(join(directory, 'docker-load.log'), MAXIMUM_LOG_BYTES),
  ]);
  const guest = parseStrictJson(guestRaw.toString('utf8'), 'kernel guest receipt');
  const runner = parseStrictJson(runnerRaw.toString('utf8'), 'kernel runner receipt');
  const result = parseStrictJson(resultRaw.toString('utf8'), 'kernel measurement result');
  const terminal = parseStrictJson(terminalRaw.toString('utf8'), 'kernel measurement terminal');
  const score = parseStrictJson(scoreRaw.toString('utf8'), 'kernel score receipt');
  validateMeasurementRunnerReceipt(runner);
  validateGuestTerminal(guest, seed, seedSha256);
  validateScoreReceipt(score);
  const digest = (raw) => `sha256:${sha256(raw)}`;
  if (runner.guestTerminalSha256 !== digest(guestRaw) ||
      guest.resultSha256 !== digest(resultRaw) ||
      guest.terminalSha256 !== digest(terminalRaw) ||
      guest.scoreReceiptSha256 !== digest(scoreRaw) ||
      guest.browserWasmSha256 !== `sha256:${score.detectorWasmSha256}` ||
      result.runId !== score.runId ||
      result.profileId !== seed.innerCompose.profileId ||
      score.profileId !== seed.innerCompose.profileId ||
      result.control?.seed !== String(seed.strategy.seed) ||
      result.measurement?.verdict !== score.verdict ||
      result.measurement.verdict !== guest.framebufferVerdict ||
      result.measurement.finalFrameSha256 !== score.framebufferSha256 ||
      terminal.authority !== 'seed-bound-development' ||
      terminal.canonical !== false ||
      terminal.baselineEligible !== false ||
      terminal.releaseAttested !== false ||
      terminal.status !== 'PASS' ||
      terminal.measurementVerdict !== score.verdict ||
      terminal.evidence?.virtualUsbAttestationSha256 !==
        guest.usbTopologyReceiptSha256 ||
      terminal.evidence?.teardownObservationSha256 !==
        guest.teardownResidueSha256) {
    throw new TypeError('kernel measurement evidence chain is not cross-bound');
  }
  return Object.freeze({
    guest: Object.freeze(guest),
    runner: Object.freeze(runner),
    result: Object.freeze(result),
    terminal: Object.freeze(terminal),
    score: Object.freeze(score),
    guestTerminalSha256: `sha256:${sha256(guestRaw)}`,
    runnerReceiptSha256: `sha256:${sha256(runnerRaw)}`,
  });
}
