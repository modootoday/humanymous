import assert from 'node:assert/strict';
import test from 'node:test';
import {
  BROWSER_PROBE_SCHEMA,
  DISPLAY_PROBE_SCHEMA,
  argvHasProcessType,
  createPreProjectProof,
  evaluateRuntimePurity,
  parseListeningSockets,
  sandboxStatusIsBounded,
  validateResetProof,
} from './runtime-purity.mjs';

const digest = (character) => character.repeat(64);

function fixture({
  profileId = 'external_input_dom_virtual',
  domRequired = true,
  usbRequired = false,
} = {}) {
  const target = domRequired ? 'browser-chromium-dom' : 'browser-chromium';
  const hardened = (image, extra = {}) => ({
    image,
    network_mode: 'none',
    read_only: true,
    restart: 'no',
    user: '1000:12001',
    cap_drop: ['ALL'],
    security_opt: ['no-new-privileges:true'],
    volumes: [],
    ...extra,
  });
  const services = {
    'external-browser': {
      image: 'browser-image',
      build: { target },
      networks: { 'external-target': null },
      volumes: domRequired
        ? [{ source: 'external-input-dom', target: '/run/dom-observer' }]
        : [],
    },
    'external-controller': {
      image: 'controller-image',
      networks: { 'external-control': null },
      volumes: [],
    },
    'external-display': {
      image: 'display-image',
      networks: { 'external-control': null },
      volumes: [],
      ...(usbRequired
        ? { devices: [{ source: '/dev/input/by-id/kbd', target: '/dev/input/kbd' }] }
        : {}),
    },
    'external-runtime-browser-probe': hardened('browser-image', {
      pid: 'service:external-browser',
      entrypoint: ['node', '/opt/external-input/runtime-browser-probe.mjs'],
      command: null,
      volumes: domRequired
        ? [{ source: 'external-input-dom', target: '/run/dom-observer' }]
        : [],
    }),
    'external-runtime-display-probe': hardened('display-image', {
      entrypoint: ['node', '/opt/external-input/runtime-display-probe.mjs'],
      command: null,
    }),
    'external-runtime-purity-evaluator': hardened('controller-image', {
      entrypoint: ['node', '/app/test/externalinput/runtime-purity-evaluator.mjs'],
      command: null,
    }),
    'external-score-trace-evaluator': hardened('controller-image', {
      entrypoint: ['node', '/app/test/externalinput/score-trace-evaluator.mjs'],
      command: null,
    }),
    'external-vusb-init': { privileged: true },
  };
  const browserProbe = {
    schemaVersion: BROWSER_PROBE_SCHEMA,
    runId: 'runtime-purity-test',
    profileId,
    engine: 'chromium',
    version: 'Chromium 123',
    binary: '/usr/lib/chromium/chromium',
    binarySha256: digest('a'),
    argv: ['/usr/lib/chromium/chromium'],
    forbiddenArgv: false,
    automationDependency: false,
    debugPortListening: false,
    sandboxed: true,
    sandboxStatus: [
      'NSpid:\t100 10 1',
      'CapEff:\t0000000000000000',
      'NoNewPrivs:\t1',
      'Seccomp:\t2',
    ],
    domObserverPresent: domRequired,
    domImagePresent: domRequired,
    domSocketPresent: domRequired,
    domExtensionSha256: domRequired ? digest('b') : '',
    domManifestSha256: domRequired ? digest('c') : '',
    uinputPresent: false,
  };
  const displayProbe = {
    schemaVersion: DISPLAY_PROBE_SCHEMA,
    runId: 'runtime-purity-test',
    profileId,
    xtestEnabled: !usbRequired,
    uinputPresent: false,
  };
  return {
    config: {
      services,
      networks: {
        'external-target': { internal: true },
        'external-control': { internal: true },
      },
    },
    browserProbe,
    displayProbe,
    profileId,
    runId: 'runtime-purity-test',
    engine: 'chromium',
    expectedDom: domRequired
      ? { extensionSha256: digest('b'), manifestSha256: digest('c') }
      : undefined,
    composeRaw: '{"services":{}}\n',
  };
}

test('container evaluator derives a bounded purity proof from raw probes and Compose config', () => {
  const result = evaluateRuntimePurity(fixture());
  assert.equal(result.purity.domObserverPresent, true);
  assert.equal(result.purity.xtestEnabled, true);
  assert.equal(result.purity.browserAutomationPortAbsent, true);
  assert.equal(result.runtimeEvidence.browser.binarySha256, digest('a'));
  assert.equal(result.runtimeEvidence.dom.extensionSha256, digest('b'));
});

test('runtime evaluator fails closed on policy, mount, and backend drift', () => {
  const argv = fixture();
  argv.browserProbe.argv.push('--remote-debugging-port=9222');
  argv.browserProbe.forbiddenArgv = true;
  assert.throws(() => evaluateRuntimePurity(argv), /stock-browser runtime proof/);

  const mount = fixture();
  mount.config.services['external-controller'].volumes.push({
    source: '/var/run/docker.sock',
    target: '/var/run/docker.sock',
  });
  assert.throws(() => evaluateRuntimePurity(mount), /forbidden Docker socket/);

  const backend = fixture();
  backend.displayProbe.xtestEnabled = false;
  assert.throws(() => evaluateRuntimePurity(backend), /input backend assignment/);

  const hiddenDom = fixture();
  hiddenDom.browserProbe.domExtensionSha256 = digest('d');
  assert.throws(() => evaluateRuntimePurity(hiddenDom), /DOM observer hashes/);

  const privilege = fixture();
  privilege.config.services['external-controller'].privileged = true;
  assert.throws(() => evaluateRuntimePurity(privilege), /unexpected privileged/);

  const evidenceService = fixture();
  evidenceService.config.services['external-runtime-purity-evaluator'].read_only = false;
  assert.throws(
    () => evaluateRuntimePurity(evidenceService),
    /runtime evidence services are not isolated/,
  );
});

