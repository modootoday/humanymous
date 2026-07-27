#!/usr/bin/env node

import { spawn } from 'node:child_process';
import {
  access,
  chmod,
  mkdir,
  realpath,
  writeFile,
} from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import process from 'node:process';
import {
  CANONICAL_MODES,
  VIRTUAL_USB_MODES as CONTRACT_VIRTUAL_USB_MODES,
} from '../test/externalinput/contracts.mjs';

const ROOT = resolve(import.meta.dirname, '..');
const COMPOSE_BASE = join(ROOT, 'deployments', 'compose.yaml');
const COMPOSE_EXTERNAL = join(ROOT, 'deployments', 'compose', 'external-input-bots.yaml');
const COMPOSE_DOM = join(ROOT, 'deployments', 'compose', 'external-input-dom.yaml');
const COMPOSE_FIREFOX = join(ROOT, 'deployments', 'compose', 'external-input-firefox.yaml');
const COMPOSE_HIL = join(ROOT, 'deployments', 'compose', 'external-input-hil.yaml');
const COMPOSE_VUSB = join(ROOT, 'deployments', 'compose', 'external-input-vusb.yaml');
const ARTIFACTS = join(ROOT, 'deployments', 'artifacts');
const MAX_MODE_MS = 5 * 60 * 1_000;

function runtimeMode(mode, { virtualUsb = false } = {}) {
  const virtualUsbRuntime =
    virtualUsb && mode.usbOrigin === 'kernel-emulated';
  return Object.freeze({
    ...mode,
    composeProfile: virtualUsbRuntime
      ? 'vusb-run'
      : `external-input-m${mode.sequence}`,
    ...(virtualUsbRuntime ? { vusb: true } : {}),
  });
}

const MODES = Object.freeze(CANONICAL_MODES.map((mode) => runtimeMode(mode)));
const VUSB_MODES = Object.freeze(
  CONTRACT_VIRTUAL_USB_MODES.map((mode) => runtimeMode(mode, { virtualUsb: true })),
);

function parseArgs(argv) {
  const options = {
    browser: 'chromium',
    hil: false,
    keep: false,
    build: true,
    domRequired: undefined,
    runId: `sot41-${new Date().toISOString().replace(/[-:.TZ]/g, '').slice(0, 14)}`,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--hil') options.hil = true;
    else if (value === '--keep') options.keep = true;
    else if (value === '--no-build') options.build = false;
    else if (value === '--browser') options.browser = argv[++index];
    else if (value === '--run-id') options.runId = argv[++index];
    else if (value === '--dom-required') {
      const flag = argv[++index];
      if (!['0', '1'].includes(flag)) {
        throw new TypeError('--dom-required must be 0 or 1');
      }
      options.domRequired = flag === '1';
    }
    else throw new TypeError(`unknown argument: ${value}`);
  }
  if (!['chromium', 'firefox'].includes(options.browser)) {
    throw new TypeError('--browser must be chromium or firefox');
  }
  if (!/^[a-z0-9][a-z0-9-]{0,47}$/.test(options.runId)) {
    throw new TypeError('--run-id must be a lower-case Docker-safe token (max 48 chars)');
  }
  return Object.freeze(options);
}

function run(command, args, {
  env = process.env,
  cwd = ROOT,
  timeoutMs = MAX_MODE_MS,
  allowExitCodes = [0],
  quiet = false,
} = {}) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(command, args, {
      cwd,
      env,
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    const timer = setTimeout(() => {
      child.kill();
      rejectPromise(new Error(`${command} timed out after ${timeoutMs}ms`));
    }, timeoutMs);
    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString('utf8');
      if (!quiet) process.stdout.write(chunk);
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString('utf8');
      if (!quiet) process.stderr.write(chunk);
    });
    child.on('error', (error) => {
      clearTimeout(timer);
      rejectPromise(error);
    });
    child.on('close', (code) => {
      clearTimeout(timer);
      if (!allowExitCodes.includes(code)) {
        rejectPromise(new Error(
          `${command} ${args.join(' ')} exited ${code}: ${stderr.trim() || stdout.trim()}`,
        ));
        return;
      }
      resolvePromise({ code, stdout, stderr });
    });
  });
}

async function docker(args, options = {}) {
  return run(process.env.DOCKER || 'docker', args, options);
}

