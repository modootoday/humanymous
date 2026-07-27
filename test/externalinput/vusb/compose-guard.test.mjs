import assert from 'node:assert/strict';
import test from 'node:test';
import { validateComposeConfig } from './compose-guard.mjs';

const digest = `sha256:${'a'.repeat(64)}`;
const names = [
  'lab-pki', 'core', 'external-display', 'external-browser',
  'external-controller', 'external-runtime-browser-probe',
  'external-runtime-display-probe', 'external-runtime-purity-evaluator',
  'external-score-trace-evaluator',
  'external-vusb-lock',
  'external-vusb-parent-provisional', 'external-vusb-parent-assert',
  'external-vusb-admission', 'external-vusb-profile-verify', 'external-vusb-ime-policy',
  'external-vusb-preflight', 'external-vusb-init', 'external-vusb-discover',
  'external-vusb-render', 'external-vusb-attestation',
  'external-vusb-static-compose-guard', 'external-vusb-compose-guard',
  'external-vusb-gateway',
  'external-vusb-broker', 'external-vusb-run-receipt',
  'external-vusb-ime-framebuffer-observer',
  'external-vusb-ime-run-receipt', 'external-vusb-cleanup',
  'external-vusb-ladder-assert',
];
const receiptWriters = {
  'external-vusb-lock': ['/output', 'lock'],
  'external-vusb-parent-provisional': ['/output', 'provisional'],
  'external-vusb-parent-assert': ['/output', 'terminal'],
  'external-vusb-admission': ['/output', 'admission'],
  'external-vusb-profile-verify': ['/output', 'profile'],
  'external-vusb-ime-policy': ['/output', 'policy'],
  'external-vusb-preflight': ['/output', 'preflight'],
  'external-vusb-init': ['/output', 'setup'],
  'external-vusb-discover': ['/output', 'prepare'],
  'external-vusb-render': ['/output', 'mapping'],
  'external-vusb-attestation': ['/output', 'attestation'],
  'external-vusb-static-compose-guard': ['/output', 'static-guard'],
  'external-vusb-compose-guard': ['/output', 'resolved-guard'],
  'external-vusb-gateway': ['/gateway-evidence', 'gateway'],
  'external-vusb-broker': ['/broker-evidence', 'broker'],
  'external-vusb-ime-framebuffer-observer': ['/output', 'ime-observer'],
  'external-vusb-run-receipt': ['/output', 'run'],
  'external-vusb-ime-run-receipt': ['/output', 'run'],
  'external-vusb-cleanup': ['/output', 'cleanup'],
};
const protectedProcesses = {
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
  'external-controller': null,
  'external-vusb-lock': [
    '/bin/sh', '/app/test/externalinput/vusb/lifecycle.sh', 'lock',
  ],
  'external-vusb-parent-provisional': [
    'node', '/app/test/externalinput/vusb/parent-assert.mjs', 'provisional',
  ],
  'external-vusb-parent-assert': [
    'node', '/app/test/externalinput/vusb/parent-assert.mjs', 'terminal',
  ],
  'external-vusb-admission': ['node', '/app/test/externalinput/vusb/admission.mjs'],
  'external-vusb-profile-verify': ['node', '/app/test/externalinput/vusb/profile-verify.mjs'],
  'external-vusb-ime-policy': [
    'node', '/app/test/externalinput/vusb/ime-policy.mjs',
    'write', 'compose-guard-test', '', '/output/ime-policy.json',
  ],
  'external-vusb-preflight': [
    '/bin/sh', '/app/test/externalinput/vusb/lifecycle.sh', 'preflight',
  ],
  'external-vusb-init': [
    '/bin/sh', '/app/test/externalinput/vusb/lifecycle.sh', 'setup',
  ],
  'external-vusb-discover': ['node', '/app/test/externalinput/vusb/discover.mjs'],
  'external-vusb-render': ['node', '/app/test/externalinput/vusb/render-devices.mjs'],
  'external-vusb-attestation': [
    'node', '/app/test/externalinput/vusb/make-attestation.mjs',
  ],
  'external-vusb-static-compose-guard': [
    'node', '/app/test/externalinput/vusb/compose-guard.mjs',
  ],
  'external-vusb-compose-guard': [
    'node', '/app/test/externalinput/vusb/compose-guard.mjs',
  ],
  'external-vusb-gateway': null,
  'external-vusb-broker': ['node', '/app/test/externalinput/vusb/broker-main.mjs'],
  'external-vusb-ime-framebuffer-observer': [
    'node', '/app/test/externalinput/vusb/ime-framebuffer-observer.mjs',
  ],
  'external-vusb-run-receipt': ['node', '/app/test/externalinput/vusb/run-receipt.mjs'],
  'external-vusb-ime-run-receipt': [
    'node', '/app/test/externalinput/vusb/ime-run-receipt.mjs',
  ],
  'external-vusb-cleanup': [
    '/bin/sh', '/app/test/externalinput/vusb/lifecycle.sh', 'cleanup',
  ],
  'external-vusb-ladder-assert': [
    'node', '/app/scripts/assert-external-input-vusb.mjs', '/ladder',
  ],
};

