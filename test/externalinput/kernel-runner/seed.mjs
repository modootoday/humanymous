import { RUNTIME_IMAGE_FIELDS } from '../vusb/catalog.mjs';
import {
  canonicalJson,
  exactObject,
  MODEL_ID,
  RUN_ID,
  SHA256,
  sha256,
} from '../vusb/common.mjs';
import { cellRuntimeImageKeys } from './runtime-images.mjs';

export const KERNEL_RUNNER_SEED_VERSION =
  'humanymous.kernel-runner-seed/v1';

const SEED_FIELDS = Object.freeze([
  'schemaVersion',
  'seedContentSha256',
  'runId',
  'runNonce',
  'modelId',
  'platform',
  'admission',
  'runtimeImages',
  'imageArchive',
  'sourceBundle',
  'runner',
  'innerCompose',
  'strategy',
  'budgets',
  'guestCommand',
]);

const ARCHIVE_FIELDS = Object.freeze(['path', 'sha256', 'bytes', 'imageKeys']);
const ADMISSION_FIELDS = Object.freeze([
  'authority',
  'canonical',
  'baselineEligible',
  'releaseAttested',
  'profileContractVersion',
  'profileManifestSha256',
  'descriptorSetId',
  'protocolContractSha256',
]);
const SOURCE_FIELDS = Object.freeze(['path', 'sha256', 'bytes', 'entries']);
const RUNNER_FIELDS = Object.freeze([
  'imageDigest',
  'qemuVersion',
  'qemuBinarySha256',
  'guestBaseSha256',
  'guestBaseFormat',
  'guestBaseVirtualMiB',
  'guestBaseAllocatedBytes',
  'kernelSha256',
  'initramfsSha256',
]);
const INNER_FIELDS = Object.freeze([
  'projectName',
  'browser',
  'sequence',
  'profileId',
  'domRequired',
  'pullPolicy',
]);
const BUDGET_FIELDS = Object.freeze([
  'cpus',
  'memoryMiB',
  'deadlineSeconds',
  'outputBytes',
]);
const STRATEGY_FIELDS = Object.freeze([
  'variant',
  'seed',
  'oracleFeedback',
]);
const MAXIMUM_ARCHIVE_BYTES = 672 * 1024 * 1024;
const MAXIMUM_SOURCE_BYTES = 8 * 1024 * 1024;
const PROJECT_ID = /^[a-z0-9][a-z0-9-]{0,62}$/;
const NONCE = /^[a-f0-9]{64}$/;
const QEMU_VERSION = /^[0-9]+\.[0-9]+\.[0-9]+$/;
const STRATEGY_SEED = /^[A-Za-z0-9._:-]{1,128}$/;

function validateRuntimeImages(runtimeImages, imageKeys) {
  exactObject(runtimeImages, imageKeys, 'kernel seed runtime images');
  for (const [name, digest] of Object.entries(runtimeImages)) {
    if (!SHA256.test(digest || '')) {
      throw new TypeError(`kernel seed runtime image ${name} is invalid`);
    }
  }
  return runtimeImages;
}

export function kernelRunnerSeedContentHash(value) {
  const copy = structuredClone(value);
  copy.seedContentSha256 = '';
  return `sha256:${sha256(canonicalJson(copy))}`;
}

