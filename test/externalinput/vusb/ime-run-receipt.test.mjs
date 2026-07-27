import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { evdevCodeForKey, IME_AXIS_VERSION, vectorFor } from './ime-contract.mjs';
import { createImePolicy } from './ime-policy.mjs';
import { createImeRunReceipt } from './ime-run-receipt.mjs';

test('IME receipt binds result to a private active engine readiness receipt', async (t) => {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-ime-receipt-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const resultPath = join(root, 'result.json');
  const observerPath = join(root, 'observer.json');
  const policyPath = join(root, 'policy.json');
  const readinessPath = join(root, 'readiness.json');
  const stopPath = join(root, 'stop.json');
  const hidEvidencePath = join(root, 'hid.json');
  const brokerEvidencePath = join(root, 'broker.json');
  const composeGuardPath = join(root, 'compose.json');
  const seatEvidencePath = join(root, 'seat.json');
  const destination = join(root, 'run.json');
  await writeFile(resultPath, JSON.stringify({
    axisVersion: IME_AXIS_VERSION,
    runId: 'ime-run',
    browser: 'chromium',
    locale: 'ko-KR',
    vectorId: vectorFor('ko-KR').vectorId,
    inputBackend: 'usb-hid-emulated',
    actorRole: 'input-only',
    hidUsageCount: 7,
    status: 'ACTED',
  }));
  await writeFile(observerPath, JSON.stringify({
    schemaVersion: 'humanymous.virtual-usb-receipt/v1',
    kind: 'ime-framebuffer-observation',
    runId: 'ime-run',
    recordedAt: '2026-07-26T00:00:00.000Z',
    browser: 'chromium',
    locale: 'ko-KR',
    vectorId: vectorFor('ko-KR').vectorId,
    observation: 'framebuffer',
    preeditFramebufferSha256: 'a'.repeat(64),
    commitFramebufferSha256: 'b'.repeat(64),
    preeditCuePixels: 96,
    commitCuePixels: 96,
    status: 'OBSERVED',
  }));
  const policy = createImePolicy('ime-run', 'ko-KR');
  await writeFile(policyPath, JSON.stringify(policy));
  await writeFile(readinessPath, JSON.stringify({
    schemaVersion: 'humanymous.ime-readiness/v1',
    runId: 'ime-run',
    locale: 'ko-KR',
    framework: 'ibus',
    frameworkVersion: 'IBus 1.5.27',
    engineId: 'hangul',
    activeEngine: 'hangul',
    enginePackage: 'ibus-hangul',
    enginePackageVersion: '1.5.4-1',
    enginePackageContentInventorySha256: '1'.repeat(64),
    fontPackage: 'fonts-noto-cjk',
    fontPackageVersion: '1:20220127+repack1-1',
    fontPackageContentInventorySha256: '2'.repeat(64),
    xkbPackage: 'xkb-data',
    xkbPackageVersion: '2.35.1-1',
    xkbPackageContentInventorySha256: '3'.repeat(64),
    privateSessionBus: true,
    networkScope: 'compose-internal-target-only',
    imeStatePersistence: 'tmpfs-only',
    runtimeRootFresh: true,
    configRootFresh: true,
    cacheRootFresh: true,
  }));
  await writeFile(stopPath, JSON.stringify({
    schemaVersion: 'humanymous.ime-stop/v1',
    runId: 'ime-run',
    locale: 'ko-KR',
    engineId: 'hangul',
    activeEngineBeforeStop: 'hangul',
    imeExitRequested: true,
    busSocketResidue: false,
    managedProcessResidue: false,
    stateRootsRemoved: true,
    configEntryCount: 0,
    cacheEntryCount: 0,
  }));
  const policySha256 = policy.policySha256;
  await writeFile(hidEvidencePath, JSON.stringify({
    schemaVersion: 'humanymous.virtual-usb-receipt/v1',
    kind: 'hid-report-evidence',
    runId: 'ime-run',
    recordedAt: '2026-07-26T00:00:00.000Z',
    locale: 'ko-KR',
    vectorId: vectorFor('ko-KR').vectorId,
    policySha256,
    acceptedCommandSequence: 11,
    actionCount: 11,
    actionSha256: 'd'.repeat(64),
    reportCount: 20,
    reportSha256: 'e'.repeat(64),
    policy: {
      policySha256,
      expectedActionCount: 9,
      acceptedActionCount: 9,
      releaseAllCount: 2,
      complete: true,
    },
  }));
  await writeFile(brokerEvidencePath, JSON.stringify({
    schemaVersion: 'humanymous.virtual-usb-receipt/v1',
    kind: 'broker-policy-evidence',
    runId: 'ime-run',
    recordedAt: '2026-07-26T00:00:00.000Z',
    locale: 'ko-KR',
    vectorId: vectorFor('ko-KR').vectorId,
    policySha256,
    firmwareAcceptedCount: 11,
    lastSequenceId: 'ime-run-11',
    acceptedActionSha256: '9'.repeat(64),
    policy: {
      policySha256,
      expectedActionCount: 9,
      acceptedActionCount: 9,
      releaseAllCount: 2,
      complete: true,
    },
  }));
  await writeFile(composeGuardPath, JSON.stringify({
    schemaVersion: 'humanymous.virtual-usb-receipt/v1',
    kind: 'compose-guard',
    runId: 'ime-run',
    exactDeviceMappings: 6,
    controllerImeBusAbsent: true,
    directTextActionForbidden: true,
  }));
  await writeFile(seatEvidencePath, JSON.stringify({
    schemaVersion: 'humanymous.virtual-usb-seat-evidence/v1',
    runId: 'ime-run',
    devices: {
      keyboard: { target: 'vusb-keyboard', rdev: '1001' },
      pointer: { target: 'vusb-pointer', rdev: '1002' },
    },
    imePolicyFileSha256: `sha256:${createHash('sha256')
      .update(JSON.stringify(policy))
      .digest('hex')}`,
    keyboardTransitions: [
      ...vectorFor('ko-KR').usages,
      ...vectorFor('ko-KR').commit,
    ].flatMap((key) => {
      const code = evdevCodeForKey(key);
      return [{ code, value: 1 }, { code, value: 0 }];
    }),
    sequenceComplete: true,
    keyboardEvents: 14,
    pointerEvents: 8,
    syncEvents: 10,
    records: 32,
    eventStreamSha256: 'f'.repeat(64),
  }));
  const receipt = await createImeRunReceipt({
    runId: 'ime-run',
    resultPath,
    observerPath,
    policyPath,
    readinessPath,
    stopPath,
    hidEvidencePath,
    brokerEvidencePath,
    composeGuardPath,
    seatEvidencePath,
    destination,
  });
  assert.equal(receipt.status, 'PASS');
  assert.match(receipt.imeReadinessSha256, /^sha256:[a-f0-9]{64}$/);
  assert.match(receipt.framebufferObservationSha256, /^sha256:[a-f0-9]{64}$/);
  assert.match(receipt.imePolicyFileSha256, /^sha256:[a-f0-9]{64}$/);

  await writeFile(policyPath, `${JSON.stringify(policy)}\n`);
  await assert.rejects(createImeRunReceipt({
    runId: 'ime-run',
    resultPath,
    observerPath,
    policyPath,
    readinessPath,
    stopPath,
    hidEvidencePath,
    brokerEvidencePath,
    composeGuardPath,
    seatEvidencePath,
    destination,
  }), /exact IME FSM/);
  await writeFile(policyPath, JSON.stringify(policy));

  const seatDrift = JSON.parse(await readFile(seatEvidencePath, 'utf8'));
  seatDrift.keyboardTransitions[0].code = 31;
  await writeFile(seatEvidencePath, JSON.stringify(seatDrift));
  await assert.rejects(createImeRunReceipt({
    runId: 'ime-run',
    resultPath,
    observerPath,
    policyPath,
    readinessPath,
    stopPath,
    hidEvidencePath,
    brokerEvidencePath,
    composeGuardPath,
    seatEvidencePath,
    destination,
  }), /exact IME FSM/);
  seatDrift.keyboardTransitions[0].code = evdevCodeForKey(
    vectorFor('ko-KR').usages[0],
  );
  await writeFile(seatEvidencePath, JSON.stringify(seatDrift));

  const stopDrift = JSON.parse(await readFile(stopPath, 'utf8'));
  stopDrift.configEntryCount = 1;
  await writeFile(stopPath, JSON.stringify(stopDrift));
  await assert.rejects(createImeRunReceipt({
    runId: 'ime-run',
    resultPath,
    observerPath,
    policyPath,
    readinessPath,
    stopPath,
    hidEvidencePath,
    brokerEvidencePath,
    composeGuardPath,
    seatEvidencePath,
    destination,
  }), /cleanup evidence/);
  stopDrift.configEntryCount = 0;
  await writeFile(stopPath, JSON.stringify(stopDrift));

  const drift = JSON.parse(await readFile(readinessPath, 'utf8'));
  drift.activeEngine = 'libpinyin';
  await writeFile(readinessPath, JSON.stringify(drift));
  await assert.rejects(createImeRunReceipt({
    runId: 'ime-run',
    resultPath,
    observerPath,
    policyPath,
    readinessPath,
    stopPath,
    hidEvidencePath,
    brokerEvidencePath,
    composeGuardPath,
    seatEvidencePath,
    destination,
  }), /private engine/);
});
