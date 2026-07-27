import { createHash } from 'node:crypto';
import { modeFor } from './contracts.mjs';

export const BROWSER_PROBE_SCHEMA = 'humanymous.external-input-browser-probe/v1';
export const DISPLAY_PROBE_SCHEMA = 'humanymous.external-input-display-probe/v1';
export const PREPROJECT_PROOF_SCHEMA = 'humanymous.external-input-preproject-proof/v1';
export const RESET_PROOF_SCHEMA = 'humanymous.external-input-reset-proof/v1';
export const RUNTIME_EVIDENCE_SCHEMA = 'humanymous.external-input-runtime-evidence/v1';

const SHA256 = /^[a-f0-9]{64}$/;
const FORBIDDEN_ARGV = /remote-debugging|enable-automation|marionette|webdriver|ignore-certificate-errors|disable-web-security|no-sandbox/i;
const AUTOMATION_PACKAGE = /playwright|puppeteer|selenium|chromedriver|geckodriver/i;
const PROCESS_INJECTION_ENVIRONMENT = Object.freeze([
  'PATH',
  'NODE_OPTIONS',
  'NODE_PATH',
  'LD_PRELOAD',
  'LD_LIBRARY_PATH',
  'BASH_ENV',
  'ENV',
  'SHELLOPTS',
]);

function exactObject(value, fields, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  const expected = new Set(fields);
  for (const field of Object.keys(value)) {
    if (!expected.has(field)) throw new TypeError(`${label} has unknown field: ${field}`);
  }
  for (const field of expected) {
    if (!Object.hasOwn(value, field)) throw new TypeError(`${label} is missing field: ${field}`);
  }
  return value;
}

function boundedString(value, label, maximum = 4096) {
  if (typeof value !== 'string' || value.length > maximum) {
    throw new TypeError(`${label} is not a bounded string`);
  }
  return value;
}

function sha256(value) {
  return `sha256:${createHash('sha256').update(value).digest('hex')}`;
}

function sourceTarget(value) {
  if (typeof value === 'string') {
    const [source, target] = value.split(':');
    return { source, target };
  }
  return { source: value?.source, target: value?.target };
}

function mountShadowsPath(mount, protectedPath) {
  const target = String(sourceTarget(mount).target || '').replace(/\/+$/, '') || '/';
  return target === protectedPath ||
    target === '/' ||
    protectedPath.startsWith(`${target}/`);
}

function exactNetwork(service, expected) {
  return Object.keys(service?.networks || {}).length === 1 &&
    Object.hasOwn(service.networks, expected);
}

function hasForbiddenMount(service) {
  return (service?.volumes || []).some((mount) => {
    const { source, target } = sourceTarget(mount);
    return source === '/var/run/docker.sock' ||
      target === '/var/run/docker.sock' ||
      source === '/tmp/.X11-unix' ||
      source === '/dev/uinput' ||
      target === '/dev/uinput';
  });
}

function hasUinputMapping(service) {
  return (service?.devices || []).some((mapping) => {
    const { source, target } = sourceTarget(mapping);
    return source === '/dev/uinput' || target === '/dev/uinput';
  });
}

function domMountPresent(service) {
  return (service?.volumes || []).some((mount) =>
    sourceTarget(mount).target === '/run/dom-observer');
}

function isHardenedEvidenceService(service) {
  return service?.network_mode === 'none' &&
    service.read_only === true &&
    service.restart === 'no' &&
    /^[1-9][0-9]*(?::[1-9][0-9]*)?$/.test(String(service.user || '')) &&
    Array.isArray(service.cap_drop) &&
    service.cap_drop.length === 1 &&
    service.cap_drop[0] === 'ALL' &&
    (!Array.isArray(service.cap_add) || service.cap_add.length === 0) &&
    Array.isArray(service.security_opt) &&
    service.security_opt.includes('no-new-privileges:true') &&
    (!Array.isArray(service.devices) || service.devices.length === 0);
}

export function argvHasForbiddenCapability(argv) {
  return !Array.isArray(argv) || argv.some((value) => FORBIDDEN_ARGV.test(String(value)));
}

export function packageInventoryHasAutomationDependency(packages) {
  return AUTOMATION_PACKAGE.test(String(packages));
}

export function argvHasProcessType(argv, expectedType) {
  if (!Array.isArray(argv) || !/^[a-z][a-z-]*$/.test(expectedType || '')) {
    return false;
  }
  const expected = `--type=${expectedType}`;
  return argv.some((entry) =>
    String(entry).split(/\s+/).some((token) => token === expected));
}

