import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

import { assertVirtualUsbAttestation } from '../input.mjs';
import { atomicJson, sha256 } from './common.mjs';
import { parseStrictJson } from './strict-json.mjs';

async function readReceipt(path, label) {
  const raw = await readFile(path, 'utf8');
  return { raw, value: parseStrictJson(raw, label) };
}

export async function createKernelSeedTerminal(input) {
  const [
    result,
    score,
    attestation,
    cleanup,
    teardown,
    compose,
  ] = await Promise.all([
    readReceipt(input.resultPath, 'external input result'),
    readReceipt(input.scorePath, 'Core score receipt'),
    readReceipt(input.attestationPath, 'virtual USB attestation'),
    readReceipt(input.cleanupPath, 'kernel cleanup receipt'),
    readReceipt(input.teardownPath, 'teardown observation'),
    readReceipt(input.composeConfigPath, 'resolved Compose config'),
  ]);
  assertVirtualUsbAttestation(attestation.value);
  if (attestation.value.authority !== 'seed-bound-development' ||
      result.value.runId !== input.runId ||
      score.value.runId !== input.runId ||
      result.value.profileId !== input.profileId ||
      score.value.profileId !== input.profileId ||
      result.value.measurement?.verdict !== score.value.verdict ||
      score.value.scorer !== 'core' ||
      score.value.scoreRecomputed !== false ||
      cleanup.value.kind !== 'kernel-cleanup' ||
      cleanup.value.neutralRelease !== true ||
      cleanup.value.udcUnbound !== true ||
      cleanup.value.configfsResidue !== false ||
      cleanup.value.moduleSetRestored !== true ||
      teardown.value.downExitCode !== 0 ||
      teardown.value.containers.length !== 0 ||
      teardown.value.networks.length !== 0 ||
      teardown.value.volumes.length !== 0) {
    throw new TypeError('kernel seed measurement or teardown evidence is incomplete');
  }
  const receipt = {
    schemaVersion: 'humanymous.kernel-seed-terminal/v1',
    kind: 'kernel-seed-development-terminal',
    status: 'PASS',
    runId: input.runId,
    profileId: input.profileId,
    authority: 'seed-bound-development',
    canonical: false,
    baselineEligible: false,
    releaseAttested: false,
    measurementVerdict: score.value.verdict,
    evidence: {
      virtualUsbAttestationSha256: `sha256:${sha256(attestation.raw)}`,
      kernelCleanupSha256: `sha256:${sha256(cleanup.raw)}`,
      teardownObservationSha256: `sha256:${sha256(teardown.raw)}`,
      coreScoreReceiptSha256: `sha256:${sha256(score.raw)}`,
      composeConfigSha256: `sha256:${sha256(compose.raw)}`,
    },
  };
  await atomicJson(input.destination, receipt);
  return Object.freeze(receipt);
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  createKernelSeedTerminal({
    runId: required('HM_VUSB_RUN_ID'),
    profileId: required('HM_EXTERNAL_MODE'),
    resultPath: required('HM_KERNEL_RESULT'),
    scorePath: required('HM_KERNEL_SCORE'),
    attestationPath: required('HM_KERNEL_ATTESTATION'),
    cleanupPath: required('HM_KERNEL_CLEANUP'),
    teardownPath: required('HM_KERNEL_TEARDOWN'),
    composeConfigPath: required('HM_KERNEL_COMPOSE_CONFIG'),
    destination: required('HM_KERNEL_TERMINAL'),
  }).catch((error) => {
    process.stderr.write(`kernel seed terminal failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
