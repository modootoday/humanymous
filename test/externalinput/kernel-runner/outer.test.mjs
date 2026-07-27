import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { RUNTIME_IMAGE_FIELDS } from '../vusb/catalog.mjs';
import { sha256 } from '../vusb/common.mjs';
import {
  cellRuntimeImageKeys,
  createOuterReceipt,
  exactRuntimeImageArguments,
  validateEmbeddedRunnerIdentity,
  validateLocalImageIdentity,
  validateOuterOutputDirectory,
  validateOuterReceipt,
} from './outer.mjs';
import { createKernelRunnerSeed } from './seed.mjs';

const digest = (character) => `sha256:${character.repeat(64)}`;
const runtimeImages = Object.fromEntries(
  RUNTIME_IMAGE_FIELDS.map((name, index) => [
    name,
    digest((index + 1).toString(16)),
  ]),
);

const fileDigest = (raw) => `sha256:${sha256(raw)}`;

function seed() {
  return createKernelRunnerSeed({
    runId: 'kernel-runner-test-0001',
    runNonce: '0'.repeat(64),
    modelId: 'reference-relative-v1',
    runtimeImages,
    imageArchiveSha256: digest('f'),
    imageArchiveBytes: 4096,
    sourceBundleSha256: digest('e'),
    sourceBundleBytes: 8192,
    sourceBundleEntries: 10,
    runner: {
      imageDigest: digest('d'),
      qemuVersion: '10.0.0',
      qemuBinarySha256: digest('c'),
      guestBaseSha256: digest('b'),
      guestBaseFormat: 'qcow2',
      guestBaseVirtualMiB: 2048,
      guestBaseAllocatedBytes: 176 * 1024 * 1024,
      kernelSha256: digest('a'),
      initramfsSha256: digest('9'),
    },
    projectName: 'hmn-kernel-test-0001',
    sequence: 3,
    strategySeed: 'strongest-human-mimic-0001',
    profileManifestSha256: digest('8'),
    protocolContractSha256: digest('7'),
    budgets: {
      cpus: 2,
      memoryMiB: 2048,
      deadlineSeconds: 1800,
      outputBytes: 64 * 1024 * 1024,
    },
  });
}

async function measurementFixture(root) {
  const value = seed();
  const seedSha256 = digest('8');
  const frame = '7'.repeat(64);
  const wasm = '6'.repeat(64);
  const result = {
    runId: 'vusb-child-test-0001',
    profileId: 'external_input_vusb',
    control: { seed: value.strategy.seed },
    measurement: {
      verdict: 'CHALLENGE',
      finalFrameSha256: frame,
    },
  };
  const terminal = {
    authority: 'seed-bound-development',
    canonical: false,
    baselineEligible: false,
    releaseAttested: false,
    status: 'PASS',
    measurementVerdict: 'CHALLENGE',
    evidence: {
      virtualUsbAttestationSha256: digest('5'),
      teardownObservationSha256: digest('4'),
    },
  };
  const score = {
    schemaVersion: 'humanymous.external-input-score-receipt/v1',
    runId: result.runId,
    profileId: 'external_input_vusb',
    scorer: 'core',
    detectorPath: '/static/detector.wasm',
    detectorWasmSha256: wasm,
    scoreRecomputed: false,
    riskScore: 75.2,
    verdict: 'CHALLENGE',
    hardRuleFired: '',
    policyVersion: 'v1',
    sessionRef: '1'.repeat(16),
    scoreTraceSha256: '2'.repeat(64),
    framebufferSha256: frame,
  };
  const resultRaw = Buffer.from(`${JSON.stringify(result)}\n`);
  const terminalRaw = Buffer.from(`${JSON.stringify(terminal)}\n`);
  const scoreRaw = Buffer.from(`${JSON.stringify(score)}\n`);
  const guest = {
    schemaVersion: 'humanymous.kernel-guest-terminal/v1',
    kind: 'kernel-guest-measurement',
    status: 'PASS',
    runId: value.runId,
    runNonce: value.runNonce,
    modelId: value.modelId,
    authority: 'seed-bound-development',
    canonical: false,
    baselineEligible: false,
    releaseAttested: false,
    evidenceClass: 'kernel-emulated-usb',
    physicalUsb: false,
    seedSha256,
    imageArchiveSha256: value.imageArchive.sha256,
    sourceBundleSha256: value.sourceBundle.sha256,
    cell: {
      browser: 'chromium',
      sequence: 3,
      profileId: 'external_input_vusb',
    },
    strategy: structuredClone(value.strategy),
    guestBootId: '12345678-1234-1234-1234-123456789abc',
    kernelRelease: '6.12.96+deb13-amd64',
    composeConfigSha256: digest('3'),
    resultSha256: fileDigest(resultRaw),
    terminalSha256: fileDigest(terminalRaw),
    scoreReceiptSha256: fileDigest(scoreRaw),
    browserWasmSha256: `sha256:${wasm}`,
    framebufferVerdict: 'CHALLENGE',
    usbTopologyReceiptSha256: terminal.evidence.virtualUsbAttestationSha256,
    teardownResidueSha256: terminal.evidence.teardownObservationSha256,
  };
  const guestRaw = Buffer.from(`${JSON.stringify(guest)}\n`);
  const runner = {
    schemaVersion: 'humanymous.kernel-runner-receipt/v1',
    kind: 'kernel-runner',
    status: 'PASS',
    accelerator: 'kvm',
    qemuVersion: '10.0.0',
    runnerIdentitySha256: digest('2'),
    guestTerminalSha256: fileDigest(guestRaw),
    guestTerminal: 'guest-terminal.json',
    consoleLog: 'console.log',
  };
  await Promise.all([
    writeFile(join(root, 'guest-terminal.json'), guestRaw),
    writeFile(join(root, 'runner.json'), `${JSON.stringify(runner)}\n`),
    writeFile(join(root, 'measurement-result.json'), resultRaw),
    writeFile(join(root, 'measurement-terminal.json'), terminalRaw),
    writeFile(join(root, 'measurement-score.json'), scoreRaw),
    writeFile(join(root, 'console.log'), 'qemu booted\n'),
    writeFile(join(root, 'guest-console.log'), 'guest booted\n'),
    writeFile(join(root, 'dockerd.log'), 'dockerd ready\n'),
    writeFile(join(root, 'docker-load.log'), 'images loaded\n'),
  ]);
  return { seed: value, seedSha256 };
}

