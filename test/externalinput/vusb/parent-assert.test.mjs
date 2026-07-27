import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { provisionalAssert, terminalAssert } from './parent-assert.mjs';
import { RUNTIME_IMAGE_FIELDS } from './catalog.mjs';
import { sha256 } from './common.mjs';
import { createAttemptManifest, createLadderManifest } from './manifest.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-vusb-parent-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const stages = [
    'ladder', 'manifest', 'admission', 'profile', 'static-guard', 'preflight', 'setup', 'prepare',
    'mapping', 'resolved-guard', 'attestation', 'run', 'provisional',
    'cleanup', 'teardown', 'terminal',
  ];
  await Promise.all(stages.map((stage) => mkdir(join(root, stage))));
  const runId = 'vusb-parent-test-0001';
  const digest = (character) => `sha256:${character.repeat(64)}`;
  const device = (name, index) => ({
    hostPath: `/dev/${name}`,
    deviceHex: `f:${index.toString(16)}`,
    sysfsDevice: `15:${index}`,
    sysfsPath: `/sys/devices/virtual/${name}`,
    driverPath: '',
  });
  const base = (kind, fields = {}) => ({
    schemaVersion: 'humanymous.virtual-usb-receipt/v1',
    kind,
    runId,
    recordedAt: '2026-07-27T00:00:00.000Z',
    ...fields,
  });
  const files = {
    admission: base('admission', {
      canonical: true,
      modelId: 'reference-relative-v1',
      authority: 'project-reference',
      catalogRevision: 'test.1',
      catalogSha256: digest('1'),
      catalogFileSha256: digest('2'),
      catalogSignaturePolicyId: 'test-policy',
      runtimeImages: Object.fromEntries(
        RUNTIME_IMAGE_FIELDS.map((name) => [name, digest('3')]),
      ),
      imageIndexDigest: digest('4'),
      platform: 'linux/amd64',
      platformManifestDigest: digest('5'),
      configDigest: digest('6'),
      profileManifestSha256: digest('7'),
      descriptorSetId: 'reference-relative-v1',
      protocolContractSha256: digest('8'),
      attestationBundleSha256: digest('9'),
      sbomSha256: digest('a'),
      vulnerabilityPolicySha256: digest('b'),
      revocationSnapshotSha256: digest('c'),
      revocationRevision: '2026-07-26.1',
      scannerDatabaseSnapshot: '2026-07-26',
      validatorSourceSha256: digest('a'),
    }),
    profileVerification: base('profile-verification', {
      modelId: 'reference-relative-v1',
      profileManifestSha256: digest('7'),
      entireRootfsValidated: true,
      gatewaySubpath: '/profile',
    }),
    staticGuard: base('compose-static-guard', {
      phase: 'static',
      composeConfigSha256: digest('b'),
      exactDeviceMappings: 0,
      cdi: false,
      controllerImeBusAbsent: true,
      directTextActionForbidden: true,
    }),
    preflight: base('preflight', {
      kernelRelease: 'test',
      kernelConfigSha256: digest('c'),
      loadedModulesSha256: digest('d'),
      dummyHcdPreloaded: false,
      uinputPresent: false,
    }),
    setup: base('setup', {
      gadgetName: `humanymous-${runId}`,
      udc: 'dummy_udc.0',
      gadgetCommand: '/dev/ttyGS0',
      gadgetKeyboard: '/dev/hidg0',
      gadgetPointer: '/dev/hidg1',
      descriptorSha256: digest('e'),
      kernelEmulated: true,
      transport: 'dummy-hcd',
    }),
    prepare: base('prepare', {
      stableObservations: 2,
      observationIntervalMs: 250,
      deviceIdentityCount: 6,
      driverContractVerified: true,
      gadget: {
        command: device('ttyGS0', 1),
        keyboard: device('hidg0', 2),
        pointer: device('hidg1', 3),
      },
      host: {
        command: {
          ...device('ttyACM0', 4),
          driverPath: '/sys/bus/usb/drivers/cdc_acm',
        },
        keyboard: {
          ...device('event0', 5),
          driverPath: '/sys/bus/usb/drivers/usbhid',
        },
        pointer: {
          ...device('event1', 6),
          driverPath: '/sys/bus/hid/drivers/hid-generic',
        },
      },
    }),
    mapping: base('device-mapping', {
      mappingCount: 6,
      exclusiveAssignment: true,
      mode: 'compose-exact-path',
      cdi: false,
      overrideSha256: digest('f'),
    }),
    composeGuard: base('compose-guard', {
      phase: 'resolved',
      composeConfigSha256: digest('0'),
      exactDeviceMappings: 6,
      exclusiveAssignment: true,
      cdi: false,
      controllerImeBusAbsent: true,
      directTextActionForbidden: true,
    }),
    run: base('run', {
      profileId: 'external_input_vusb',
      browserEngine: 'chromium',
      status: 'PASS',
      measurementVerdict: 'CHALLENGE',
      resultSha256: digest('1'),
      seatEvidenceSha256: digest('2'),
      usbOrigin: 'kernel-emulated',
      physicalUsb: false,
    }),
    cleanup: base('kernel-cleanup', {
      neutralRelease: true,
      neutralKeyboardBytes: 8,
      neutralPointerBytes: 4,
      udcUnbound: true,
      configfsResidue: false,
      moduleSetRestored: true,
    }),
  };
  const ladder = createLadderManifest({
    ladderId: 'vusb-ladder-parent-test-0001',
    modelId: 'reference-relative-v1',
    catalogSha256: digest('1'),
    runtimeImages: files.admission.runtimeImages,
  });
  const ladderRaw = `${JSON.stringify(ladder, null, 2)}\n`;
  const attempt = createAttemptManifest({
    ladder,
    ladderManifestSha256: `sha256:${sha256(ladderRaw)}`,
    runId,
    axis: 'control',
    browser: 'chromium',
    sequence: 3,
    profileId: 'external_input_vusb',
    childProject: 'hm-vusb-child-0001',
    parentProject: 'hm-vusb-parent-test-0001',
  });
  const attemptRaw = `${JSON.stringify(attempt, null, 2)}\n`;
  files.admission.ladderManifestSha256 = `sha256:${sha256(ladderRaw)}`;
  files.admission.attemptManifestSha256 = `sha256:${sha256(attemptRaw)}`;
  files.admission.selectedBrowserImageDigest = attempt.selectedBrowserImageDigest;
  await Promise.all([
    writeFile(join(root, 'ladder', 'ladder.json'), ladderRaw),
    writeFile(join(root, 'manifest', 'attempt.json'), attemptRaw),
    writeFile(join(root, 'admission', 'admission.json'), `${JSON.stringify(files.admission)}\n`),
    writeFile(join(root, 'profile', 'profile-verification.json'), `${JSON.stringify(files.profileVerification)}\n`),
    writeFile(join(root, 'static-guard', 'compose-static-guard.json'), `${JSON.stringify(files.staticGuard)}\n`),
    writeFile(join(root, 'preflight', 'preflight.json'), `${JSON.stringify(files.preflight)}\n`),
    writeFile(join(root, 'setup', 'setup.json'), `${JSON.stringify(files.setup)}\n`),
    writeFile(join(root, 'prepare', 'prepare.json'), `${JSON.stringify(files.prepare)}\n`),
    writeFile(join(root, 'mapping', 'device-mapping.json'), `${JSON.stringify(files.mapping)}\n`),
    writeFile(join(root, 'resolved-guard', 'compose-guard.json'), `${JSON.stringify(files.composeGuard)}\n`),
    writeFile(join(root, 'attestation', 'virtual-usb-attestation.json'), `${JSON.stringify({
      contractVersion: 'humanymous.virtual-usb-profile/v1',
      runId,
      modelId: 'reference-relative-v1',
      authority: 'project-reference',
      catalogRevision: 'test.1',
      catalogSha256: digest('1'),
      imageIndexDigest: digest('4'),
      platformManifestDigest: digest('5'),
      configDigest: digest('6'),
      profileManifestSha256: digest('7'),
      descriptorSetId: 'reference-relative-v1',
      protocolContractSha256: digest('8'),
      hidGatewayImageDigest: digest('3'),
      descriptorSha256: digest('e'),
      topologySha256: digest('9'),
      kernelConfigSha256: digest('c'),
      exclusiveAssignment: true,
      uinputPresent: false,
      emulationAttested: true,
      physicalAttested: false,
      physicalUsb: false,
      kernelEmulated: true,
      transport: 'dummy-hcd',
      deadManReleaseMs: 500,
    })}\n`),
    writeFile(join(root, 'run', 'run.json'), `${JSON.stringify(files.run)}\n`),
    writeFile(join(root, 'cleanup', 'kernel-cleanup.json'), `${JSON.stringify(files.cleanup)}\n`),
  ]);
  await provisionalAssert({
    runId,
    runReceiptPath: join(root, 'run', 'run.json'),
    destination: join(root, 'provisional', 'provisional-assertion.json'),
    now: new Date('2026-07-27T00:01:00.000Z'),
  });
  const teardown = {
    schemaVersion: 'humanymous.virtual-usb-teardown/v1',
    runId,
    projectName: 'hm-vusb-child-0001',
    downExitCode: 0,
    containers: [],
    networks: [],
    volumes: [],
    observedAt: '2026-07-27T00:02:00.000Z',
  };
  await writeFile(
    join(root, 'teardown', 'teardown-observation.json'),
    `${JSON.stringify(teardown)}\n`,
  );
  return {
    root,
    runId,
    teardown,
    ladderManifestPath: join(root, 'ladder', 'ladder.json'),
    attemptManifestPath: join(root, 'manifest', 'attempt.json'),
  };
}

