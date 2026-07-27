import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { atomicJson, exactObject, receiptBase, SHA256, sha256 } from './common.mjs';
import { parseStrictJson } from './strict-json.mjs';

async function readReceipt(path, kind, runId) {
  const raw = await readFile(path, 'utf8');
  const value = parseStrictJson(raw, `${kind} receipt`);
  if (value.kind !== kind || value.runId !== runId) {
    throw new TypeError(`${kind} receipt is not bound to this run`);
  }
  return value;
}

export async function makeVirtualUsbAttestation({
  runId,
  admissionPath,
  preflightPath,
  setupPath,
  preparePath,
  mappingPath,
  composeGuardPath,
  destination,
  gatewayImageDigest,
}) {
  if (!SHA256.test(gatewayImageDigest || '')) throw new TypeError('gateway image digest is invalid');
  const [admission, preflight, setup, prepare, mapping, composeGuard] = await Promise.all([
    readReceipt(admissionPath, 'admission', runId),
    readReceipt(preflightPath, 'preflight', runId),
    readReceipt(setupPath, 'setup', runId),
    readReceipt(preparePath, 'prepare', runId),
    readReceipt(mappingPath, 'device-mapping', runId),
    readReceipt(composeGuardPath, 'compose-guard', runId),
  ]);
  if (mapping.mappingCount !== 6 || mapping.cdi !== false ||
      mapping.exclusiveAssignment !== true ||
      composeGuard.exactDeviceMappings !== 6 ||
      prepare.stableObservations !== 2 ||
      prepare.deviceIdentityCount !== 6 ||
      prepare.driverContractVerified !== true ||
      prepare.observationIntervalMs < 200 ||
      preflight.uinputPresent !== false ||
      setup.kernelEmulated !== true ||
      setup.transport !== 'dummy-hcd' ||
      !SHA256.test(preflight.kernelConfigSha256 || '') ||
      !SHA256.test(setup.descriptorSha256 || '')) {
    throw new TypeError('virtual USB preparation evidence is incomplete');
  }
  const topologySource = JSON.stringify({ host: prepare.host, stableObservations: 2 });
  const result = {
    contractVersion: 'humanymous.virtual-usb-profile/v1',
    runId,
    modelId: admission.modelId,
    authority: admission.authority,
    catalogRevision: admission.catalogRevision,
    catalogSha256: admission.catalogSha256,
    imageIndexDigest: admission.imageIndexDigest,
    platformManifestDigest: admission.platformManifestDigest,
    configDigest: admission.configDigest,
    profileManifestSha256: admission.profileManifestSha256,
    descriptorSetId: admission.descriptorSetId,
    protocolContractSha256: admission.protocolContractSha256,
    hidGatewayImageDigest: gatewayImageDigest,
    descriptorSha256: setup.descriptorSha256,
    topologySha256: `sha256:${sha256(topologySource)}`,
    kernelConfigSha256: preflight.kernelConfigSha256,
    exclusiveAssignment: mapping.exclusiveAssignment,
    uinputPresent: preflight.uinputPresent,
    emulationAttested: setup.kernelEmulated &&
      setup.transport === 'dummy-hcd' &&
      prepare.driverContractVerified,
    physicalAttested: false,
    physicalUsb: false,
    kernelEmulated: setup.kernelEmulated,
    transport: setup.transport,
    deadManReleaseMs: 500,
    ...(admission.authority === 'seed-bound-development'
      ? {
          canonical: false,
          baselineEligible: false,
          releaseAttested: false,
          seedSha256: admission.seedSha256,
        }
      : {}),
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
  makeVirtualUsbAttestation({
    runId: required('HM_VUSB_RUN_ID'),
    admissionPath: required('HM_VUSB_ADMISSION_RECEIPT'),
    preflightPath: required('HM_VUSB_PREFLIGHT_RECEIPT'),
    setupPath: required('HM_VUSB_SETUP_RECEIPT'),
    preparePath: required('HM_VUSB_PREPARE_RECEIPT'),
    mappingPath: required('HM_VUSB_MAPPING_RECEIPT'),
    composeGuardPath: required('HM_VUSB_COMPOSE_GUARD_RECEIPT'),
    destination: required('HM_EXTERNAL_USB_ATTESTATION'),
    gatewayImageDigest: required('HM_VUSB_GATEWAY_IMAGE_DIGEST'),
  }).catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-attestation',
      code: 'PURITY_FAIL',
      message: error.message,
    })}\n`);
    process.exitCode = 1;
  });
}