function composeArgs(mode, { hil = false, browser = 'chromium' } = {}) {
  const args = [
    'compose',
    '-f', COMPOSE_BASE,
    '-f', COMPOSE_EXTERNAL,
  ];
  if (mode.domRequired) args.push('-f', COMPOSE_DOM);
  if (browser === 'firefox') args.push('-f', COMPOSE_FIREFOX);
  if (mode.usbRequired && hil) args.push('-f', COMPOSE_HIL);
  if (mode.vusb) {
    args.push('-f', COMPOSE_VUSB);
    const override = process.env.HM_VUSB_DEVICE_OVERRIDE;
    if (!override) throw new Error('HM_VUSB_DEVICE_OVERRIDE is required for virtual USB mode');
    args.push('-f', override);
  }
  args.push('--profile', mode.composeProfile);
  return args;
}

function modeEnv(mode, options, controlDir, scoreDir, project = 'external-input-pilot') {
  return {
    ...process.env,
    HM_EXTERNAL_MODE: mode.profileId,
    HM_EXTERNAL_COMPOSE_PROFILE: mode.composeProfile,
    HM_EXTERNAL_BROWSER: options.browser,
    HM_EXTERNAL_BROWSER_TARGET: `browser-${options.browser}`,
    HM_EXTERNAL_BROWSER_TARGET_DOM: `browser-${options.browser}-dom`,
    HM_EXTERNAL_RUN_ID: options.runId,
    HM_EXTERNAL_SEED:
      process.env.HM_EXTERNAL_STRATEGY_SEED || `${options.runId}-${mode.sequence}`,
    HM_EXTERNAL_CONTROL_DIR: controlDir,
    HM_EXTERNAL_SCORE_DIR: scoreDir,
    HM_EXTERNAL_COMPOSE_PROJECT: project,
    HM_EXTERNAL_HOST_UID: typeof process.getuid === 'function'
      ? String(process.getuid())
      : '1000',
  };
}

async function composeConfig(mode, options, env, project) {
  const result = await docker([
    ...composeArgs(mode, options),
    '-p', project,
    'config',
    '--format', 'json',
  ], { env, quiet: true, timeoutMs: 30_000 });
  return result.stdout;
}

async function prepareControl(controlDir, composeRaw, inventory) {
  await writeFile(join(controlDir, 'compose-config.json'), composeRaw, {
    encoding: 'utf8',
    flag: 'wx',
  });
  await Promise.all([
    writeFile(join(controlDir, 'preproject-containers.txt'), inventory.containers, {
      encoding: 'utf8',
      flag: 'wx',
    }),
    writeFile(join(controlDir, 'preproject-networks.txt'), inventory.networks, {
      encoding: 'utf8',
      flag: 'wx',
    }),
    writeFile(join(controlDir, 'preproject-volumes.txt'), inventory.volumes, {
      encoding: 'utf8',
      flag: 'wx',
    }),
  ]);
}

async function captureProjectInventory(project) {
  const probes = await Promise.all([
    docker([
      'ps', '-a', '--filter', `label=com.docker.compose.project=${project}`, '-q',
    ], { quiet: true, timeoutMs: 30_000 }),
    docker([
      'network', 'ls', '--filter', `label=com.docker.compose.project=${project}`, '-q',
    ], { quiet: true, timeoutMs: 30_000 }),
    docker([
      'volume', 'ls', '--filter', `label=com.docker.compose.project=${project}`, '-q',
    ], { quiet: true, timeoutMs: 30_000 }),
  ]);
  return Object.freeze({
    containers: probes[0].stdout,
    networks: probes[1].stdout,
    volumes: probes[2].stdout,
  });
}

async function down(project, mode, options, env) {
  await docker([
    ...composeArgs(mode, options),
    '-p', project,
    'down', '-v', '--remove-orphans',
  ], { env, quiet: false, timeoutMs: 60_000, allowExitCodes: [0] });
}

