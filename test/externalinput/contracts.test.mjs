import assert from 'node:assert/strict';
import test from 'node:test';
import {
  CANONICAL_MODES,
  EXECUTION_KIND,
  assertCanonicalSequence,
} from './contracts.mjs';
import { CapabilityUnavailableError, ContractViolationError } from './errors.mjs';
import { createActionFirewall } from './firewall.mjs';
import { createUsbInputAdapter, createVirtualInputAdapter } from './input.mjs';
import {
  createDomFramebufferObservation,
  createFramebufferObservation,
} from './observation.mjs';
import { runCanonicalLadder, runExternalProfile } from './runner.mjs';
import { validateMeasurement } from './result.mjs';

const HASH = 'a'.repeat(64);
const BROWSER_HASH = 'b'.repeat(64);

function allTargets(targetTokens) {
  return targetTokens.map((token, index) => ({
    token,
    rect: { x: 20 + index * 40, y: 40, width: 30, height: 20 },
    enabled: true,
    visible: true,
  }));
}

function fakeObservation(mode) {
  const common = {
    readFrame: async () => ({ width: 1280, height: 720, sha256: HASH, cues: [] }),
    locateVisualTargets: async ({ targetTokens }) => allTargets(targetTokens),
  };
  return mode.domRequired
    ? createDomFramebufferObservation({
        ...common,
        domManifest: {
          extensionSha256: 'c'.repeat(64),
          manifestSha256: 'd'.repeat(64),
        },
        queryDom: async (request) => request.method === 'findByTextToken'
          ? { rect: { x: 60, y: 60, width: 30, height: 20 }, enabled: true, visible: true }
          : null,
      })
    : createFramebufferObservation(common);
}

function completePurity(mode) {
  return {
    forbiddenArgv: false,
    debugPortListening: false,
    automationDependency: false,
    controllerHasLabNetwork: false,
    hostDisplayMounted: false,
    domMutationAttempt: false,
    mixedInputBackends: false,
    uinputPresent: false,
    xtestEnabled: mode.usbRequired ? false : true,
    usbAssigned: mode.usbRequired,
    domObserverPresent: mode.domRequired,
    domObserverHashPinned: mode.domRequired,
    browserAutomationPortAbsent: true,
    controllerNetworkIsolated: true,
    browserLabOnly: true,
  };
}

function usbAttestation() {
  return {
    vid: '1209',
    pid: '0001',
    serialSha256: '1'.repeat(64),
    descriptorSha256: '2'.repeat(64),
    topologySha256: '3'.repeat(64),
    firmwareSha256: '4'.repeat(64),
    dedicatedSeat: true,
    seatEventObserved: true,
    physicalUsb: true,
    uinputPresent: false,
    interfaceSet: 'command+keyboard+pointer',
    exclusiveAssignment: true,
    emergencyStopReady: true,
    deadManReleaseMs: 750,
  };
}

function context({ usbAvailable = true } = {}) {
  const sent = [];
  let releases = 0;
  return {
    sent,
    get releases() { return releases; },
    executionKind: EXECUTION_KIND,
    engine: 'chromium',
    runId: 'contract-test',
    seed: 'paired-seed',
    reset: async () => ({
      freshBrowserProfile: true,
      freshDetectorState: true,
      freshTaskState: true,
    }),
    createAdapters: async ({ mode }) => {
      if (mode.usbRequired && !usbAvailable) {
        throw new CapabilityUnavailableError('usb-hid', 'test USB is absent');
      }
      const firewall = createActionFirewall();
      const base = {
        firewall,
        sleep: async () => {},
        send: async (action) => sent.push([mode.profileId, action]),
        release: async () => { releases += 1; },
      };
      const input = mode.usbRequired
        ? createUsbInputAdapter({
            ...base,
            usbAttestation: usbAttestation(),
            rfbInputEnabled: false,
            xtestEnabled: false,
          })
        : createVirtualInputAdapter(base);
      return {
        observation: fakeObservation(mode),
        input,
        browser: { version: 'test-browser', binarySha256: BROWSER_HASH, sandbox: true },
      };
    },
    inspectPurity: async ({ mode }) => completePurity(mode),
    classifyFramebuffer: async () => ({
      verdict: 'CHALLENGE',
      source: 'framebuffer',
      confidence: 1,
      cueCount: 2,
      finalFrameSha256: HASH,
    }),
  };
}

test('canonical ladder is the exact framebuffer/DOM by virtual/USB 2x2 sequence', () => {
  assert.deepEqual(
    CANONICAL_MODES.map(({ sequence, profileId, observation, inputBackend }) => ({
      sequence, profileId, observation, inputBackend,
    })),
    [
      { sequence: 1, profileId: 'external_input_virtual', observation: 'framebuffer', inputBackend: 'rfb-xtest' },
      { sequence: 2, profileId: 'external_input_dom_virtual', observation: 'dom+framebuffer', inputBackend: 'rfb-xtest' },
      { sequence: 3, profileId: 'external_input_usb', observation: 'framebuffer', inputBackend: 'usb-hid' },
      { sequence: 4, profileId: 'external_input_dom_usb', observation: 'dom+framebuffer', inputBackend: 'usb-hid' },
    ],
  );
  assert.throws(
    () => assertCanonicalSequence([...CANONICAL_MODES].reverse()),
    /must be external_input_virtual/,
  );
});

