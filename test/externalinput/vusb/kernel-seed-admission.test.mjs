import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { createKernelRunnerSeed } from '../kernel-runner/seed.mjs';
import { RUNTIME_IMAGE_FIELDS } from './catalog.mjs';
import { sha256 } from './common.mjs';
import { createKernelSeedAdmission } from './kernel-seed-admission.mjs';

const runtimeImages = Object.fromEntries(
  RUNTIME_IMAGE_FIELDS.map((name, index) => [
    name,
    `sha256:${(index + 1).toString(16).repeat(64)}`,
  ]),
);

test('kernel admission accepts only the exact eight images for its selected cell', async () => {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-kernel-admission-'));
  const profilePath = join(root, 'profile.json');
  const seedPath = join(root, 'seed.json');
  const destination = join(root, 'admission.json');
  const profileRaw = `${JSON.stringify({
    contractVersion: 'humanymous.virtual-usb-profile/v1',
    modelId: 'reference-relative-v1',
  })}\n`;
  const runId = 'kernel-admission-test-0001';
  const previousRunId = process.env.HM_KERNEL_SEED_RUN_ID;
  const previousSeedHash = process.env.HM_KERNEL_SEED_FILE_SHA256;
  try {
    const seed = createKernelRunnerSeed({
      runId,
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
        guestBaseVirtualMiB: 768,
        guestBaseAllocatedBytes: 176 * 1024 * 1024,
        kernelSha256: `sha256:${'a'.repeat(64)}`,
        initramfsSha256: `sha256:${'9'.repeat(64)}`,
      },
      projectName: 'hmn-kernel-admission-test',
      browser: 'chromium',
      sequence: 3,
      strategySeed: 'strongest-human-mimic-0001',
      profileManifestSha256: `sha256:${sha256(profileRaw)}`,
      protocolContractSha256: `sha256:${'7'.repeat(64)}`,
      budgets: {
        cpus: 2,
        memoryMiB: 2048,
        deadlineSeconds: 1800,
        outputBytes: 64 * 1024 * 1024,
      },
    });
    const seedRaw = `${JSON.stringify(seed, null, 2)}\n`;
    await writeFile(profilePath, profileRaw);
    await writeFile(seedPath, seedRaw);
    process.env.HM_KERNEL_SEED_RUN_ID = runId;
    process.env.HM_KERNEL_SEED_FILE_SHA256 = sha256(seedRaw);

    const receipt = await createKernelSeedAdmission({
      seedPath,
      runId,
      destination,
      profileManifestPath: profilePath,
    });

    assert.deepEqual(Object.keys(receipt.runtimeImages), [
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
      JSON.parse(await readFile(destination, 'utf8')).runtimeImages,
      receipt.runtimeImages,
    );
  } finally {
    if (previousRunId === undefined) delete process.env.HM_KERNEL_SEED_RUN_ID;
    else process.env.HM_KERNEL_SEED_RUN_ID = previousRunId;
    if (previousSeedHash === undefined) delete process.env.HM_KERNEL_SEED_FILE_SHA256;
    else process.env.HM_KERNEL_SEED_FILE_SHA256 = previousSeedHash;
    await rm(root, { recursive: true, force: true });
  }
});