export function validateImmutableServiceProcess(service, {
  name,
  entrypoint,
  command = null,
}) {
  if (!service || typeof service !== 'object' ||
      !Array.isArray(entrypoint) && entrypoint !== null) {
    throw new TypeError('immutable service process contract is invalid');
  }
  if (JSON.stringify(service.entrypoint ?? null) !== JSON.stringify(entrypoint)) {
    throw new TypeError(`${name} entrypoint differs`);
  }
  if (JSON.stringify(service.command ?? null) !== JSON.stringify(command)) {
    throw new TypeError(`${name} command differs`);
  }
  const environment = service.environment || {};
  for (const key of PROCESS_INJECTION_ENVIRONMENT) {
    if (Object.hasOwn(environment, key)) {
      throw new TypeError(`${name} has process-injection environment: ${key}`);
    }
  }
  if ((Array.isArray(service.post_start) && service.post_start.length > 0) ||
      (Array.isArray(service.pre_stop) && service.pre_stop.length > 0)) {
    throw new TypeError(`${name} lifecycle hooks are forbidden`);
  }
  const protectedPaths = (entrypoint || [])
    .map(String)
    .filter((value) => value.startsWith('/') && /\.(?:mjs|js|sh)$/.test(value));
  if ((service.volumes || []).some((mount) =>
    protectedPaths.some((protectedPath) => mountShadowsPath(mount, protectedPath)))) {
    throw new TypeError(`${name} mounts over a protected executable`);
  }
  return true;
}

export function parseListeningSockets(raw) {
  boundedString(raw, 'socket table', 256 * 1024);
  const listeners = [];
  for (const line of raw.trim().split(/\r?\n/).slice(1, 4097)) {
    const fields = line.trim().split(/\s+/);
    if (fields.length < 4 || fields[3] !== '0A') continue;
    const address = fields[1] || '';
    if (address.startsWith('0B00007F:')) continue;
    listeners.push(address);
  }
  return Object.freeze(listeners);
}

export function sandboxStatusIsBounded(status, engine = 'chromium') {
  boundedString(status, 'sandbox status', 64 * 1024);
  const common = /^Seccomp:\s*2$/m.test(status) &&
    /^NoNewPrivs:\s*1$/m.test(status) &&
    /^CapEff:\s*0+$/m.test(status);
  if (!common) return false;
  if (engine === 'chromium') {
    return /^NSpid:\s*\d+\s+\d+\s+1$/m.test(status);
  }
  if (engine === 'firefox') {
    const filters = /^Seccomp_filters:\s*(\d+)$/m.exec(status);
    return Boolean(filters) && Number(filters[1]) >= 3 &&
      /^NSpid:\s*\d+$/m.test(status);
  }
  throw new TypeError('browser engine is invalid');
}

export function validateBrowserProbe(value, expected = {}) {
  exactObject(value, [
    'schemaVersion', 'runId', 'profileId', 'engine', 'version', 'binary',
    'binarySha256', 'argv', 'forbiddenArgv', 'automationDependency',
    'debugPortListening', 'sandboxed', 'sandboxStatus', 'domObserverPresent',
    'domImagePresent', 'domSocketPresent', 'domExtensionSha256',
    'domManifestSha256', 'uinputPresent',
  ], 'browser runtime probe');
  if (value.schemaVersion !== BROWSER_PROBE_SCHEMA ||
      value.runId !== expected.runId || value.profileId !== expected.profileId ||
      value.engine !== expected.engine ||
      !['chromium', 'firefox'].includes(value.engine) ||
      !SHA256.test(value.binarySha256 || '') ||
      !Array.isArray(value.argv) || value.argv.length < 1 || value.argv.length > 128 ||
      value.argv.some((entry) => typeof entry !== 'string' || entry.length > 2048) ||
      !Array.isArray(value.sandboxStatus) || value.sandboxStatus.length > 16 ||
      value.sandboxStatus.some((entry) => typeof entry !== 'string' || entry.length > 256)) {
    throw new TypeError('browser runtime probe identity or bounds are invalid');
  }
  for (const field of [
    'forbiddenArgv', 'automationDependency', 'debugPortListening', 'sandboxed',
    'domObserverPresent', 'domImagePresent', 'domSocketPresent', 'uinputPresent',
  ]) {
    if (typeof value[field] !== 'boolean') throw new TypeError(`${field} must be boolean`);
  }
  boundedString(value.version, 'browser version', 256);
  boundedString(value.binary, 'browser binary', 256);
  for (const field of ['domExtensionSha256', 'domManifestSha256']) {
    if (value[field] !== '' && !SHA256.test(value[field])) {
      throw new TypeError(`${field} is invalid`);
    }
  }
  if (value.forbiddenArgv !== argvHasForbiddenCapability(value.argv)) {
    throw new TypeError('browser argv classification is inconsistent');
  }
  return Object.freeze(structuredClone(value));
}