async function runMode(mode, options, runRoot) {
  const project = `hmn-ext-${options.runId}-m${mode.sequence}`.slice(0, 63);
  const controlDir = join(runRoot, `control-m${mode.sequence}`);
  const scoreDir = join(runRoot, `score-m${mode.sequence}`);
  const env = modeEnv(mode, options, controlDir, scoreDir, project);
  await Promise.all([
    mkdir(controlDir, { recursive: true }),
    mkdir(scoreDir, { recursive: true }),
  ]);
  await chmod(scoreDir, 0o733);
  const inventory = await captureProjectInventory(project);
  const composeRaw = await composeConfig(mode, options, env, project);
  await prepareControl(controlDir, composeRaw, inventory);
  console.log(`\n[external-input] mode ${mode.sequence}/4 ${mode.profileId}`);
  try {
    if (options.build) {
      const buildServices = [
        'lab-pki',
        'core',
        'external-display',
        'external-browser',
        'external-controller',
        'external-input-assert',
        'external-runtime-browser-probe',
        'external-runtime-display-probe',
        'external-runtime-purity-evaluator',
        'external-score-trace-evaluator',
      ];
      if (mode.usbRequired && !mode.vusb) buildServices.push('external-usb-broker');
      await docker([
        ...composeArgs(mode, options),
        '-p', project,
        'build',
        ...buildServices,
      ], { env, timeoutMs: MAX_MODE_MS });
    }
    await docker([
      ...composeArgs(mode, options),
      '-p', project,
      'run', '--rm', '--no-deps',
      '-e', 'HM_EXTERNAL_RUNTIME_EVALUATOR_PHASE=preflight',
      'external-runtime-purity-evaluator',
    ], { env, timeoutMs: 60_000 });
    const up = [
      ...composeArgs(mode, options),
      '-p', project,
      'up', '-d',
    ];
    up.push('lab-pki', 'core', 'external-display', 'external-browser');
    if (mode.vusb) up.push('external-vusb-gateway', 'external-vusb-broker');
    else if (mode.usbRequired) up.push('external-usb-broker');
    await docker(up, { env, timeoutMs: MAX_MODE_MS });

    // Evidence probes are independent and short-lived. Run them serially so
    // the nested Docker/QEMU cell does not pay two image/process peaks at once.
    await docker([
      ...composeArgs(mode, options),
      '-p', project,
      'run', '--rm', '--no-deps',
      'external-runtime-browser-probe',
    ], { env, timeoutMs: 60_000 });
    await docker([
      ...composeArgs(mode, options),
      '-p', project,
      'run', '--rm', '--no-deps',
      'external-runtime-display-probe',
    ], { env, timeoutMs: 60_000 });
    await docker([
      ...composeArgs(mode, options),
      '-p', project,
      'run', '--rm', '--no-deps',
      'external-runtime-purity-evaluator',
    ], { env, timeoutMs: 60_000 });

    await docker([
      ...composeArgs(mode, options),
      '-p', project,
      'run', '--rm', '--no-deps',
      '--entrypoint', 'node',
      'external-controller',
      '/app/scripts/external-input-readiness.mjs',
    ], { env, timeoutMs: 60_000 });

    const controllerArgs = [
      ...composeArgs(mode, options),
      '-p', project,
      'run', '--rm', '--no-deps',
      'external-controller',
    ];
    await docker(controllerArgs, { env, timeoutMs: MAX_MODE_MS });
    // Core owns the authoritative JSONL sink. Stop it before evaluation so the
    // async logger is flushed and the offline evaluator sees one terminal file.
    await docker([
      ...composeArgs(mode, options),
      '-p', project,
      'stop', 'external-browser', 'core',
    ], { env, timeoutMs: 60_000 });
    await docker([
      ...composeArgs(mode, options),
      '-p', project,
      'run', '--rm', '--no-deps',
      'external-score-trace-evaluator',
    ], { env, timeoutMs: 60_000 });
    return { status: 'MEASURED', project, controlDir };
  } finally {
    if (!options.keep && process.env.HM_EXTERNAL_SKIP_DOWN !== '1') {
      await down(project, mode, options, env);
    }
  }
}

function requireAbsoluteLinuxById(value, prefix, name) {
  if (!value || !value.startsWith(prefix) || !value.includes('/by-id/')) {
    throw new Error(`${name} must be an exact stable ${prefix}by-id path`);
  }
}

async function hilPreflight(options) {
  if (!options.hil) {
    throw new Error('physical USB execution requires --hil');
  }
  if (process.platform !== 'linux') {
    throw new Error('canonical physical USB evidence requires a dedicated Linux Docker host');
  }
  const paths = {
    command: process.env.HM_EXTERNAL_USB_COMMAND,
    keyboard: process.env.HM_EXTERNAL_USB_KEYBOARD,
    pointer: process.env.HM_EXTERNAL_USB_POINTER,
  };
  requireAbsoluteLinuxById(paths.command, '/dev/serial/', 'HM_EXTERNAL_USB_COMMAND');
  requireAbsoluteLinuxById(paths.keyboard, '/dev/input/', 'HM_EXTERNAL_USB_KEYBOARD');
  requireAbsoluteLinuxById(paths.pointer, '/dev/input/', 'HM_EXTERNAL_USB_POINTER');
  for (const [name, path] of Object.entries(paths)) {
    await access(path).catch(() => {
      throw new Error(`physical USB ${name} path is unavailable: ${path}`);
    });
    const canonical = await realpath(path);
    if (!canonical.startsWith('/dev/')) throw new Error(`${name} did not resolve under /dev`);
  }
}

