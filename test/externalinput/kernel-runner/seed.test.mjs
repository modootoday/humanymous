import assert from 'node:assert/strict';
import test from 'node:test';

import { RUNTIME_IMAGE_FIELDS } from '../vusb/catalog.mjs';
import { canonicalJson, sha256 } from '../vusb/common.mjs';
import {
  createKernelRunnerSeed,
  validateKernelRunnerSeed,
} from './seed.mjs';

const runtimeImages = Object.fromEntries(
  RUNTIME_IMAGE_FIELDS.map((name, index) => [
    name,
    `sha256:${(index + 1).toString(16).repeat(64)}`,
  ]),
);

function seed() {
  return createKernelRunnerSeed({
    runId: 'kernel-runner-test-0001',
    runNonce: '0'.repeat(64),
    modelId: 'reference-relative-v1',
    runtimeImages,
    imageArchiveSha256: `sha256:${'f'.repeat(64)}`,
    imageArchiveBytes: 4096,
    sourceBundleSha256: `sha256:${'e'.repeat(64)}`,
    sourceBundleBytes: 8192,
    sourceBundleEntries: 10,
    runner: {
      imageDigest: `sha256:${'d'.repeat(64)}`,
      qemuVersion: '10.0.0',
      qemuBinarySha256: `sha256:${'c'.repeat(64)}`,
      guestBaseSha256: `sha256:${'b'.repeat(64)}`,
      guestBaseFormat: 'qcow2',
      guestBaseVirtualMiB: 2048,
      guestBaseAllocatedBytes: 176 * 1024 * 1024,
      kernelSha256: `sha256:${'a'.repeat(64)}`,
      initramfsSha256: `sha256:${'9'.repeat(64)}`,
    },
    projectName: 'hmn-kernel-test-0001',
    sequence: 3,
    strategySeed: 'strongest-human-mimic-0001',
    profileManifestSha256: `sha256:${'8'.repeat(64)}`,
    protocolContractSha256: `sha256:${'7'.repeat(64)}`,
    budgets: {
      cpus: 2,
      memoryMiB: 2048,
      deadlineSeconds: 1800,
      outputBytes: 64 * 1024 * 1024,
    },
  });
}

test('creates an exact digest-bound guest ladder seed', () => {
  const value = seed();
  assert.equal(validateKernelRunnerSeed(value).imageArchive.path, 'images.oci.tar');
  assert.deepEqual(value.imageArchive.imageKeys, [
    'labCore',
    'pki',
    'display',
    'browserChromium',
    'controller',
    'lifecycle',
    'gateway',
    'profile',
  ]);
  assert.deepEqual(
    Object.keys(value.runtimeImages),
    value.imageArchive.imageKeys,
  );
  assert.deepEqual(value.guestCommand, [
    'bash',
    'scripts/external-input-kernel-guest.sh',
    '--seed',
    '/seed/seed.json',
  ]);
  assert.deepEqual(value.admission, {
    authority: 'seed-bound-development',
    canonical: false,
    baselineEligible: false,
    releaseAttested: false,
    profileContractVersion: 'humanymous.virtual-usb-profile/v1',
    profileManifestSha256: `sha256:${'8'.repeat(64)}`,
    descriptorSetId: 'reference-relative-v1',
    protocolContractSha256: `sha256:${'7'.repeat(64)}`,
  });
});

test('rejects a mutable image identity and an unbounded archive', () => {
  assert.throws(
    () => validateKernelRunnerSeed({
      ...seed(),
      runtimeImages: { ...seed().runtimeImages, labCore: 'latest' },
    }),
    /runtime image labCore/,
  );
  assert.throws(
    () => validateKernelRunnerSeed({
      ...seed(),
      imageArchive: { ...seed().imageArchive, bytes: 673 * 1024 * 1024 },
      seedContentSha256: '',
    }),
    /seed identity|invalid or unbounded/,
  );
  const extraBrowser = structuredClone(seed());
  extraBrowser.imageArchive.imageKeys.push('browserFirefox');
  extraBrowser.runtimeImages.browserFirefox = runtimeImages.browserFirefox;
  extraBrowser.seedContentSha256 = '';
  extraBrowser.seedContentSha256 =
    `sha256:${sha256(canonicalJson(extraBrowser))}`;
  assert.throws(
    () => validateKernelRunnerSeed(extraBrowser),
    /does not match its cell/,
  );
});

test('rejects a substituted guest command', () => {
  assert.throws(
    () => validateKernelRunnerSeed({
      ...seed(),
      guestCommand: ['bash', '-c', 'true', 'reference-relative-v1'],
      seedContentSha256: '',
    }),
    /seed identity|guest command is not canonical/,
  );
});

test('preserves the strongest no-oracle strategy across 3V and 4V', () => {
  const mode3 = seed();
  const mode4 = createKernelRunnerSeed({
    ...{
      runId: 'kernel-runner-test-0002',
      runNonce: '1'.repeat(64),
      modelId: 'reference-relative-v1',
      runtimeImages,
      imageArchiveSha256: `sha256:${'f'.repeat(64)}`,
      imageArchiveBytes: 4096,
      sourceBundleSha256: `sha256:${'e'.repeat(64)}`,
      sourceBundleBytes: 8192,
      sourceBundleEntries: 10,
      runner: mode3.runner,
      projectName: 'hmn-kernel-test-0002',
      sequence: 4,
      strategySeed: 'strongest-human-mimic-0002',
      profileManifestSha256: mode3.admission.profileManifestSha256,
      protocolContractSha256: mode3.admission.protocolContractSha256,
      budgets: mode3.budgets,
    },
  });
  assert.deepEqual(mode3.strategy, {
    variant: 'mixed-input',
    seed: 'strongest-human-mimic-0001',
    oracleFeedback: false,
  });
  assert.equal(mode4.innerCompose.profileId, 'external_input_dom_vusb');
  assert.equal(mode4.innerCompose.domRequired, true);
});