function config() {
  const services = Object.fromEntries(names.map((name) => [name, {
    image: digest,
    restart: 'no',
    volumes: [],
  }]));
  for (const name of ['external-vusb-init', 'external-vusb-cleanup']) {
    services[name].privileged = true;
  }
  for (const name of names.filter((name) =>
    (name.startsWith('external-vusb-') &&
      !['external-vusb-init', 'external-vusb-cleanup'].includes(name)) ||
    name.startsWith('external-runtime-') ||
    name.startsWith('external-score-'))) {
    services[name].user = '1000';
  }
  for (const [name, entrypoint] of Object.entries(protectedProcesses)) {
    services[name].entrypoint = entrypoint === null ? null : [...entrypoint];
    services[name].command = null;
  }
  for (const [name, [target, stage]] of Object.entries(receiptWriters)) {
    services[name].volumes.push(
      {
        type: 'bind',
        source: '/tmp/humanymous-vusb-run',
        target: '/receipts',
        read_only: true,
      },
      {
        type: 'bind',
        source: `/tmp/humanymous-vusb-run/${stage}`,
        target,
      },
    );
  }
  services['external-vusb-gateway'].devices = [
    { source: '/dev/ttyGS0', target: '/dev/vusb-command', permissions: 'rw' },
    { source: '/dev/hidg0', target: '/dev/vusb-keyboard', permissions: 'rw' },
    { source: '/dev/hidg1', target: '/dev/vusb-pointer', permissions: 'rw' },
  ];
  services['external-vusb-discover'].volumes.push({
    type: 'bind', source: '/dev', target: '/host-dev', read_only: true,
  });
  services['external-browser'].cap_add = [
    'SYS_ADMIN', 'SYS_CHROOT', 'SETUID', 'SETGID',
  ];
  services['external-browser'].environment = {
    HM_EXTERNAL_BROWSER: 'chromium',
    HM_EXTERNAL_IME_LOCALE: '',
  };
  services['external-vusb-gateway'].environment = {
    HM_VUSB_RUN_ID: 'compose-guard-test',
  };
  services['external-vusb-gateway'].volumes.push({
    type: 'image',
    source: 'hmnvusbprofile:vusb-compose-guard-test',
    target: '/profile',
    read_only: true,
    image: { subpath: 'profile' },
  });
  services['external-vusb-profile-verify'].volumes.push({
    type: 'image',
    source: 'hmnvusbprofile:vusb-compose-guard-test',
    target: '/profile-image',
    read_only: true,
  });
  services['external-vusb-broker'].devices = [
    { source: '/dev/serial/by-id/command', target: '/dev/vusb-host-command', permissions: 'rw' },
  ];
  services['external-display'].devices = [
    { source: '/dev/input/by-id/keyboard', target: '/dev/input/vusb-keyboard', permissions: 'r' },
    { source: '/dev/input/by-id/pointer', target: '/dev/input/vusb-pointer', permissions: 'r' },
  ];
  return { services };
}