test('orders and de-duplicates exact image archive arguments', () => {
  const images = { ...runtimeImages, pki: runtimeImages.labCore };
  const arguments_ = exactRuntimeImageArguments(images);
  assert.equal(arguments_[0], runtimeImages.labCore);
  assert.equal(arguments_.length, RUNTIME_IMAGE_FIELDS.length - 1);
  assert.throws(
    () => exactRuntimeImageArguments({ ...runtimeImages, labCore: 'latest' }),
    /labCore is invalid/,
  );
  const firefoxDomKeys = cellRuntimeImageKeys('firefox', 4);
  const selected = exactRuntimeImageArguments(runtimeImages, firefoxDomKeys);
  assert.equal(selected.length, firefoxDomKeys.length);
  assert.ok(selected.includes(runtimeImages.browserFirefoxDom));
  assert.ok(!selected.includes(runtimeImages.browserChromium));
});

test('requires an exact local linux/amd64 OCI identity', () => {
  assert.equal(validateLocalImageIdentity(digest('a'), {
    Id: digest('0'),
    Os: 'linux',
    Architecture: 'amd64',
    Descriptor: { digest: digest('a') },
  }), true);
  assert.throws(
    () => validateLocalImageIdentity(digest('a'), {
      Id: digest('a'),
      Os: 'linux',
      Architecture: 'arm64',
    }),
    /linux\/amd64/,
  );
});

test('requires all immutable embedded guest and QEMU identities', () => {
  assert.equal(validateEmbeddedRunnerIdentity({
    schemaVersion: 'humanymous.kernel-runner-identity/v1',
    qemuVersion: '10.0.0',
    qemuBinarySha256: digest('1'),
    guestBaseSha256: digest('2'),
    guestBaseFormat: 'qcow2',
    guestBaseVirtualMiB: 2048,
    guestBaseAllocatedBytes: 176 * 1024 * 1024,
    kernelSha256: digest('3'),
    initramfsSha256: digest('4'),
  }).qemuVersion, '10.0.0');
  assert.throws(
    () => validateEmbeddedRunnerIdentity({
      schemaVersion: 'humanymous.kernel-runner-identity/v1',
      qemuVersion: 'latest',
      qemuBinarySha256: digest('1'),
      guestBaseSha256: digest('2'),
      guestBaseFormat: 'qcow2',
      guestBaseVirtualMiB: 2048,
      guestBaseAllocatedBytes: 176 * 1024 * 1024,
      kernelSha256: digest('3'),
      initramfsSha256: digest('4'),
    }),
    /identity is invalid/,
  );
});

test('validates the exact bounded runner output and publishes outer hashes', async (t) => {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-kernel-outer-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const input = await measurementFixture(root);
  const validated = await validateOuterOutputDirectory(root, input);
  const receipt = createOuterReceipt({
    runId: 'kernel-runner-test-0001',
    modelId: 'reference-relative-v1',
    runnerImageDigest: digest('c'),
    seedSha256: digest('d'),
    imageArchiveSha256: digest('e'),
    runnerReceiptSha256: validated.runnerReceiptSha256,
    guestTerminalSha256: validated.guestTerminalSha256,
    measurementTerminalSha256: validated.guest.terminalSha256,
    browserWasmSha256: validated.guest.browserWasmSha256,
    cell: validated.guest.cell,
    strategy: validated.guest.strategy,
    coreMeasurement: {
      scorer: validated.score.scorer,
      scoreRecomputed: validated.score.scoreRecomputed,
      riskScore: validated.score.riskScore,
      verdict: validated.score.verdict,
    },
  });
  assert.equal(validateOuterReceipt(receipt).physicalUsb, false);
  assert.equal(receipt.authority, 'seed-bound-development');
  assert.equal(receipt.canonical, false);
  assert.equal(receipt.baselineEligible, false);
  assert.equal(receipt.releaseAttested, false);
  assert.deepEqual(receipt.coreMeasurement, {
    scorer: 'core',
    scoreRecomputed: false,
    riskScore: 75.2,
    verdict: 'CHALLENGE',
  });
});

test('fails closed for missing, duplicate-key, or unexpected runner evidence', async (t) => {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-kernel-outer-bad-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const input = await measurementFixture(root);
  await writeFile(
    join(root, 'guest-terminal.json'),
    '{"status":"PASS","status":"PASS"}\n',
  );
  await assert.rejects(
    validateOuterOutputDirectory(root, input),
    /duplicate key/,
  );
  await mkdir(join(root, 'unexpected'));
  await assert.rejects(
    validateOuterOutputDirectory(root, input),
    /missing or unexpected/,
  );
});
