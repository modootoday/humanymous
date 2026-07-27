import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { evdevCodeForKey, validateImeResult, vectorFor } from './ime-contract.mjs';
import { atomicJson, canonicalJson, exactObject, receiptBase, sha256 } from './common.mjs';
import { validateImePolicy } from './ime-policy.mjs';
import { parseStrictJson } from './strict-json.mjs';
import { readSeatEvidence } from './seat-evidence.mjs';

export async function createImeRunReceipt({
  runId,
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
  now = new Date(),
}) {
  const raw = await readFile(resultPath, 'utf8');
  const result = validateImeResult(parseStrictJson(raw, 'IME result'));
  if (result.runId !== runId || result.status !== 'ACTED') {
    throw new TypeError('IME input actor is not bound to this run or is incomplete');
  }
  const observerRaw = await readFile(observerPath, 'utf8');
  const observer = parseStrictJson(observerRaw, 'IME framebuffer observation');
  exactObject(observer, [
    'schemaVersion', 'kind', 'runId', 'recordedAt', 'browser', 'locale',
    'vectorId', 'observation', 'preeditFramebufferSha256',
    'commitFramebufferSha256', 'preeditCuePixels', 'commitCuePixels', 'status',
  ], 'IME framebuffer observation');
  if (observer.schemaVersion !== 'humanymous.virtual-usb-receipt/v1' ||
      observer.kind !== 'ime-framebuffer-observation' ||
      observer.runId !== runId || observer.browser !== result.browser ||
      observer.locale !== result.locale || observer.vectorId !== result.vectorId ||
      observer.observation !== 'framebuffer' ||
      !/^[a-f0-9]{64}$/.test(observer.preeditFramebufferSha256 || '') ||
      !/^[a-f0-9]{64}$/.test(observer.commitFramebufferSha256 || '') ||
      !Number.isInteger(observer.preeditCuePixels) ||
      observer.preeditCuePixels < 96 || observer.preeditCuePixels > 921_600 ||
      !Number.isInteger(observer.commitCuePixels) ||
      observer.commitCuePixels < 96 || observer.commitCuePixels > 921_600 ||
      observer.status !== 'OBSERVED') {
    throw new TypeError('independent IME framebuffer observation is incomplete or unbound');
  }
  const policyRaw = await readFile(policyPath, 'utf8');
  const policy = validateImePolicy(
    parseStrictJson(policyRaw, 'IME HID policy'),
    runId,
  );
  if (policy.locale !== result.locale || policy.vectorId !== result.vectorId) {
    throw new TypeError('IME HID policy is not bound to the actor vector');
  }
  const readinessRaw = await readFile(readinessPath, 'utf8');
  const readiness = parseStrictJson(readinessRaw, 'IME readiness');
  exactObject(readiness, [
    'schemaVersion', 'runId', 'locale', 'framework', 'frameworkVersion',
    'engineId', 'activeEngine', 'enginePackage', 'enginePackageVersion',
    'enginePackageContentInventorySha256', 'fontPackage', 'fontPackageVersion',
    'fontPackageContentInventorySha256', 'xkbPackage', 'xkbPackageVersion',
    'xkbPackageContentInventorySha256', 'privateSessionBus', 'networkScope',
    'imeStatePersistence', 'runtimeRootFresh', 'configRootFresh', 'cacheRootFresh',
  ], 'IME readiness');
  const expectedEngine = {
    'ko-KR': 'hangul',
    'zh-CN': 'libpinyin',
    'ja-JP': 'anthy',
  }[result.locale];
  const expectedEnginePackage = {
    'ko-KR': 'ibus-hangul',
    'zh-CN': 'ibus-libpinyin',
    'ja-JP': 'ibus-anthy',
  }[result.locale];
  const packageVersion = /^[A-Za-z0-9][A-Za-z0-9.+:~_-]{0,127}$/;
  const bareSha256 = /^[a-f0-9]{64}$/;
  if (readiness.schemaVersion !== 'humanymous.ime-readiness/v1' ||
      readiness.runId !== runId || readiness.locale !== result.locale ||
      readiness.framework !== 'ibus' ||
      typeof readiness.frameworkVersion !== 'string' ||
      readiness.frameworkVersion.length < 1 || readiness.frameworkVersion.length > 64 ||
      readiness.engineId !== expectedEngine || readiness.activeEngine !== expectedEngine ||
      readiness.enginePackage !== expectedEnginePackage ||
      !packageVersion.test(readiness.enginePackageVersion || '') ||
      !bareSha256.test(readiness.enginePackageContentInventorySha256 || '') ||
      readiness.fontPackage !== 'fonts-noto-cjk' ||
      !packageVersion.test(readiness.fontPackageVersion || '') ||
      !bareSha256.test(readiness.fontPackageContentInventorySha256 || '') ||
      readiness.xkbPackage !== 'xkb-data' ||
      !packageVersion.test(readiness.xkbPackageVersion || '') ||
      !bareSha256.test(readiness.xkbPackageContentInventorySha256 || '') ||
      readiness.privateSessionBus !== true || readiness.runtimeRootFresh !== true ||
      readiness.networkScope !== 'compose-internal-target-only' ||
      readiness.imeStatePersistence !== 'tmpfs-only' ||
      readiness.configRootFresh !== true || readiness.cacheRootFresh !== true) {
    throw new TypeError('IME readiness is not bound to the selected private engine');
  }
  const stopRaw = await readFile(stopPath, 'utf8');
  const stop = parseStrictJson(stopRaw, 'IME stop evidence');
  exactObject(stop, [
    'schemaVersion', 'runId', 'locale', 'engineId', 'activeEngineBeforeStop',
    'imeExitRequested', 'busSocketResidue', 'managedProcessResidue',
    'stateRootsRemoved', 'configEntryCount', 'cacheEntryCount',
  ], 'IME stop evidence');
  if (stop.schemaVersion !== 'humanymous.ime-stop/v1' ||
      stop.runId !== runId || stop.locale !== result.locale ||
      stop.engineId !== expectedEngine || stop.activeEngineBeforeStop !== expectedEngine ||
      stop.imeExitRequested !== true ||
      stop.busSocketResidue !== false || stop.managedProcessResidue !== false ||
      stop.stateRootsRemoved !== true ||
      stop.configEntryCount !== 0 || stop.cacheEntryCount !== 0) {
    throw new TypeError('IME browser, bus, or engine cleanup evidence is incomplete');
  }
  const hidRaw = await readFile(hidEvidencePath, 'utf8');
  const hid = parseStrictJson(hidRaw, 'HID report evidence');
  exactObject(hid, [
    'schemaVersion', 'kind', 'runId', 'recordedAt', 'locale', 'vectorId',
    'policySha256', 'acceptedCommandSequence', 'actionCount', 'actionSha256',
    'reportCount', 'reportSha256', 'policy',
  ], 'HID report evidence');
  exactObject(hid.policy, [
    'policySha256', 'expectedActionCount', 'acceptedActionCount',
    'releaseAllCount', 'complete',
  ], 'HID report policy evidence');
  if (hid.schemaVersion !== 'humanymous.virtual-usb-receipt/v1' ||
      hid.kind !== 'hid-report-evidence' || hid.runId !== runId ||
      hid.locale !== result.locale || hid.vectorId !== result.vectorId ||
      hid.policySha256 !== policy.policySha256 ||
      !/^[a-f0-9]{64}$/.test(hid.actionSha256 || '') ||
      !/^[a-f0-9]{64}$/.test(hid.reportSha256 || '') ||
      hid.policy.policySha256 !== hid.policySha256 ||
      hid.policy.expectedActionCount !== result.hidUsageCount + 2 ||
      hid.policy.acceptedActionCount !== hid.policy.expectedActionCount ||
      hid.policy.releaseAllCount < 2 || hid.policy.complete !== true ||
      hid.actionCount < hid.policy.expectedActionCount + 2 ||
      hid.reportCount < result.hidUsageCount * 2) {
    throw new TypeError('independent HID report evidence is incomplete or unbound');
  }
  const brokerRaw = await readFile(brokerEvidencePath, 'utf8');
  const broker = parseStrictJson(brokerRaw, 'broker policy evidence');
  exactObject(broker, [
    'schemaVersion', 'kind', 'runId', 'recordedAt', 'locale', 'vectorId',
    'policySha256', 'firmwareAcceptedCount', 'lastSequenceId',
    'acceptedActionSha256', 'policy',
  ], 'broker policy evidence');
  exactObject(broker.policy, [
    'policySha256', 'expectedActionCount', 'acceptedActionCount',
    'releaseAllCount', 'complete',
  ], 'broker policy completion');
  if (broker.schemaVersion !== 'humanymous.virtual-usb-receipt/v1' ||
      broker.kind !== 'broker-policy-evidence' || broker.runId !== runId ||
      broker.locale !== result.locale || broker.vectorId !== result.vectorId ||
      broker.policySha256 !== hid.policySha256 ||
      broker.policy.policySha256 !== hid.policySha256 ||
      broker.policy.expectedActionCount !== hid.policy.expectedActionCount ||
      broker.policy.acceptedActionCount !== hid.policy.acceptedActionCount ||
      broker.policy.releaseAllCount !== hid.policy.releaseAllCount ||
      broker.policy.complete !== true ||
      broker.firmwareAcceptedCount !==
        broker.policy.acceptedActionCount + broker.policy.releaseAllCount ||
      typeof broker.lastSequenceId !== 'string' || broker.lastSequenceId.length < 1 ||
      !/^[a-f0-9]{64}$/.test(broker.acceptedActionSha256 || '')) {
    throw new TypeError('independent broker policy evidence is incomplete or unbound');
  }
  const composeRaw = await readFile(composeGuardPath, 'utf8');
  const composeGuard = parseStrictJson(composeRaw, 'Compose guard receipt');
  if (composeGuard.kind !== 'compose-guard' || composeGuard.runId !== runId ||
      composeGuard.exactDeviceMappings !== 6 ||
      composeGuard.controllerImeBusAbsent !== true ||
      composeGuard.directTextActionForbidden !== true) {
    throw new TypeError('IME capability isolation is not proven by the Compose guard');
  }
  const { raw: seatRaw, evidence: seat } = await readSeatEvidence(seatEvidencePath, runId, {
    minimumKeyboardEvents: result.hidUsageCount * 2,
    minimumPointerEvents: 2,
  });
  const vector = vectorFor(result.locale);
  const expectedKeyboardTransitions = [...vector.usages, ...vector.commit]
    .flatMap((key) => {
      const code = evdevCodeForKey(key);
      return [{ code, value: 1 }, { code, value: 0 }];
    });
  if (canonicalJson(seat.keyboardTransitions) !==
      canonicalJson(expectedKeyboardTransitions) ||
      seat.sequenceComplete !== true ||
      seat.imePolicyFileSha256 !== `sha256:${sha256(policyRaw)}`) {
    throw new TypeError('seat keyboard transitions differ from the exact IME FSM');
  }
  const receipt = {
    ...receiptBase('run', runId, now),
    profileId: 'ime-composition-vusb',
    axis: 'ime-composition-vusb',
    browserEngine: result.browser,
    locale: result.locale,
    vectorId: result.vectorId,
    status: 'PASS',
    measurementVerdict: 'NOT_APPLICABLE',
    resultSha256: `sha256:${sha256(raw)}`,
    framebufferObservationSha256: `sha256:${sha256(observerRaw)}`,
    preeditFramebufferSha256: observer.preeditFramebufferSha256,
    commitFramebufferSha256: observer.commitFramebufferSha256,
    imePolicyFileSha256: `sha256:${sha256(policyRaw)}`,
    imePolicySha256: policy.policySha256,
    imeReadinessSha256: `sha256:${sha256(readinessRaw)}`,
    imeStopSha256: `sha256:${sha256(stopRaw)}`,
    hidReportEvidenceSha256: `sha256:${sha256(hidRaw)}`,
    brokerPolicyEvidenceSha256: `sha256:${sha256(brokerRaw)}`,
    composeGuardSha256: `sha256:${sha256(composeRaw)}`,
    seatEvidenceSha256: `sha256:${sha256(seatRaw)}`,
    directUnicodeCapabilityPresent: false,
    controllerImeBusPresent: false,
    usbOrigin: 'kernel-emulated',
    physicalUsb: false,
  };
  await atomicJson(destination, receipt);
  return receipt;
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  createImeRunReceipt({
    runId: required('HM_VUSB_RUN_ID'),
    resultPath: required('HM_VUSB_RESULT_PATH'),
    observerPath: required('HM_VUSB_IME_FRAMEBUFFER_OBSERVATION'),
    policyPath: required('HM_VUSB_IME_POLICY'),
    readinessPath: required('HM_VUSB_IME_READINESS'),
    stopPath: required('HM_VUSB_IME_STOP'),
    hidEvidencePath: required('HM_VUSB_HID_EVIDENCE'),
    brokerEvidencePath: required('HM_VUSB_BROKER_EVIDENCE'),
    composeGuardPath: required('HM_VUSB_COMPOSE_GUARD_RECEIPT'),
    seatEvidencePath: required('HM_VUSB_SEAT_EVIDENCE'),
    destination: required('HM_VUSB_RUN_RECEIPT'),
  }).catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-ime-run-receipt',
      code: 'PURITY_FAIL',
      message: error.message,
    })}\n`);
    process.exitCode = 1;
  });
}