async function writeUnavailable(runRoot, mode, options, reason) {
  const value = {
    schemaVersion: '2.0.0',
    runId: options.runId,
    sequence: mode.sequence,
    profileId: mode.profileId,
    groundTruth: 'automation',
    browser: { engine: options.browser },
    status: 'UNAVAILABLE',
    availability: {
      capability: 'physical-usb-hil',
      reason,
    },
  };
  await writeFile(
    join(runRoot, `${mode.profileId}.availability.json`),
    `${JSON.stringify(value, null, 2)}\n`,
  );
  return value;
}

async function assertVirtualPilot(lastMode, options, env) {
  const project = `hmn-ext-${options.runId}-assert`.slice(0, 63);
  const args = [
    ...composeArgs(lastMode, options),
    '-p', project,
    'run', '--rm', '--no-deps',
    '-e', 'HM_EXTERNAL_EXPECT_MODES=2',
    'external-input-assert',
  ];
  await docker(args, { env, timeoutMs: 60_000 });
  await down(project, lastMode, options, env);
}

export async function main(argv = process.argv.slice(2)) {
  const options = parseArgs(argv);
  const modes = process.env.HM_EXTERNAL_VUSB_LADDER === '1' ? VUSB_MODES : MODES;
  const runRoot = join(ARTIFACTS, 'external-input', options.runId);
  await mkdir(runRoot, { recursive: true });
  const outcomes = [];

  if (process.env.HM_EXTERNAL_ONLY_MODE) {
    const mode = modes.find(({ profileId }) => profileId === process.env.HM_EXTERNAL_ONLY_MODE);
    if (!mode) throw new Error('HM_EXTERNAL_ONLY_MODE is not in the selected ladder');
    if (options.domRequired !== undefined && options.domRequired !== mode.domRequired) {
      throw new Error('--dom-required differs from the canonical selected mode');
    }
    const outcome = await runMode(mode, { ...options, keep: process.env.HM_EXTERNAL_SKIP_DOWN === '1' }, runRoot);
    return { runId: options.runId, browser: options.browser, outcome };
  }
  if (options.domRequired !== undefined) {
    throw new Error('--dom-required is valid only with HM_EXTERNAL_ONLY_MODE');
  }

  for (const mode of modes.slice(0, 2)) {
    outcomes.push(await runMode(mode, options, runRoot));
  }

  const assertMode = modes[1];
  await assertVirtualPilot(
    assertMode,
    options,
    modeEnv(
      assertMode,
      options,
      join(runRoot, 'control-m2'),
      join(runRoot, 'score-m2'),
    ),
  );

  let hilReason = '';
  try {
    await hilPreflight(options);
  } catch (error) {
    hilReason = error.message;
  }
  if (hilReason) {
    for (const mode of modes.slice(2)) {
      outcomes.push(await writeUnavailable(runRoot, mode, options, hilReason));
    }
  } else {
    for (const mode of modes.slice(2)) {
      outcomes.push(await runMode(mode, options, runRoot));
    }
  }

  const summary = {
    runId: options.runId,
    browser: options.browser,
    order: modes.map((mode) => mode.profileId),
    outcomes: outcomes.map((outcome, index) => ({
      sequence: index + 1,
      profileId: modes[index].profileId,
      status: outcome.status,
      ...(outcome.availability ? { availability: outcome.availability } : {}),
    })),
    canonicalComplete: outcomes.every((outcome) => outcome.status === 'MEASURED'),
    virtualPilotComplete: outcomes.slice(0, 2).every((outcome) => outcome.status === 'MEASURED'),
  };
  await writeFile(join(runRoot, 'ladder-summary.json'), `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`\n${JSON.stringify(summary, null, 2)}`);
  if (!summary.canonicalComplete) process.exitCode = 3;
  return summary;
}

if (import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main().catch((error) => {
    console.error(`external-input smoke failed: ${error.message || error}`);
    process.exitCode = 1;
  });
}