test('Compose guard admits exact digest-pinned privilege and device topology', () => {
  assert.equal(validateComposeConfig(config()), true);
  const beforeMutation = config();
  for (const service of Object.values(beforeMutation.services)) delete service.devices;
  assert.equal(validateComposeConfig(beforeMutation, { phase: 'static' }), true);
  const firefox = config();
  firefox.services['external-browser'].environment.HM_EXTERNAL_BROWSER = 'firefox';
  assert.equal(validateComposeConfig(firefox), true);

  const ime = config();
  ime.services['external-browser'].environment.HM_EXTERNAL_IME_LOCALE = 'ko-KR';
  ime.services['external-vusb-ime-policy'].entrypoint[4] = 'ko-KR';
  ime.services['external-vusb-gateway'].environment = {
    ...ime.services['external-vusb-gateway'].environment,
    HM_VUSB_IME_POLICY: '/receipts/policy/ime-policy.json',
  };
  ime.services['external-vusb-broker'].environment = {
    HM_VUSB_IME_POLICY: '/receipts/policy/ime-policy.json',
  };
  assert.equal(validateComposeConfig(ime), true);
});

test('Compose guard rejects broad privilege, Docker socket, and mapping drift', () => {
  const privileged = config();
  privileged.services['external-vusb-gateway'].privileged = true;
  assert.throws(() => validateComposeConfig(privileged), /unexpected privileged/);

  const missingPrivilege = config();
  delete missingPrivilege.services['external-vusb-init'].privileged;
  assert.throws(() => validateComposeConfig(missingPrivilege), /must be the exact privileged/);

  const writableDag = config();
  writableDag.services['external-vusb-admission'].volumes
    .find((volume) => volume.target === '/receipts').read_only = false;
  assert.throws(() => validateComposeConfig(writableDag), /DAG root read-only/);

  const rootVerifier = config();
  rootVerifier.services['external-vusb-admission'].user = '0';
  assert.throws(() => validateComposeConfig(rootVerifier), /explicit non-root/);

  const writableDiscovery = config();
  writableDiscovery.services['external-vusb-discover'].volumes
    .find((volume) => volume.target === '/host-dev').read_only = false;
  assert.throws(() => validateComposeConfig(writableDiscovery), /inventory must be read-only/);

  const missingImePolicy = config();
  missingImePolicy.services['external-browser'].environment.HM_EXTERNAL_IME_LOCALE = 'ko-KR';
  missingImePolicy.services['external-vusb-ime-policy'].entrypoint[4] = 'ko-KR';
  assert.throws(() => validateComposeConfig(missingImePolicy), /both IME policy/);

  const socket = config();
  socket.services['external-controller'].volumes.push({
    type: 'bind', source: '/var/run/docker.sock', target: '/var/run/docker.sock',
  });
  assert.throws(() => validateComposeConfig(socket), /forbidden mount/);

  const hostBus = config();
  hostBus.services['external-controller'].volumes.push({
    type: 'bind', source: '/run/user', target: '/run/user',
  });
  assert.throws(() => validateComposeConfig(hostBus), /forbidden mount/);

  const missing = config();
  missing.services['external-display'].devices.pop();
  assert.throws(() => validateComposeConfig(missing), /exact device mappings differ/);

  const hidden = config();
  hidden.services['external-controller'].devices = [{
    source: '/dev/uinput', target: '/dev/uinput', permissions: 'rw',
  }];
  assert.throws(() => validateComposeConfig(hidden), /unexpected device mapping/);

  const capability = config();
  capability.services['external-controller'].cap_add = ['SYS_ADMIN'];
  assert.throws(() => validateComposeConfig(capability), /unexpected added capability/);
});

