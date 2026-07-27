import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

import { validateKernelRunnerSeed } from '../kernel-runner/seed.mjs';
import { atomicJson, receiptBase, SHA256, sha256 } from './common.mjs';
import { parseStrictJson } from './strict-json.mjs';

export async function createKernelSeedAdmission({
  seedPath,
  runId,
  destination,
  profileManifestPath,
}) {
  const profileRaw = await readFile(profileManifestPath, 'utf8');
  const profile = parseStrictJson(profileRaw, 'kernel seed profile manifest');
  const seed = validateKernelRunnerSeed(parseStrictJson(
    await readFile(seedPath, 'utf8'),
    'kernel runner seed',
  ));
  if (seed.admission.authority !== 'seed-bound-development' ||
      seed.admission.canonical !== false ||
      seed.admission.baselineEligible !== false ||
      seed.admission.releaseAttested !== false ||
      seed.admission.profileContractVersion !==
        'humanymous.virtual-usb-profile/v1' ||
      !SHA256.test(seed.admission.profileManifestSha256 || '') ||
      seed.admission.descriptorSetId !== 'reference-relative-v1' ||
      !SHA256.test(seed.admission.protocolContractSha256 || '') ||
      Object.values(seed.runtimeImages).some((value) => !SHA256.test(value || ''))) {
    throw new TypeError('kernel seed development authority is invalid');
  }
  if (profile.modelId !== seed.modelId ||
      profile.contractVersion !== 'humanymous.virtual-usb-profile/v1' ||
      `sha256:${sha256(profileRaw)}` !== seed.admission.profileManifestSha256) {
    throw new TypeError('kernel seed profile differs from the closed model');
  }
  if (seed.runId !== process.env.HM_KERNEL_SEED_RUN_ID ||
      seed.innerCompose.projectName.length < 1) {
    throw new TypeError('kernel seed admission is not bound to the outer run');
  }
  const profileDigest = seed.runtimeImages.profile;
  const result = {
    ...receiptBase('admission', runId),
    modelId: seed.modelId,
    authority: 'seed-bound-development',
    canonical: false,
    baselineEligible: false,
    releaseAttested: false,
    seedSha256: `sha256:${process.env.HM_KERNEL_SEED_FILE_SHA256}`,
    catalogRevision: 'kernel-seed-development',
    catalogSha256: seed.seedContentSha256,
    imageIndexDigest: profileDigest,
    platformManifestDigest: profileDigest,
    configDigest: seed.sourceBundle.sha256,
    profileManifestSha256: seed.admission.profileManifestSha256,
    descriptorSetId: seed.admission.descriptorSetId,
    protocolContractSha256: seed.admission.protocolContractSha256,
    runtimeImages: seed.runtimeImages,
  };
  await atomicJson(destination, result);
  return Object.freeze(result);
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  createKernelSeedAdmission({
    seedPath: required('HM_KERNEL_SEED_PATH'),
    runId: required('HM_VUSB_RUN_ID'),
    destination: required('HM_KERNEL_SEED_ADMISSION'),
    profileManifestPath: required('HM_KERNEL_PROFILE_MANIFEST'),
  }).catch((error) => {
    process.stderr.write(`kernel seed admission failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