export function validateKernelRunnerSeed(value) {
  exactObject(value, SEED_FIELDS, 'kernel runner seed');
  if (value.schemaVersion !== KERNEL_RUNNER_SEED_VERSION ||
      !RUN_ID.test(value.runId || '') ||
      !NONCE.test(value.runNonce || '') ||
      !MODEL_ID.test(value.modelId || '') ||
      value.platform !== 'linux/amd64') {
    throw new TypeError('kernel runner seed identity is invalid');
  }
  exactObject(value.admission, ADMISSION_FIELDS, 'kernel seed admission');
  if (value.admission.authority !== 'seed-bound-development' ||
      value.admission.canonical !== false ||
      value.admission.baselineEligible !== false ||
      value.admission.releaseAttested !== false ||
      value.admission.profileContractVersion !==
        'humanymous.virtual-usb-profile/v1' ||
      !SHA256.test(value.admission.profileManifestSha256 || '') ||
      value.admission.descriptorSetId !== 'reference-relative-v1' ||
      !SHA256.test(value.admission.protocolContractSha256 || '')) {
    throw new TypeError('kernel seed admission truth boundary is invalid');
  }
  exactObject(value.imageArchive, ARCHIVE_FIELDS, 'kernel runner image archive');
  if (value.imageArchive.path !== 'images.oci.tar' ||
      !SHA256.test(value.imageArchive.sha256 || '') ||
      !Number.isSafeInteger(value.imageArchive.bytes) ||
      value.imageArchive.bytes < 1 ||
      value.imageArchive.bytes > MAXIMUM_ARCHIVE_BYTES ||
      !Array.isArray(value.imageArchive.imageKeys)) {
    throw new TypeError('kernel runner image archive is invalid or unbounded');
  }
  validateRuntimeImages(value.runtimeImages, value.imageArchive.imageKeys);
  exactObject(value.sourceBundle, SOURCE_FIELDS, 'kernel runner source bundle');
  if (value.sourceBundle.path !== 'bundle.tar' ||
      !SHA256.test(value.sourceBundle.sha256 || '') ||
      !Number.isSafeInteger(value.sourceBundle.bytes) ||
      value.sourceBundle.bytes < 1 ||
      value.sourceBundle.bytes > MAXIMUM_SOURCE_BYTES ||
      !Number.isSafeInteger(value.sourceBundle.entries) ||
      value.sourceBundle.entries < 1 ||
      value.sourceBundle.entries > 4096) {
    throw new TypeError('kernel runner source bundle is invalid or unbounded');
  }
  exactObject(value.runner, RUNNER_FIELDS, 'kernel runner identity');
  if (!SHA256.test(value.runner.imageDigest || '') ||
      !QEMU_VERSION.test(value.runner.qemuVersion || '') ||
      value.runner.guestBaseFormat !== 'qcow2' ||
      !Number.isSafeInteger(value.runner.guestBaseVirtualMiB) ||
      value.runner.guestBaseVirtualMiB < 512 ||
      value.runner.guestBaseVirtualMiB > 4096 ||
      !Number.isSafeInteger(value.runner.guestBaseAllocatedBytes) ||
      value.runner.guestBaseAllocatedBytes < 1 ||
      value.runner.guestBaseAllocatedBytes > 192 * 1024 * 1024) {
    throw new TypeError('kernel runner image or QEMU version is invalid');
  }
  for (const field of [
    'qemuBinarySha256',
    'guestBaseSha256',
    'kernelSha256',
    'initramfsSha256',
  ]) {
    if (!SHA256.test(value.runner[field] || '')) {
      throw new TypeError(`kernel runner ${field} is invalid`);
    }
  }
  exactObject(value.innerCompose, INNER_FIELDS, 'kernel inner Compose cell');
  const expectedCell = value.innerCompose.sequence === 3
    ? { profileId: 'external_input_vusb', domRequired: false }
    : value.innerCompose.sequence === 4
      ? { profileId: 'external_input_dom_vusb', domRequired: true }
      : null;
  if (!PROJECT_ID.test(value.innerCompose.projectName || '') ||
      !['chromium', 'firefox'].includes(value.innerCompose.browser) ||
      !expectedCell ||
      value.innerCompose.profileId !== expectedCell.profileId ||
      value.innerCompose.domRequired !== expectedCell.domRequired ||
      value.innerCompose.pullPolicy !== 'never') {
    throw new TypeError('kernel inner Compose cell is not canonical');
  }
  if (canonicalJson(value.imageArchive.imageKeys) !== canonicalJson(
    cellRuntimeImageKeys(
      value.innerCompose.browser,
      value.innerCompose.sequence,
    ),
  )) {
    throw new TypeError('kernel image archive does not match its cell');
  }
  exactObject(value.strategy, STRATEGY_FIELDS, 'kernel Red strategy');
  if (value.strategy.variant !== 'mixed-input' ||
      !STRATEGY_SEED.test(value.strategy.seed || '') ||
      value.strategy.oracleFeedback !== false) {
    throw new TypeError('kernel Red strategy is invalid or oracle-enabled');
  }
  exactObject(value.budgets, BUDGET_FIELDS, 'kernel guest budgets');
  if (!Number.isInteger(value.budgets.cpus) ||
      value.budgets.cpus < 1 || value.budgets.cpus > 8 ||
      !Number.isInteger(value.budgets.memoryMiB) ||
      value.budgets.memoryMiB < 512 || value.budgets.memoryMiB > 8192 ||
      !Number.isInteger(value.budgets.deadlineSeconds) ||
      value.budgets.deadlineSeconds < 60 || value.budgets.deadlineSeconds > 3600 ||
      !Number.isInteger(value.budgets.outputBytes) ||
      value.budgets.outputBytes < 1024 * 1024 ||
      value.budgets.outputBytes > 1024 * 1024 * 1024) {
    throw new TypeError('kernel guest budgets are invalid');
  }
  if (!Array.isArray(value.guestCommand) ||
      value.guestCommand.length !== 4 ||
      value.guestCommand[0] !== 'bash' ||
      value.guestCommand[1] !== 'scripts/external-input-kernel-guest.sh' ||
      value.guestCommand[2] !== '--seed' ||
      value.guestCommand[3] !== '/seed/seed.json') {
    throw new TypeError('kernel runner guest command is not canonical');
  }
  if (value.seedContentSha256 !== kernelRunnerSeedContentHash(value)) {
    throw new TypeError('kernel runner seed content hash mismatch');
  }
  return Object.freeze(structuredClone(value));
}