test('RED r01: Compose guard rejects runtime browser-probe entrypoint substitution', () => {
  const substitutedProbe = config();
  substitutedProbe.services['external-runtime-browser-probe'].entrypoint = [
    'node',
    '-e',
    'require("node:fs").writeFileSync(process.env.HM_EXTERNAL_BROWSER_PROBE_PATH, "{}")',
  ];
  assert.throws(
    () => validateComposeConfig(substitutedProbe),
    /external-runtime-browser-probe entrypoint differs/,
  );
});

test('runtime evidence producer commands are immutable', () => {
  const displaySubstitution = config();
  displaySubstitution.services['external-runtime-display-probe'].entrypoint = [
    'node', '-e', 'process.exit(0)',
  ];
  assert.throws(
    () => validateComposeConfig(displaySubstitution),
    /external-runtime-display-probe entrypoint differs/,
  );

  const evaluatorArguments = config();
  evaluatorArguments.services['external-runtime-purity-evaluator'].command = [
    '--forged-evidence',
  ];
  assert.throws(
    () => validateComposeConfig(evaluatorArguments),
    /external-runtime-purity-evaluator command differs/,
  );
});

test('RED: Compose guard rejects evidence script mounts and process environment injection', () => {
  for (const [name, path] of [
    ['external-runtime-browser-probe', '/opt/external-input/runtime-browser-probe.mjs'],
    ['external-runtime-display-probe', '/opt/external-input/runtime-display-probe.mjs'],
    ['external-runtime-purity-evaluator', '/app/test/externalinput/runtime-purity-evaluator.mjs'],
    ['external-score-trace-evaluator', '/app/test/externalinput/score-trace-evaluator.mjs'],
  ]) {
    const scriptOverride = config();
    scriptOverride.services[name].volumes.push({
      type: 'bind',
      source: '/tmp/forged-evidence.mjs',
      target: path,
      read_only: true,
    });
    assert.throws(
      () => validateComposeConfig(scriptOverride),
      /mounts over a protected executable/,
      `${name} bind override must fail closed`,
    );

    for (const key of ['NODE_OPTIONS', 'PATH']) {
      const environmentInjection = config();
      environmentInjection.services[name].environment = {
        [key]: '/tmp/forged-runtime',
      };
      assert.throws(
        () => validateComposeConfig(environmentInjection),
        new RegExp(`process-injection environment: ${key}`),
        `${name} ${key} injection must fail closed`,
      );
    }
  }
});

test('RED: lifecycle and receipt-writer processes are immutable', () => {
  const receiptSubstitution = config();
  receiptSubstitution.services['external-vusb-run-receipt'].entrypoint = [
    'node', '/tmp/forged-receipt.mjs',
  ];
  assert.throws(
    () => validateComposeConfig(receiptSubstitution),
    /external-vusb-run-receipt entrypoint differs/,
  );

  const lifecycleArguments = config();
  lifecycleArguments.services['external-vusb-cleanup'].command = ['skip-cleanup'];
  assert.throws(
    () => validateComposeConfig(lifecycleArguments),
    /external-vusb-cleanup command differs/,
  );

  const scriptDirectoryOverride = config();
  scriptDirectoryOverride.services['external-vusb-compose-guard'].volumes.push({
    type: 'bind',
    source: '/tmp/forged-vusb-scripts',
    target: '/app/test/externalinput/vusb',
    read_only: true,
  });
  assert.throws(
    () => validateComposeConfig(scriptDirectoryOverride),
    /mounts over a protected executable/,
  );

  const lifecycleHook = config();
  lifecycleHook.services['external-vusb-gateway'].post_start = [{
    command: ['node', '/tmp/forge-gateway-evidence.mjs'],
  }];
  assert.throws(
    () => validateComposeConfig(lifecycleHook),
    /lifecycle hooks are forbidden/,
  );
});