export function validateDisplayProbe(value, expected = {}) {
  exactObject(value, [
    'schemaVersion', 'runId', 'profileId', 'xtestEnabled', 'uinputPresent',
  ], 'display runtime probe');
  if (value.schemaVersion !== DISPLAY_PROBE_SCHEMA ||
      value.runId !== expected.runId || value.profileId !== expected.profileId ||
      typeof value.xtestEnabled !== 'boolean' || typeof value.uinputPresent !== 'boolean') {
    throw new TypeError('display runtime probe identity or fields are invalid');
  }
  return Object.freeze(structuredClone(value));
}

export function validateRuntimeEvidence(value, expected = {}) {
  exactObject(value, [
    'schemaVersion', 'runId', 'profileId', 'composeConfigSha256',
    'browser', 'display', 'dom',
  ], 'runtime evidence');
  exactObject(value.browser, [
    'engine', 'version', 'binary', 'binarySha256', 'sandbox',
  ], 'runtime browser evidence');
  exactObject(value.display, ['xtestEnabled'], 'runtime display evidence');
  exactObject(value.dom, [
    'present', 'extensionSha256', 'manifestSha256',
  ], 'runtime DOM evidence');
  if (value.schemaVersion !== RUNTIME_EVIDENCE_SCHEMA ||
      value.runId !== expected.runId || value.profileId !== expected.profileId ||
      !/^sha256:[a-f0-9]{64}$/.test(value.composeConfigSha256 || '') ||
      !['chromium', 'firefox'].includes(value.browser.engine) ||
      (expected.engine && value.browser.engine !== expected.engine) ||
      !SHA256.test(value.browser.binarySha256 || '') ||
      value.browser.sandbox !== true ||
      typeof value.display.xtestEnabled !== 'boolean' ||
      typeof value.dom.present !== 'boolean') {
    throw new TypeError('runtime evidence identity or proof is invalid');
  }
  boundedString(value.browser.version, 'runtime browser version', 256);
  boundedString(value.browser.binary, 'runtime browser binary', 256);
  for (const field of ['extensionSha256', 'manifestSha256']) {
    if (value.dom[field] !== '' && !SHA256.test(value.dom[field])) {
      throw new TypeError(`runtime DOM ${field} is invalid`);
    }
  }
  if (expected.profileId) {
    const mode = modeFor(expected.profileId);
    if (value.dom.present !== mode.domRequired ||
        value.display.xtestEnabled !== !mode.usbRequired ||
        (mode.domRequired &&
          (!SHA256.test(value.dom.extensionSha256) ||
           !SHA256.test(value.dom.manifestSha256))) ||
        (!mode.domRequired &&
          (value.dom.extensionSha256 !== '' || value.dom.manifestSha256 !== ''))) {
      throw new TypeError('runtime evidence differs from the canonical mode');
    }
  }
  return Object.freeze(structuredClone(value));
}

export function createPreProjectProof({
  runId,
  profileId,
  composeProject,
  composeRaw,
  containersRaw,
  networksRaw,
  volumesRaw,
}) {
  modeFor(profileId);
  if (!/^[a-z0-9][a-z0-9_-]{0,62}$/.test(composeProject || '')) {
    throw new TypeError('Compose project is invalid');
  }
  for (const [label, raw] of Object.entries({
    containers: containersRaw,
    networks: networksRaw,
    volumes: volumesRaw,
  })) {
    boundedString(raw, `pre-project ${label}`, 64 * 1024);
    if (raw.trim() !== '') {
      throw new TypeError(`pre-project ${label} inventory is not empty`);
    }
  }
  boundedString(composeRaw, 'Compose config', 512 * 1024);
  return validatePreProjectProof({
    schemaVersion: PREPROJECT_PROOF_SCHEMA,
    runId,
    profileId,
    composeProject,
    composeConfigSha256: sha256(composeRaw),
    inventorySha256: {
      containers: sha256(containersRaw),
      networks: sha256(networksRaw),
      volumes: sha256(volumesRaw),
    },
  }, { runId, profileId, composeProject });
}

