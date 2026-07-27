import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { validateImmutableServiceProcess } from '../runtime-purity.mjs';
import { atomicJson, receiptBase, SHA256, sha256 } from './common.mjs';
import { parseStrictJson } from './strict-json.mjs';

const RELEVANT = new Set([
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
  'external-vusb-gateway', 'external-vusb-broker',
  'external-vusb-ime-framebuffer-observer',
  'external-vusb-run-receipt', 'external-vusb-ime-run-receipt',
  'external-vusb-cleanup', 'external-vusb-ladder-assert',
]);
const PRIVILEGED = new Set([
  'external-vusb-init', 'external-vusb-cleanup',
]);
const NON_ROOT = new Set([...RELEVANT].filter((name) =>
  (name.startsWith('external-vusb-') && !PRIVILEGED.has(name)) ||
  name.startsWith('external-runtime-') ||
  name.startsWith('external-score-')));
const EXPECTED_DEVICES = Object.freeze({
  'external-vusb-gateway': [
    '/dev/vusb-command', '/dev/vusb-keyboard', '/dev/vusb-pointer',
  ],
  'external-vusb-broker': ['/dev/vusb-host-command'],
  'external-display': ['/dev/input/vusb-keyboard', '/dev/input/vusb-pointer'],
});
const BROWSER_CAPABILITIES = Object.freeze(['SETGID', 'SETUID', 'SYS_ADMIN', 'SYS_CHROOT']);
const FIXED_PROCESSES = Object.freeze({
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
  'external-vusb-admission': [
    'node', '/app/test/externalinput/vusb/admission.mjs',
  ],
  'external-vusb-profile-verify': [
    'node', '/app/test/externalinput/vusb/profile-verify.mjs',
  ],
  'external-vusb-preflight': [
    '/bin/sh', '/app/test/externalinput/vusb/lifecycle.sh', 'preflight',
  ],
  'external-vusb-init': [
    '/bin/sh', '/app/test/externalinput/vusb/lifecycle.sh', 'setup',
  ],
  'external-vusb-discover': [
    'node', '/app/test/externalinput/vusb/discover.mjs',
  ],
  'external-vusb-render': [
    'node', '/app/test/externalinput/vusb/render-devices.mjs',
  ],
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
  'external-vusb-broker': [
    'node', '/app/test/externalinput/vusb/broker-main.mjs',
  ],
  'external-vusb-ime-framebuffer-observer': [
    'node', '/app/test/externalinput/vusb/ime-framebuffer-observer.mjs',
  ],
  'external-vusb-run-receipt': [
    'node', '/app/test/externalinput/vusb/run-receipt.mjs',
  ],
  'external-vusb-ime-run-receipt': [
    'node', '/app/test/externalinput/vusb/ime-run-receipt.mjs',
  ],
  'external-vusb-cleanup': [
    '/bin/sh', '/app/test/externalinput/vusb/lifecycle.sh', 'cleanup',
  ],
  'external-vusb-ladder-assert': [
    'node', '/app/scripts/assert-external-input-vusb.mjs', '/ladder',
  ],
});
const RECEIPT_WRITERS = Object.freeze({
  'external-vusb-lock': ['/output', '/lock'],
  'external-vusb-parent-provisional': ['/output', '/provisional'],
  'external-vusb-parent-assert': ['/output', '/terminal'],
  'external-vusb-admission': ['/output', '/admission'],
  'external-vusb-profile-verify': ['/output', '/profile'],
  'external-vusb-ime-policy': ['/output', '/policy'],
  'external-vusb-preflight': ['/output', '/preflight'],
  'external-vusb-init': ['/output', '/setup'],
  'external-vusb-discover': ['/output', '/prepare'],
  'external-vusb-render': ['/output', '/mapping'],
  'external-vusb-attestation': ['/output', '/attestation'],
  'external-vusb-static-compose-guard': ['/output', '/static-guard'],
  'external-vusb-compose-guard': ['/output', '/resolved-guard'],
  'external-vusb-gateway': ['/gateway-evidence', '/gateway'],
  'external-vusb-broker': ['/broker-evidence', '/broker'],
  'external-vusb-ime-framebuffer-observer': ['/output', '/ime-observer'],
  'external-vusb-run-receipt': ['/output', '/run'],
  'external-vusb-ime-run-receipt': ['/output', '/run'],
  'external-vusb-cleanup': ['/output', '/cleanup'],
});