test('action firewall rejects browser/OS shortcuts and unbounded/free-form actions', () => {
  const firewall = createActionFirewall();
  assert.throws(
    () => firewall.validate({ kind: 'keyStroke', key: 'L', modifiers: ['Control'] }),
    /shortcut modifiers/,
  );
  assert.throws(
    () => firewall.validate({ kind: 'navigate', url: 'https://example.test' }),
    /forbidden action kind/,
  );
  assert.throws(
    () => firewall.validate({ kind: 'typeText', text: 'operator secret' }),
    /fixed synthetic/,
  );
  assert.throws(
    () => firewall.validate({ kind: 'pointerMove', x: 9000, y: 1, durationMs: 1 }),
    /bounds/,
  );
  assert.throws(
    () => firewall.validate({ kind: 'pointerClick', button: 'middle' }),
    /not allowlisted/,
  );
});

test('DOM observation is absent in modes 1/3 and constrained to typed reads in modes 2/4', async () => {
  const visual = fakeObservation(CANONICAL_MODES[0]);
  await assert.rejects(
    () => visual.query({ method: 'findByTextToken', token: 'choice-correct' }),
    /DOM observer is absent/,
  );
  const dom = fakeObservation(CANONICAL_MODES[1]);
  const state = await dom.observe({ taskId: 'read-select', targetTokens: ['choice-correct'] });
  assert.equal(state.domQueries, 1);
  assert.equal(state.targets.some((target) => target.source === 'dom'), true);
  await assert.rejects(
    () => dom.query({ method: 'click', nodeId: 'n1' }),
    /not allowlisted/,
  );
  await assert.rejects(
    () => dom.query({ method: 'snapshot', selector: '*' }),
    /forbidden DOM request field/,
  );
  const malicious = createDomFramebufferObservation({
    readFrame: async () => ({ width: 1280, height: 720, sha256: HASH }),
    locateVisualTargets: async () => [],
    domManifest: {
      extensionSha256: 'c'.repeat(64),
      manifestSha256: 'd'.repeat(64),
    },
    queryDom: async () => ({ hiddenValue: 'secret', visible: true }),
  });
  await assert.rejects(
    () => malicious.query({ method: 'visibleState', nodeId: 'opaque-1' }),
    /forbidden field/,
  );
});

test('USB input fails closed without physical identity, firmware, and dedicated-seat proof', () => {
  const firewall = createActionFirewall();
  const base = { firewall, send: async () => {}, release: async () => {} };
  assert.throws(
    () => createUsbInputAdapter({ ...base }),
    CapabilityUnavailableError,
  );
  assert.throws(
    () => createUsbInputAdapter({
      ...base,
      usbAttestation: { ...usbAttestation(), physicalUsb: false },
    }),
    /virtual input cannot satisfy/,
  );
  assert.throws(
    () => createUsbInputAdapter({
      ...base,
      usbAttestation: { ...usbAttestation(), uinputPresent: true },
    }),
    /virtual input cannot satisfy/,
  );
  assert.throws(
    () => createUsbInputAdapter({
      ...base,
      usbAttestation: { ...usbAttestation(), serial: 'raw-device-serial' },
    }),
    /undeclared USB attestation field/,
  );
});

test('runner completes all four modes in exact order with one backend and cleanup per mode', async () => {
  const runContext = context();
  const results = await runCanonicalLadder(runContext);
  assert.deepEqual(results.map((result) => result.profileId), CANONICAL_MODES.map((mode) => mode.profileId));
  assert.deepEqual(results.map((result) => result.status), ['PASS', 'PASS', 'PASS', 'PASS']);
  assert.equal(runContext.releases, 4);
  for (const result of results) {
    assert.equal(result.groundTruth, 'automation');
    assert.equal(result.measurement.source, 'framebuffer');
    assert.equal(result.strategy.version, '1.0.0');
    assert.equal(result.tasks.length, 5);
  }
});

test('missing USB produces explicit UNAVAILABLE and never PASS', async () => {
  const result = await runExternalProfile('external_input_usb', context({ usbAvailable: false }));
  assert.equal(result.status, 'UNAVAILABLE');
  assert.equal(result.availability.capability, 'usb-hid');
  assert.notEqual(result.status, 'PASS');
});

test('USB or broker loss after interaction starts is an invalid run, not UNAVAILABLE', async () => {
  const runContext = context();
  runContext.createAdapters = async ({ mode }) => {
    const firewall = createActionFirewall();
    return {
      observation: fakeObservation(mode),
      input: createUsbInputAdapter({
        firewall,
        sleep: async () => {},
        usbAttestation: usbAttestation(),
        rfbInputEnabled: false,
        xtestEnabled: false,
        send: async () => {
          throw new CapabilityUnavailableError('usb-broker', 'device detached');
        },
        release: async () => {},
      }),
      browser: { version: 'test-browser', binarySha256: BROWSER_HASH, sandbox: true },
    };
  };
  const result = await runExternalProfile('external_input_usb', runContext);
  assert.equal(result.status, 'FAIL');
  assert.notEqual(result.status, 'UNAVAILABLE');
});

test('ordinary catalog invocation cannot run an external-input profile', async () => {
  await assert.rejects(
    () => runExternalProfile('external_input_virtual', {
      ...context(),
      executionKind: 'ordinary',
    }),
    ContractViolationError,
  );
});

test('result schema rejects detector-oracle or unbounded measurement fields', () => {
  assert.throws(
    () => validateMeasurement({
      verdict: 'DENY',
      source: 'framebuffer',
      confidence: 1,
      cueCount: 2,
      finalFrameSha256: HASH,
      riskScore: 99,
    }),
    /forbidden measurement field/,
  );
});