test('terminal assertion stays pending until cleanup and exact child teardown are proven', async (t) => {
  const input = await fixture(t);
  const terminal = await terminalAssert({
    runId: input.runId,
    projectName: input.teardown.projectName,
    parentProject: 'hm-vusb-parent-test-0001',
    receiptsRoot: input.root,
    teardownObservationPath: join(input.root, 'teardown', 'teardown-observation.json'),
    ladderManifestPath: input.ladderManifestPath,
    attemptManifestPath: input.attemptManifestPath,
    destination: join(input.root, 'terminal', 'terminal.json'),
    releasePath: join(input.root, 'terminal', 'release-lock'),
    now: new Date('2026-07-27T00:03:00.000Z'),
  });
  assert.equal(terminal.status, 'PASS');
  assert.equal(terminal.measurementVerdict, 'CHALLENGE');
  assert.match(terminal.selectedBrowserImageDigest, /^sha256:3{64}$/);
  assert.equal(
    await readFile(join(input.root, 'terminal', 'release-lock'), 'utf8'),
    `${input.runId}\n`,
  );
});

test('parent rejects child container, network, or volume residue', async (t) => {
  const input = await fixture(t);
  input.teardown.containers = ['unexpected-container'];
  await writeFile(
    join(input.root, 'teardown', 'teardown-observation.json'),
    `${JSON.stringify(input.teardown)}\n`,
  );
  await assert.rejects(() => terminalAssert({
    runId: input.runId,
    projectName: input.teardown.projectName,
    parentProject: 'hm-vusb-parent-test-0001',
    receiptsRoot: input.root,
    teardownObservationPath: join(input.root, 'teardown', 'teardown-observation.json'),
    ladderManifestPath: input.ladderManifestPath,
    attemptManifestPath: input.attemptManifestPath,
    destination: join(input.root, 'terminal', 'terminal.json'),
    releasePath: join(input.root, 'terminal', 'release-lock'),
  }), /container.*residue/);
});