export function validatePreProjectProof(value, expected = {}) {
  exactObject(value, [
    'schemaVersion', 'runId', 'profileId', 'composeProject',
    'composeConfigSha256', 'inventorySha256',
  ], 'pre-project proof');
  exactObject(value.inventorySha256, [
    'containers', 'networks', 'volumes',
  ], 'pre-project inventory hashes');
  if (value.schemaVersion !== PREPROJECT_PROOF_SCHEMA ||
      value.runId !== expected.runId ||
      value.profileId !== expected.profileId ||
      value.composeProject !== expected.composeProject ||
      !/^sha256:[a-f0-9]{64}$/.test(value.composeConfigSha256 || '') ||
      Object.values(value.inventorySha256)
        .some((digest) => !/^sha256:[a-f0-9]{64}$/.test(digest || ''))) {
    throw new TypeError('pre-project proof identity or hashes are invalid');
  }
  return Object.freeze(structuredClone(value));
}

export function validateResetProof(value, expected = {}) {
  exactObject(value, [
    'schemaVersion', 'runId', 'profileId', 'composeProject',
    'freshBrowserProfile', 'freshDetectorState', 'freshTaskState',
    'composeConfigSha256', 'preProjectInventorySha256',
    'visualManifestSha256',
  ], 'reset proof');
  exactObject(value.preProjectInventorySha256, [
    'containers', 'networks', 'volumes',
  ], 'reset proof inventory hashes');
  if (value.schemaVersion !== RESET_PROOF_SCHEMA ||
      value.runId !== expected.runId ||
      value.profileId !== expected.profileId ||
      value.composeProject !== expected.composeProject ||
      value.freshBrowserProfile !== true ||
      value.freshDetectorState !== true ||
      value.freshTaskState !== true ||
      !/^sha256:[a-f0-9]{64}$/.test(value.composeConfigSha256 || '') ||
      !/^sha256:[a-f0-9]{64}$/.test(value.visualManifestSha256 || '') ||
      Object.values(value.preProjectInventorySha256)
        .some((digest) => !/^sha256:[a-f0-9]{64}$/.test(digest || ''))) {
    throw new TypeError('reset proof identity or hashes are invalid');
  }
  return Object.freeze(structuredClone(value));
}