test('runtime evaluator rejects evidence-process substitution and injection', () => {
  for (const [name, path] of [
    ['external-runtime-browser-probe', '/opt/external-input/runtime-browser-probe.mjs'],
    ['external-runtime-display-probe', '/opt/external-input/runtime-display-probe.mjs'],
    ['external-runtime-purity-evaluator', '/app/test/externalinput/runtime-purity-evaluator.mjs'],
    ['external-score-trace-evaluator', '/app/test/externalinput/score-trace-evaluator.mjs'],
  ]) {
    const scriptOverride = fixture();
    scriptOverride.config.services[name].volumes.push({
      type: 'bind',
      source: '/tmp/forged-evidence.mjs',
      target: path,
      read_only: true,
    });
    assert.throws(
      () => evaluateRuntimePurity(scriptOverride),
      /mounts over a protected executable/,
      `${name} script override must fail closed`,
    );

    for (const key of ['NODE_OPTIONS', 'PATH']) {
      const environmentInjection = fixture();
      environmentInjection.config.services[name].environment = {
        [key]: '/tmp/forged-runtime',
      };
      assert.throws(
        () => evaluateRuntimePurity(environmentInjection),
        new RegExp(`process-injection environment: ${key}`),
        `${name} ${key} injection must fail closed`,
      );
    }
  }

  const hook = fixture();
  hook.config.services['external-runtime-purity-evaluator'].post_start = [{
    command: ['node', '/tmp/forge-evidence.mjs'],
  }];
  assert.throws(() => evaluateRuntimePurity(hook), /lifecycle hooks are forbidden/);
});

test('probe parsers keep socket and sandbox evidence bounded', () => {
  const sockets = [
    'sl local_address rem_address st',
    '0: 00000000:2382 00000000:0000 0A',
    '1: 0B00007F:9E95 00000000:0000 0A',
  ].join('\n');
  assert.deepEqual(parseListeningSockets(sockets), ['00000000:2382']);
  assert.equal(argvHasProcessType([
    '/usr/lib/chromium/chromium',
    '--type=renderer',
  ], 'renderer'), true);
  assert.equal(argvHasProcessType([
    '/usr/lib/chromium/chromium --type=renderer --renderer-client-id=6',
  ], 'renderer'), true);
  assert.equal(argvHasProcessType([
    '/usr/lib/chromium/chromium--type=renderer',
    '--type=renderer-spoof',
  ], 'renderer'), false);
  assert.equal(sandboxStatusIsBounded([
    'NSpid:\t100 10 1',
    'CapEff:\t0000000000000000',
    'NoNewPrivs:\t1',
    'Seccomp:\t2',
  ].join('\n'), 'chromium'), true);
  assert.equal(sandboxStatusIsBounded([
    'NSpid:\t283',
    'CapEff:\t0000000000000000',
    'NoNewPrivs:\t1',
    'Seccomp:\t2',
    'Seccomp_filters:\t3',
  ].join('\n'), 'firefox'), true);
  assert.equal(sandboxStatusIsBounded([
    'NSpid:\t283',
    'CapEff:\t0000000000000000',
    'NoNewPrivs:\t0',
    'Seccomp:\t2',
    'Seccomp_filters:\t2',
  ].join('\n'), 'firefox'), false);
});

test('pre-project proof binds only empty bounded raw Docker inventories', () => {
  const proof = createPreProjectProof({
    runId: 'runtime-purity-test',
    profileId: 'external_input_virtual',
    composeProject: 'hmn-ext-runtime-purity-test-m1',
    composeRaw: '{"services":{}}\n',
    containersRaw: '',
    networksRaw: '\n',
    volumesRaw: '\r\n',
  });
  assert.match(proof.composeConfigSha256, /^sha256:[a-f0-9]{64}$/);
  assert.notEqual(
    proof.inventorySha256.containers,
    proof.inventorySha256.networks,
    'raw line endings remain bound into the proof',
  );
  assert.equal(validateResetProof({
    schemaVersion: 'humanymous.external-input-reset-proof/v1',
    runId: proof.runId,
    profileId: proof.profileId,
    composeProject: proof.composeProject,
    freshBrowserProfile: true,
    freshDetectorState: true,
    freshTaskState: true,
    composeConfigSha256: proof.composeConfigSha256,
    preProjectInventorySha256: proof.inventorySha256,
    visualManifestSha256: `sha256:${digest('d')}`,
  }, {
    runId: proof.runId,
    profileId: proof.profileId,
    composeProject: proof.composeProject,
  }).freshBrowserProfile, true);
  assert.throws(() => createPreProjectProof({
    runId: 'runtime-purity-test',
    profileId: 'external_input_virtual',
    composeProject: 'hmn-ext-runtime-purity-test-m1',
    composeRaw: '{"services":{}}\n',
    containersRaw: 'existing-container-id\n',
    networksRaw: '',
    volumesRaw: '',
  }), /containers inventory is not empty/);
});
