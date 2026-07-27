import { randomUUID } from 'node:crypto';
import {
  CANONICAL_MODES,
  EXECUTION_KIND,
  assertAdapterPair,
  assertCanonicalSequence,
  assertEngine,
  modeFor,
} from './contracts.mjs';
import { CapabilityUnavailableError, ContractViolationError } from './errors.mjs';
import { completedResult, unavailableResult } from './result.mjs';
import { runStrategicPolicy } from './strategy.mjs';

const REQUIRED_PURITY_FALSE = [
  'forbiddenArgv',
  'debugPortListening',
  'automationDependency',
  'controllerHasLabNetwork',
  'hostDisplayMounted',
  'domMutationAttempt',
  'mixedInputBackends',
  'uinputPresent',
];
const PURITY_FIELDS = new Set([
  ...REQUIRED_PURITY_FALSE,
  'xtestEnabled',
  'usbAssigned',
  'browserAutomationPortAbsent',
  'controllerNetworkIsolated',
  'browserLabOnly',
  'domObserverPresent',
  'domObserverHashPinned',
]);

function assertContext(context) {
  if (!context || context.executionKind !== EXECUTION_KIND) {
    throw new ContractViolationError(
      `external-input profiles require executionKind=${EXECUTION_KIND}`,
    );
  }
  for (const method of ['reset', 'createAdapters', 'inspectPurity', 'classifyFramebuffer']) {
    if (typeof context[method] !== 'function') {
      throw new ContractViolationError(`runner context requires ${method}()`);
    }
  }
}

function assertBrowser(browser) {
  if (!browser || typeof browser.version !== 'string' ||
      !/^[a-f0-9]{64}$/.test(browser.binarySha256 || '') || browser.sandbox !== true) {
    throw new ContractViolationError('stock browser version/hash/sandbox proof is incomplete');
  }
  return browser;
}

function assertPurity(mode, purity) {
  if (!purity || typeof purity !== 'object') throw new ContractViolationError('purity proof is missing');
  for (const field of Object.keys(purity)) {
    if (!PURITY_FIELDS.has(field)) throw new ContractViolationError(`unknown purity field: ${field}`);
  }
  for (const field of REQUIRED_PURITY_FALSE) {
    if (purity[field] !== false) throw new ContractViolationError(`purity check failed: ${field}`);
  }
  if (mode.usbRequired && purity.xtestEnabled !== false) {
    throw new ContractViolationError('USB mode must prove XTEST is disabled');
  }
  if (mode.usbRequired && purity.usbAssigned !== true) {
    throw new ContractViolationError('USB mode must prove exclusive USB assignment');
  }
  if (mode.inputBackend === 'rfb-xtest' &&
      (purity.usbAssigned !== false || purity.xtestEnabled !== true)) {
    throw new ContractViolationError('virtual mode must prove USB absence and XTEST presence');
  }
  if (mode.domRequired &&
      (purity.domObserverPresent !== true || purity.domObserverHashPinned !== true)) {
    throw new ContractViolationError('DOM mode must prove a pinned observer is present');
  }
  if (!mode.domRequired && purity.domObserverPresent !== false) {
    throw new ContractViolationError('framebuffer-only mode must prove DOM observer absence');
  }
  if (purity.browserAutomationPortAbsent !== true ||
      purity.controllerNetworkIsolated !== true ||
      purity.browserLabOnly !== true) {
    throw new ContractViolationError('network/automation purity proof is incomplete');
  }
  return Object.freeze(Object.fromEntries([...PURITY_FIELDS].map((field) => [field, purity[field]])));
}

export async function runExternalProfile(profileId, context) {
  assertContext(context);
  const mode = modeFor(profileId);
  const engine = assertEngine(context.engine);
  const runId = context.runId || randomUUID();
  const seed = context.seed || `${engine}-${mode.sequence}`;
  let input;
  let cleanup;
  let interactionStarted = false;

  try {
    const resetProof = await context.reset({ engine, mode, seed });
    if (!resetProof || resetProof.freshBrowserProfile !== true ||
        resetProof.freshDetectorState !== true || resetProof.freshTaskState !== true) {
      throw new ContractViolationError('mode reset proof is incomplete');
    }
    const adapters = await context.createAdapters({ engine, mode, seed });
    assertAdapterPair(mode, adapters?.observation, adapters?.input);
    input = adapters.input;
    cleanup = adapters.cleanup;
    const browser = assertBrowser(adapters.browser);
    const purity = assertPurity(mode, await context.inspectPurity({ engine, mode, adapters }));
    interactionStarted = true;
    const strategy = await (context.runStrategy || runStrategicPolicy)({
      observation: adapters.observation,
      input,
      seed,
      strategyVariant: context.strategyVariant || 'mixed-input',
    });
    const rawMeasurement = await context.classifyFramebuffer({ engine, mode, adapters, strategy });
    const classificationFrames = rawMeasurement?.frameCount ?? 1;
    if (!Number.isInteger(classificationFrames) ||
        classificationFrames < 1 || classificationFrames > 300) {
      throw new ContractViolationError('framebuffer classification frame count is invalid');
    }
    const { frameCount: _frameCount, ...measurement } = rawMeasurement;
    return completedResult({
      runId,
      engine,
      mode,
      seed,
      browser,
      strategy,
      observation: adapters.observation,
      input,
      purity,
      measurement,
      classificationFrames,
    });
  } catch (error) {
    if (error instanceof CapabilityUnavailableError && !interactionStarted) {
      return unavailableResult({
        runId,
        engine,
        mode,
        capability: error.capability,
        reason: error.message,
      });
    }
    if (error instanceof CapabilityUnavailableError) {
      throw new ContractViolationError(
        `required capability was lost during ${mode.profileId}: ${error.message}`,
      );
    }
    throw error;
  } finally {
    try {
      if (input) await input.releaseAll();
    } finally {
      if (typeof cleanup === 'function') await cleanup();
    }
  }
}

export async function runCanonicalLadder(context) {
  assertContext(context);
  const results = [];
  for (const mode of CANONICAL_MODES) {
    const result = await runExternalProfile(mode.profileId, {
      ...context,
      runId: `${context.runId || randomUUID()}-${mode.sequence}`,
      seed: context.seedForMode ? context.seedForMode(mode) : context.seed,
    });
    results.push(result);
  }
  assertCanonicalSequence(results);
  return Object.freeze(results);
}