export function createKernelRunnerSeed({
  runId,
  runNonce,
  modelId,
  runtimeImages,
  imageArchiveSha256,
  imageArchiveBytes,
  imageArchiveKeys,
  sourceBundleSha256,
  sourceBundleBytes,
  sourceBundleEntries,
  runner,
  projectName,
  browser = 'chromium',
  sequence = 3,
  strategySeed,
  profileManifestSha256,
  protocolContractSha256,
  budgets,
}) {
  const cell = sequence === 3
    ? { profileId: 'external_input_vusb', domRequired: false }
    : sequence === 4
      ? { profileId: 'external_input_dom_vusb', domRequired: true }
      : {};
  const archiveKeys = imageArchiveKeys ||
    cellRuntimeImageKeys(browser, sequence);
  const seed = {
    schemaVersion: KERNEL_RUNNER_SEED_VERSION,
    seedContentSha256: '',
    runId,
    runNonce,
    modelId,
    platform: 'linux/amd64',
    admission: {
      authority: 'seed-bound-development',
      canonical: false,
      baselineEligible: false,
      releaseAttested: false,
      profileContractVersion: 'humanymous.virtual-usb-profile/v1',
      profileManifestSha256,
      descriptorSetId: 'reference-relative-v1',
      protocolContractSha256,
    },
    runtimeImages: Object.fromEntries(
      archiveKeys.map((key) => [key, runtimeImages[key]]),
    ),
    imageArchive: {
      path: 'images.oci.tar',
      sha256: imageArchiveSha256,
      bytes: imageArchiveBytes,
      imageKeys: structuredClone(archiveKeys),
    },
    sourceBundle: {
      path: 'bundle.tar',
      sha256: sourceBundleSha256,
      bytes: sourceBundleBytes,
      entries: sourceBundleEntries,
    },
    runner: structuredClone(runner),
    innerCompose: {
      projectName,
      browser,
      sequence,
      ...cell,
      pullPolicy: 'never',
    },
    strategy: {
      variant: 'mixed-input',
      seed: strategySeed,
      oracleFeedback: false,
    },
    budgets: structuredClone(budgets),
    guestCommand: [
      'bash',
      'scripts/external-input-kernel-guest.sh',
      '--seed',
      '/seed/seed.json',
    ],
  };
  seed.seedContentSha256 = kernelRunnerSeedContentHash(seed);
  return validateKernelRunnerSeed(seed);
}
