import { open, readFile } from 'node:fs/promises';
import { basename, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { assertVirtualUsbAttestation } from '../input.mjs';
import { atomicJson, exactObject, receiptBase, SHA256, sha256 } from './common.mjs';
import {
  loadLadderManifest,
  validateAttemptManifest,
} from './manifest.mjs';
import { parseStrictJson } from './strict-json.mjs';

async function receipt(path, kind, runId) {
  const raw = await readFile(path, 'utf8');
  const value = parseStrictJson(raw, `${kind} receipt`);
  if (value.kind !== kind || value.runId !== runId) {
    throw new TypeError(`${kind} receipt is not bound to this run`);
  }
  return { value, hash: `sha256:${sha256(raw)}` };
}

function sha(value, label) {
  if (!SHA256.test(value || '')) throw new TypeError(`${label} is invalid`);
  return value;
}

function validateDeviceSet(prepare) {
  const identities = [];
  for (const side of ['gadget', 'host']) {
    if (!prepare[side] || typeof prepare[side] !== 'object') {
      throw new TypeError(`prepare ${side} device set is missing`);
    }
    for (const name of ['command', 'keyboard', 'pointer']) {
      const device = prepare[side][name];
      if (!device || !String(device.hostPath || '').startsWith('/dev/') ||
          !/^[0-9a-f]+:[0-9a-f]+$/.test(device.deviceHex || '') ||
          !/^\d+:\d+$/.test(device.sysfsDevice || '') ||
          !String(device.sysfsPath || '').startsWith('/sys/') ||
          (side === 'host' && !String(device.driverPath || '').startsWith('/sys/')) ||
          (side === 'gadget' && device.driverPath !== '' &&
            !String(device.driverPath || '').startsWith('/sys/'))) {
        throw new TypeError(`prepare ${side}.${name} identity is invalid`);
      }
      const [major, minor] = device.deviceHex
        .split(':')
        .map((part) => Number.parseInt(part, 16));
      if (device.sysfsDevice !== `${major}:${minor}`) {
        throw new TypeError(`prepare ${side}.${name} sysfs identity is invalid`);
      }
      identities.push(device.sysfsDevice);
    }
  }
  if (new Set(identities).size !== 6 ||
      !/(?:^|\/)cdc_acm$/.test(prepare.host.command.driverPath) ||
      !/(?:^|\/)(?:usbhid|hid-generic)$/.test(prepare.host.keyboard.driverPath) ||
      !/(?:^|\/)(?:usbhid|hid-generic)$/.test(prepare.host.pointer.driverPath)) {
    throw new TypeError('prepare device driver contract is invalid');
  }
}

export async function provisionalAssert({
  runId,
  runReceiptPath,
  destination,
  now = new Date(),
}) {
  const run = await receipt(runReceiptPath, 'run', runId);
  if (!['PASS', 'RESIDUAL'].includes(run.value.status)) {
    throw new TypeError('run receipt cannot become a successful terminal result');
  }
  const result = {
    ...receiptBase('provisional-assertion', runId, now),
    status: 'PENDING_CLEANUP',
    measuredStatus: run.value.status,
    runReceiptSha256: run.hash,
  };
  await atomicJson(destination, result);
  return result;
}

export async function terminalAssert({
  runId,
  projectName,
  parentProject,
  receiptsRoot,
  teardownObservationPath,
  ladderManifestPath,
  attemptManifestPath,
  destination,
  releasePath,
  now = new Date(),
}) {
  const ladder = await loadLadderManifest(ladderManifestPath);
  const attemptRaw = await readFile(attemptManifestPath, 'utf8');
  const attempt = validateAttemptManifest(
    parseStrictJson(attemptRaw, 'attempt manifest'),
    ladder.value,
  );
  if (attempt.ladderManifestSha256 !== ladder.sha256 ||
      attempt.runId !== runId ||
      attempt.childProject !== projectName ||
      attempt.parentProject !== parentProject) {
    throw new TypeError('attempt manifest is not bound to this terminal assertion');
  }
  const [
    admission, profileVerification, staticGuard, preflight, setup, prepare, mapping,
    composeGuard, run, provisional, cleanup,
  ] = await Promise.all([
    receipt(join(receiptsRoot, 'admission', 'admission.json'), 'admission', runId),
    receipt(join(receiptsRoot, 'profile', 'profile-verification.json'), 'profile-verification', runId),
    receipt(join(receiptsRoot, 'static-guard', 'compose-static-guard.json'), 'compose-static-guard', runId),
    receipt(join(receiptsRoot, 'preflight', 'preflight.json'), 'preflight', runId),
    receipt(join(receiptsRoot, 'setup', 'setup.json'), 'setup', runId),
    receipt(join(receiptsRoot, 'prepare', 'prepare.json'), 'prepare', runId),
    receipt(join(receiptsRoot, 'mapping', 'device-mapping.json'), 'device-mapping', runId),
    receipt(join(receiptsRoot, 'resolved-guard', 'compose-guard.json'), 'compose-guard', runId),
    receipt(join(receiptsRoot, 'run', 'run.json'), 'run', runId),
    receipt(join(receiptsRoot, 'provisional', 'provisional-assertion.json'), 'provisional-assertion', runId),
    receipt(join(receiptsRoot, 'cleanup', 'kernel-cleanup.json'), 'kernel-cleanup', runId),
  ]);
  const attestationRaw = await readFile(
    join(receiptsRoot, 'attestation', 'virtual-usb-attestation.json'),
    'utf8',
  );
  const attestation = parseStrictJson(attestationRaw, 'virtual USB attestation');
  assertVirtualUsbAttestation(attestation);
  if (attestation.runId !== runId) {
    throw new TypeError('virtual USB attestation is not bound to this run');
  }
  const teardownRaw = await readFile(teardownObservationPath, 'utf8');
  const teardown = parseStrictJson(teardownRaw, 'teardown observation');
  exactObject(teardown, [
    'schemaVersion', 'runId', 'projectName', 'downExitCode',
    'containers', 'networks', 'volumes', 'observedAt',
  ], 'teardown observation');
  if (teardown.schemaVersion !== 'humanymous.virtual-usb-teardown/v1' ||
      teardown.runId !== runId || teardown.projectName !== projectName ||
      teardown.downExitCode !== 0) {
    throw new TypeError('child teardown observation is not bound or successful');
  }
  for (const field of ['containers', 'networks', 'volumes']) {
    if (!Array.isArray(teardown[field]) || teardown[field].length !== 0) {
      throw new TypeError(`child project ${field} residue remains`);
    }
  }
  if (provisional.value.status !== 'PENDING_CLEANUP' ||
      provisional.value.measuredStatus !== run.value.status ||
      provisional.value.runReceiptSha256 !== run.hash ||
      admission.value.canonical !== true ||
      typeof admission.value.modelId !== 'string' ||
      admission.value.authority !== 'project-reference' ||
      !admission.value.runtimeImages ||
      !SHA256.test(admission.value.runtimeImages.labCore || '') ||
      admission.value.ladderManifestSha256 !== ladder.sha256 ||
      admission.value.attemptManifestSha256 !== `sha256:${sha256(attemptRaw)}` ||
      admission.value.selectedBrowserImageDigest !==
        attempt.selectedBrowserImageDigest ||
      !/^linux\/[a-z0-9_]+$/.test(admission.value.platform || '') ||
      profileVerification.value.modelId !== admission.value.modelId ||
      profileVerification.value.profileManifestSha256 !==
        admission.value.profileManifestSha256 ||
      profileVerification.value.entireRootfsValidated !== true ||
      profileVerification.value.gatewaySubpath !== '/profile' ||
      staticGuard.value.phase !== 'static' ||
      staticGuard.value.exactDeviceMappings !== 0 ||
      staticGuard.value.cdi !== false ||
      preflight.value.uinputPresent !== false ||
      setup.value.gadgetName !== `humanymous-${runId}` ||
      typeof setup.value.udc !== 'string' ||
      setup.value.kernelEmulated !== true ||
      setup.value.transport !== 'dummy-hcd' ||
      prepare.value.stableObservations !== 2 ||
      prepare.value.deviceIdentityCount !== 6 ||
      prepare.value.driverContractVerified !== true ||
      prepare.value.observationIntervalMs < 200 ||
      mapping.value.mappingCount !== 6 ||
      mapping.value.exclusiveAssignment !== true ||
      mapping.value.mode !== 'compose-exact-path' ||
      mapping.value.cdi !== false ||
      composeGuard.value.phase !== 'resolved' ||
      composeGuard.value.exactDeviceMappings !== 6 ||
      composeGuard.value.exclusiveAssignment !== true ||
      composeGuard.value.cdi !== false ||
      composeGuard.value.controllerImeBusAbsent !== true ||
      composeGuard.value.directTextActionForbidden !== true ||
      !['PASS', 'RESIDUAL'].includes(run.value.status) ||
      !['ALLOW', 'CHALLENGE', 'DENY', 'NOT_APPLICABLE']
        .includes(run.value.measurementVerdict) ||
      run.value.usbOrigin !== 'kernel-emulated' ||
      run.value.physicalUsb !== false ||
      (attempt.axis === 'control' &&
        (attempt.profileId !== run.value.profileId ||
          (run.value.axis || 'control') !== 'control')) ||
      (attempt.axis === 'ime' &&
        (run.value.profileId !== 'ime-composition-vusb' ||
          run.value.axis !== 'ime-composition-vusb')) ||
      attempt.browser !== run.value.browserEngine ||
      attempt.locale !== (run.value.locale || '') ||
      cleanup.value.neutralRelease !== true ||
      cleanup.value.neutralKeyboardBytes !== 8 ||
      cleanup.value.neutralPointerBytes !== 4 ||
      cleanup.value.udcUnbound !== true ||
      cleanup.value.configfsResidue !== false ||
      cleanup.value.moduleSetRestored !== true) {
    throw new TypeError('provisional result or kernel cleanup evidence is invalid');
  }
  for (const [value, label] of [
    [admission.value.catalogSha256, 'admission catalog hash'],
    [admission.value.catalogFileSha256, 'admission catalog file hash'],
    [admission.value.imageIndexDigest, 'admission image index digest'],
    [admission.value.platformManifestDigest, 'admission platform manifest digest'],
    [admission.value.configDigest, 'admission config digest'],
    [admission.value.profileManifestSha256, 'admission profile manifest hash'],
    [admission.value.protocolContractSha256, 'admission protocol hash'],
    [admission.value.attestationBundleSha256, 'admission bundle hash'],
    [admission.value.ladderManifestSha256, 'admission ladder manifest hash'],
    [admission.value.attemptManifestSha256, 'admission attempt manifest hash'],
    [admission.value.selectedBrowserImageDigest, 'admission selected browser image digest'],
    [admission.value.sbomSha256, 'admission SPDX hash'],
    [admission.value.vulnerabilityPolicySha256, 'admission vulnerability policy hash'],
    [admission.value.revocationSnapshotSha256, 'admission revocation snapshot hash'],
    [admission.value.validatorSourceSha256, 'admission validator hash'],
    [preflight.value.kernelConfigSha256, 'preflight kernel config hash'],
    [preflight.value.loadedModulesSha256, 'preflight module inventory hash'],
    [setup.value.descriptorSha256, 'setup descriptor hash'],
    [mapping.value.overrideSha256, 'mapping override hash'],
    [staticGuard.value.composeConfigSha256, 'static Compose hash'],
    [composeGuard.value.composeConfigSha256, 'resolved Compose hash'],
    [run.value.resultSha256, 'run result hash'],
    [run.value.seatEvidenceSha256, 'run seat evidence hash'],
  ]) sha(value, label);
  validateDeviceSet(prepare.value);
  if (attestation.catalogSha256 !== admission.value.catalogSha256 ||
      attestation.imageIndexDigest !== admission.value.imageIndexDigest ||
      attestation.platformManifestDigest !== admission.value.platformManifestDigest ||
      attestation.configDigest !== admission.value.configDigest ||
      attestation.profileManifestSha256 !== admission.value.profileManifestSha256 ||
      attestation.protocolContractSha256 !== admission.value.protocolContractSha256 ||
      attestation.descriptorSha256 !== setup.value.descriptorSha256 ||
      attestation.kernelConfigSha256 !== preflight.value.kernelConfigSha256) {
    throw new TypeError('virtual USB attestation does not bind its predecessor receipts');
  }
  const result = {
    ...receiptBase('terminal', runId, now),
    canonical: true,
    status: run.value.status,
    measurementVerdict: run.value.measurementVerdict,
    profileId: run.value.profileId || '',
    axis: run.value.axis || 'control',
    browserEngine: run.value.browserEngine || '',
    locale: run.value.locale || '',
    vectorId: run.value.vectorId || '',
    resultSha256: run.value.resultSha256,
    selectedBrowserImageDigest: attempt.selectedBrowserImageDigest,
    projectName,
    evidence: {
      admissionReceiptSha256: admission.hash,
      ladderManifestSha256: ladder.sha256,
      attemptManifestSha256: `sha256:${sha256(attemptRaw)}`,
      profileVerificationReceiptSha256: profileVerification.hash,
      staticComposeGuardReceiptSha256: staticGuard.hash,
      preflightReceiptSha256: preflight.hash,
      setupReceiptSha256: setup.hash,
      prepareReceiptSha256: prepare.hash,
      deviceMappingReceiptSha256: mapping.hash,
      composeGuardReceiptSha256: composeGuard.hash,
      virtualUsbAttestationSha256: `sha256:${sha256(attestationRaw)}`,
      runReceiptSha256: run.hash,
      provisionalAssertionReceiptSha256: provisional.hash,
      kernelCleanupReceiptSha256: cleanup.hash,
      teardownObservationSha256: `sha256:${sha256(teardownRaw)}`,
    },
  };
  await atomicJson(destination, result);
  const handle = await open(releasePath, 'wx', 0o600);
  await handle.writeFile(`${runId}\n`);
  await handle.close();
  return Object.freeze(result);
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

async function main() {
  const action = process.argv[2];
  const root = required('HM_VUSB_RECEIPT_ROOT');
  if (action === 'provisional') {
    await provisionalAssert({
      runId: required('HM_VUSB_RUN_ID'),
      runReceiptPath: join(root, 'run', 'run.json'),
      destination: required('HM_VUSB_PROVISIONAL_RECEIPT'),
    });
  } else if (action === 'terminal') {
    await terminalAssert({
      runId: required('HM_VUSB_RUN_ID'),
      projectName: required('HM_VUSB_CHILD_PROJECT'),
      parentProject: required('HM_VUSB_PARENT_PROJECT'),
      receiptsRoot: root,
      teardownObservationPath: join(root, 'teardown', 'teardown-observation.json'),
      ladderManifestPath: required('HM_VUSB_LADDER_MANIFEST'),
      attemptManifestPath: required('HM_VUSB_ATTEMPT_MANIFEST'),
      destination: required('HM_VUSB_TERMINAL_RECEIPT'),
      releasePath: required('HM_VUSB_RELEASE_LOCK'),
    });
  } else {
    throw new TypeError('parent assertion action must be provisional or terminal');
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-parent-assert',
      code: 'SAFETY_ABORT',
      message: error.message,
    })}\n`);
    process.exitCode = 1;
  });
}