test('parent rejects an attempt bound to another parent project', async (t) => {
  const input = await fixture(t);
  await assert.rejects(() => terminalAssert({
    runId: input.runId,
    projectName: input.teardown.projectName,
    parentProject: 'hm-vusb-parent-substituted',
    receiptsRoot: input.root,
    teardownObservationPath: join(input.root, 'teardown', 'teardown-observation.json'),
    ladderManifestPath: input.ladderManifestPath,
    attemptManifestPath: input.attemptManifestPath,
    destination: join(input.root, 'terminal', 'terminal.json'),
    releasePath: join(input.root, 'terminal', 'release-lock'),
  }), /attempt manifest is not bound/);
});

test('parent never turns FAIL or UNAVAILABLE into a successful terminal result', async (t) => {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-vusb-parent-reject-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await writeFile(join(root, 'run.json'), JSON.stringify({
    schemaVersion: 'humanymous.virtual-usb-receipt/v1',
    kind: 'run',
    runId: 'vusb-parent-test-0002',
    recordedAt: '2026-07-27T00:00:00.000Z',
    status: 'FAIL',
  }));
  await assert.rejects(() => provisionalAssert({
    runId: 'vusb-parent-test-0002',
    runReceiptPath: join(root, 'run.json'),
    destination: join(root, 'provisional.json'),
  }), /cannot become a successful terminal result/);
});