export function evaluateRuntimePurity({
  config,
  browserProbe,
  displayProbe,
  profileId,
  runId,
  engine,
  expectedDom,
  composeRaw,
}) {
  const mode = modeFor(profileId);
  const browserEvidence = validateBrowserProbe(browserProbe, { runId, profileId, engine });
  const displayEvidence = validateDisplayProbe(displayProbe, { runId, profileId });
  if (!config?.services || typeof config.services !== 'object') {
    throw new TypeError('Compose config has no services');
  }
  const services = config.services;
  const browser = services['external-browser'];
  const controller = services['external-controller'];
  const display = services['external-display'];
  const browserRuntimeProbe = services['external-runtime-browser-probe'];
  const displayRuntimeProbe = services['external-runtime-display-probe'];
  const runtimeEvaluator = services['external-runtime-purity-evaluator'];
  const scoreTraceEvaluator = services['external-score-trace-evaluator'];
  if (!browser || !controller || !display) {
    throw new TypeError('external-input services are incomplete');
  }
  if (!exactNetwork(browser, 'external-target') ||
      !exactNetwork(controller, 'external-control') ||
      !exactNetwork(display, 'external-control')) {
    throw new TypeError('external-input network separation is invalid');
  }
  if (![browserRuntimeProbe, displayRuntimeProbe, runtimeEvaluator, scoreTraceEvaluator]
    .every(isHardenedEvidenceService) ||
      browserRuntimeProbe.pid !== 'service:external-browser' ||
      browserRuntimeProbe.image !== browser.image ||
      displayRuntimeProbe.image !== display.image ||
      runtimeEvaluator.image !== controller.image ||
      scoreTraceEvaluator.image !== controller.image) {
    throw new TypeError('runtime evidence services are not isolated and image-bound');
  }
  for (const [name, entrypoint] of Object.entries({
    'external-runtime-browser-probe': [
      'node', '/opt/external-input/runtime-browser-probe.mjs',
    ],
    'external-runtime-display-probe': [
      'node', '/opt/external-input/runtime-display-probe.mjs',
    ],
    'external-runtime-purity-evaluator': [
      'node', '/app/test/externalinput/runtime-purity-evaluator.mjs',
    ],
    'external-score-trace-evaluator': [
      'node', '/app/test/externalinput/score-trace-evaluator.mjs',
    ],
  })) {
    validateImmutableServiceProcess(services[name], { name, entrypoint });
  }
  if (Object.entries(services).some(([name, service]) =>
    service?.privileged === true &&
      !['external-vusb-init', 'external-vusb-cleanup'].includes(name))) {
    throw new TypeError('unexpected privileged service is present');
  }
  if ([
    browser, controller, display,
    browserRuntimeProbe, displayRuntimeProbe, runtimeEvaluator, scoreTraceEvaluator,
  ].some(hasForbiddenMount)) {
    throw new TypeError('forbidden Docker socket, host display, or uinput mount is present');
  }
  const domMounted = domMountPresent(browser);
  const expectedBrowserTarget = `browser-${engine}${mode.domRequired ? '-dom' : ''}`;
  if (browser.build?.target !== expectedBrowserTarget ||
      domMounted !== mode.domRequired ||
      domMountPresent(browserRuntimeProbe) !== mode.domRequired ||
      browserEvidence.domObserverPresent !== mode.domRequired ||
      browserEvidence.domImagePresent !== mode.domRequired ||
      browserEvidence.domSocketPresent !== mode.domRequired) {
    throw new TypeError('DOM observer image/mount does not match the selected mode');
  }
  if (mode.domRequired) {
    if (!expectedDom ||
        browserEvidence.domExtensionSha256 !== expectedDom.extensionSha256 ||
        browserEvidence.domManifestSha256 !== expectedDom.manifestSha256) {
      throw new TypeError('DOM observer hashes do not match the evaluator image');
    }
  } else if (browserEvidence.domExtensionSha256 || browserEvidence.domManifestSha256) {
    throw new TypeError('framebuffer-only mode retained DOM observer hashes');
  }
  const hasDevices = Object.values(services).some((service) =>
    Array.isArray(service?.devices) && service.devices.length > 0);
  const uinputMapped = Object.values(services).some((service) =>
    hasForbiddenMount(service) || hasUinputMapping(service));
  if (hasDevices !== mode.usbRequired ||
      displayEvidence.xtestEnabled !== !mode.usbRequired) {
    throw new TypeError('input backend assignment does not match the selected mode');
  }
  if (browserEvidence.forbiddenArgv || browserEvidence.automationDependency ||
      browserEvidence.debugPortListening || !browserEvidence.sandboxed) {
    throw new TypeError('stock-browser runtime proof is incomplete');
  }
  if (browserEvidence.uinputPresent || displayEvidence.uinputPresent || uinputMapped) {
    throw new TypeError('uinput purity proof failed');
  }
  if (!config.networks?.['external-target']?.internal ||
      !config.networks?.['external-control']?.internal) {
    throw new TypeError('external-input lab networks must be internal');
  }
  const purity = {
    forbiddenArgv: false,
    debugPortListening: false,
    automationDependency: false,
    controllerHasLabNetwork: false,
    hostDisplayMounted: false,
    domMutationAttempt: false,
    mixedInputBackends: false,
    uinputPresent: false,
    xtestEnabled: displayEvidence.xtestEnabled,
    usbAssigned: mode.usbRequired,
    browserAutomationPortAbsent: true,
    controllerNetworkIsolated: true,
    browserLabOnly: true,
    domObserverPresent: mode.domRequired,
    ...(mode.domRequired ? { domObserverHashPinned: true } : {}),
  };
  const runtimeEvidence = validateRuntimeEvidence({
    schemaVersion: RUNTIME_EVIDENCE_SCHEMA,
    runId,
    profileId,
    composeConfigSha256: `sha256:${createHash('sha256').update(composeRaw).digest('hex')}`,
    browser: {
      engine,
      version: browserEvidence.version,
      binary: browserEvidence.binary,
      binarySha256: browserEvidence.binarySha256,
      sandbox: true,
    },
    display: { xtestEnabled: displayEvidence.xtestEnabled },
    dom: {
      present: mode.domRequired,
      extensionSha256: mode.domRequired ? browserEvidence.domExtensionSha256 : '',
      manifestSha256: mode.domRequired ? browserEvidence.domManifestSha256 : '',
    },
  }, { runId, profileId, engine });
  return Object.freeze({ purity: Object.freeze(purity), runtimeEvidence });
}