function targetOf(device) {
  if (typeof device === 'string') return device.split(':')[1];
  return device?.target;
}

export function validateComposeConfig(config, { phase = 'resolved' } = {}) {
  if (!['static', 'resolved'].includes(phase)) {
    throw new TypeError('Compose guard phase is invalid');
  }
  if (!config?.services || typeof config.services !== 'object') {
    throw new TypeError('merged Compose config has no services');
  }
  for (const name of RELEVANT) {
    if (!config.services[name]) throw new TypeError(`merged Compose config is missing ${name}`);
  }
  for (const [name, service] of Object.entries(config.services)) {
    if (service.privileged === true && !PRIVILEGED.has(name)) {
      throw new TypeError(`unexpected privileged service: ${name}`);
    }
    const capabilities = [...(service.cap_add || [])].sort();
    if (name === 'external-browser') {
      if (JSON.stringify(capabilities) !== JSON.stringify(BROWSER_CAPABILITIES)) {
        throw new TypeError('browser sandbox capability set differs');
      }
    } else if (capabilities.length > 0) {
      throw new TypeError(`unexpected added capability in ${name}`);
    }
    for (const volume of service.volumes || []) {
      const source = typeof volume === 'string' ? volume.split(':')[0] : volume.source;
      const target = typeof volume === 'string' ? volume.split(':')[1] : volume.target;
      if (source === '/var/run/docker.sock' || target === '/var/run/docker.sock' ||
          target === '/dev/uinput' || source === '/run/user' ||
          source === '/run/dbus' ||
          (name === 'external-controller' &&
            (String(source).includes('ibus') || String(target).includes('ibus') ||
             String(source).includes('dbus') || String(target).includes('dbus')))) {
        throw new TypeError(`forbidden mount in ${name}`);
      }
    }
    if (name === 'external-controller') {
      const environment = service.environment || {};
      for (const key of ['DBUS_SESSION_BUS_ADDRESS', 'IBUS_ADDRESS', 'GTK_IM_MODULE', 'XMODIFIERS']) {
        if (Object.hasOwn(environment, key)) {
          throw new TypeError(`controller IME environment is forbidden: ${key}`);
        }
      }
    }
    if (RELEVANT.has(name) && !SHA256.test(service.image || '')) {
      throw new TypeError(`${name} is not pinned to a preloaded sha256 image`);
    }
    if (service.restart && service.restart !== 'no') {
      throw new TypeError(`${name} has a restart policy`);
    }
    if (NON_ROOT.has(name) && !/^[1-9][0-9]*(?::[0-9]+)?$/.test(String(service.user || ''))) {
      throw new TypeError(`${name} must run as an explicit non-root user`);
    }
  }
  for (const name of PRIVILEGED) {
    if (config.services[name].privileged !== true) {
      throw new TypeError(`${name} must be the exact privileged lifecycle service`);
    }
  }
  const immutableProcesses = {
    ...FIXED_PROCESSES,
    'external-vusb-ime-policy': [
      'node',
      '/app/test/externalinput/vusb/ime-policy.mjs',
      'write',
      config.services['external-vusb-gateway'].environment?.HM_VUSB_RUN_ID,
      config.services['external-browser'].environment?.HM_EXTERNAL_IME_LOCALE,
      '/output/ime-policy.json',
    ],
  };
  for (const [name, entrypoint] of Object.entries(immutableProcesses)) {
    validateImmutableServiceProcess(config.services[name], { name, entrypoint });
  }
  for (const [name, [outputTarget, sourceSuffix]] of Object.entries(RECEIPT_WRITERS)) {
    const volumes = config.services[name].volumes || [];
    const rootMounts = volumes.filter((volume) => volume.target === '/receipts');
    if (rootMounts.length !== 1 || rootMounts[0].read_only !== true) {
      throw new TypeError(`${name} must receive the receipt DAG root read-only`);
    }
    const outputMounts = volumes.filter((volume) => volume.target === outputTarget);
    if (outputMounts.length !== 1 || outputMounts[0].type !== 'bind' ||
        outputMounts[0].read_only === true ||
        !String(outputMounts[0].source || '').replaceAll('\\', '/').endsWith(sourceSuffix)) {
      throw new TypeError(`${name} writable receipt stage is invalid`);
    }
  }
  if (phase === 'resolved') {
    for (const [name, targets] of Object.entries(EXPECTED_DEVICES)) {
      const actual = (config.services[name].devices || []).map(targetOf).sort();
      if (JSON.stringify(actual) !== JSON.stringify([...targets].sort())) {
        throw new TypeError(`${name} exact device mappings differ`);
      }
    }
  }
  const allTargets = [];
  for (const [name, service] of Object.entries(config.services)) {
    const targets = (service.devices || []).map(targetOf);
    if ((phase === 'static' || !Object.hasOwn(EXPECTED_DEVICES, name)) && targets.length > 0) {
      throw new TypeError(`unexpected device mapping in ${name}`);
    }
    allTargets.push(...targets);
  }
  if (phase === 'resolved' && new Set(allTargets).size !== 6) {
    throw new TypeError('canonical mapping count is not six');
  }
  const profileMount = (config.services['external-vusb-gateway'].volumes || [])
    .find((volume) => volume.target === '/profile');
  const verificationProfileMount =
    (config.services['external-vusb-profile-verify'].volumes || [])
      .find((volume) => volume.target === '/profile-image');
  if (profileMount?.type !== 'image' || profileMount.read_only !== true ||
      profileMount.image?.subpath !== 'profile' ||
      verificationProfileMount?.type !== 'image' ||
      verificationProfileMount.read_only !== true ||
      verificationProfileMount.source !== profileMount.source ||
      !/^hmnvusbprofile:vusb-[a-z0-9-]{8,63}$/.test(
        String(profileMount.source || ''),
      )) {
    throw new TypeError('gateway profile image subpath mount is invalid');
  }
  if (JSON.stringify(config).toLowerCase().includes('"cdi"')) {
    throw new TypeError('CDI content is forbidden in canonical v1');
  }
  const discoverDev = (config.services['external-vusb-discover'].volumes || [])
    .find((volume) => volume.target === '/host-dev');
  if (discoverDev?.source !== '/dev' || discoverDev.read_only !== true) {
    throw new TypeError('discovery host device inventory must be read-only');
  }
  const imeLocale = config.services['external-browser'].environment?.HM_EXTERNAL_IME_LOCALE || '';
  if (imeLocale) {
    const policyPath = '/receipts/policy/ime-policy.json';
    if (config.services['external-vusb-gateway'].environment?.HM_VUSB_IME_POLICY !== policyPath ||
        config.services['external-vusb-broker'].environment?.HM_VUSB_IME_POLICY !== policyPath) {
      throw new TypeError('both IME policy enforcement points must use the run policy');
    }
  }
  return true;
}

export async function guardCompose({
  runId,
  configPath,
  destination,
  phase = 'resolved',
  now = new Date(),
}) {
  const raw = await readFile(configPath, 'utf8');
  const config = parseStrictJson(raw, 'merged Compose config');
  validateComposeConfig(config, { phase });
  const receipt = {
    ...receiptBase(phase === 'static' ? 'compose-static-guard' : 'compose-guard', runId, now),
    phase,
    composeConfigSha256: `sha256:${sha256(raw)}`,
    exactDeviceMappings: phase === 'resolved' ? 6 : 0,
    exclusiveAssignment: phase === 'resolved',
    cdi: false,
    controllerImeBusAbsent: true,
    directTextActionForbidden: true,
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
  guardCompose({
    runId: required('HM_VUSB_RUN_ID'),
    configPath: required('HM_VUSB_COMPOSE_CONFIG'),
    destination: required('HM_VUSB_COMPOSE_GUARD_RECEIPT'),
    phase: process.env.HM_VUSB_COMPOSE_GUARD_PHASE || 'resolved',
  }).catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-compose-guard',
      code: 'PURITY_FAIL',
      message: error.message,
    })}\n`);
    process.exitCode = 1;
  });
}
