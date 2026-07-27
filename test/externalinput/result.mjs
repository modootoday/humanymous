import { SCHEMA_VERSION, STRATEGY_VERSION, TASK_SUITE_VERSION } from './contracts.mjs';
import { ContractViolationError } from './errors.mjs';

const VERDICTS = new Set(['ALLOW', 'CHALLENGE', 'DENY', 'NO_RESPONSE']);
const SHA256 = /^[a-f0-9]{64}$/;
const MEASUREMENT_FIELDS = new Set([
  'verdict', 'source', 'confidence', 'finalFrameSha256', 'cueCount',
]);

export function validateMeasurement(measurement) {
  if (!measurement || !VERDICTS.has(measurement.verdict)) {
    throw new ContractViolationError('measurement must contain a coarse framebuffer verdict');
  }
  for (const field of Object.keys(measurement)) {
    if (!MEASUREMENT_FIELDS.has(field)) {
      throw new ContractViolationError(`forbidden measurement field: ${field}`);
    }
  }
  if (measurement.source !== 'framebuffer') {
    throw new ContractViolationError('verdict source must be framebuffer');
  }
  if (!SHA256.test(measurement.finalFrameSha256 || '')) {
    throw new ContractViolationError('final framebuffer hash is invalid');
  }
  const confidence = Number(measurement.confidence);
  if (!Number.isFinite(confidence) || confidence < 0 || confidence > 1) {
    throw new ContractViolationError('measurement confidence is invalid');
  }
  if (measurement.cueCount < 2 && measurement.verdict !== 'NO_RESPONSE') {
    throw new ContractViolationError('a classified verdict requires two framebuffer cues');
  }
  return Object.freeze({
    verdict: measurement.verdict,
    source: measurement.source,
    confidence,
    finalFrameSha256: measurement.finalFrameSha256,
    cueCount: measurement.cueCount,
  });
}

export function unavailableResult({ runId, engine, mode, reason, capability }) {
  return Object.freeze({
    schemaVersion: SCHEMA_VERSION,
    runId,
    sequence: mode.sequence,
    profileId: mode.profileId,
    groundTruth: 'automation',
    browser: { engine },
    status: 'UNAVAILABLE',
    availability: { capability, reason },
  });
}

export function completedResult({
  runId,
  engine,
  mode,
  seed,
  browser,
  strategy,
  observation,
  input,
  purity,
  measurement,
  classificationFrames = 1,
}) {
  const measured = validateMeasurement(measurement);
  const failed = !strategy.completed || measured.verdict === 'NO_RESPONSE';
  const status = failed ? 'FAIL' : measured.verdict === 'ALLOW' ? 'RESIDUAL' : 'PASS';
  const usb = input.attestation;
  return Object.freeze({
    schemaVersion: SCHEMA_VERSION,
    runId,
    sequence: mode.sequence,
    profileId: mode.profileId,
    groundTruth: 'automation',
    browser: {
      engine,
      version: browser.version,
      binarySha256: browser.binarySha256,
      sandbox: browser.sandbox === true,
    },
    control: {
      observation: mode.observation,
      inputBackend: mode.inputBackend,
      seed: String(seed),
      frames: strategy.frames + classificationFrames,
      events: input.events,
      elapsedMs: strategy.elapsedMs,
    },
    strategy: {
      version: strategy.version || STRATEGY_VERSION,
      taskSuiteVersion: strategy.taskSuiteVersion || TASK_SUITE_VERSION,
      decisions: strategy.decisions,
      recoveries: strategy.recoveries,
      modalityChanges: strategy.modalityChanges,
      visibleChallenges: strategy.visibleChallenges,
    },
    dom: {
      enabled: observation.domEnabled,
      ...(observation.domManifest || {}),
      queries: strategy.domQueries,
      mutationSurface: false,
    },
    usb: mode.inputBackend === 'usb-hid-emulated'
      ? {
          required: true,
          ...(usb || {}),
        }
      : {
          required: mode.usbRequired,
          attested: Boolean(usb),
          ...(usb || {}),
        },
    purity: { ...purity },
    tasks: strategy.tasks,
    measurement: measured,
    status,
  });
}
