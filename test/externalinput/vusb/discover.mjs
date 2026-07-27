import { execFileSync } from 'node:child_process';
import { lstat, readFile, readdir, realpath } from 'node:fs/promises';
import { basename, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { atomicJson, receiptBase } from './common.mjs';

async function nearestDriver(sysfs) {
  let current = sysfs;
  for (let depth = 0; depth < 16; depth += 1) {
    for (const candidate of [join(current, 'driver'), join(current, 'device', 'driver')]) {
      const driver = await realpath(candidate).catch(() => '');
      if (driver) return driver;
    }
    const parent = await realpath(join(current, '..'));
    if (parent === current) break;
    current = parent;
  }
  return '';
}

async function characterEvidence(containerPath, hostPath) {
  const stat = await lstat(containerPath);
  if (!stat.isCharacterDevice()) throw new TypeError(`${hostPath} is not a character device`);
  const deviceHex = execFileSync('stat', ['-Lc', '%t:%T', containerPath], {
    encoding: 'utf8',
  }).trim().toLowerCase();
  const sysfsDevice = sysfsDeviceKey(deviceHex);
  const sysfs = await realpath(`/sys/dev/char/${sysfsDevice}`);
  return Object.freeze({
    hostPath,
    deviceHex,
    sysfsDevice,
    sysfsPath: sysfs,
    driverPath: await nearestDriver(sysfs),
  });
}

export function sysfsDeviceKey(deviceHex) {
  if (!/^[0-9a-f]+:[0-9a-f]+$/i.test(deviceHex || '')) {
    throw new TypeError('character-device identifier is invalid');
  }
  const [major, minor] = deviceHex.split(':').map((part) => Number.parseInt(part, 16));
  return `${major}:${minor}`;
}

async function candidates(directory) {
  const names = await readdir(directory).catch((error) => {
    if (error.code === 'ENOENT') return [];
    throw error;
  });
  return names.sort().map((name) => join(directory, name));
}

async function belongsToRun(path, runId) {
  const target = await realpath(path);
  const stat = await lstat(target);
  if (!stat.isCharacterDevice()) return false;
  const deviceHex = execFileSync('stat', ['-Lc', '%t:%T', target], { encoding: 'utf8' }).trim();
  let current = await realpath(`/sys/dev/char/${sysfsDeviceKey(deviceHex)}`);
  for (let depth = 0; depth < 12; depth += 1) {
    const serial = await readFile(join(current, 'serial'), 'utf8').catch(() => null);
    if (serial?.trim() === runId) return true;
    const parent = join(current, '..');
    const resolved = await realpath(parent);
    if (resolved === current) break;
    current = resolved;
  }
  return false;
}

async function exactById(directory, predicate, runId, label) {
  const matches = [];
  for (const path of await candidates(directory)) {
    if (predicate(basename(path)) && await belongsToRun(path, runId)) matches.push(path);
  }
  if (matches.length !== 1) throw new TypeError(`${label} requires exactly one by-id device, got ${matches.length}`);
  const hostPath = matches[0].replace(/^\/host-dev/, '/dev');
  return characterEvidence(await realpath(matches[0]), hostPath);
}

async function snapshot(runId, hostDevRoot) {
  const gadget = {
    command: await characterEvidence(join(hostDevRoot, 'ttyGS0'), '/dev/ttyGS0'),
    keyboard: await characterEvidence(join(hostDevRoot, 'hidg0'), '/dev/hidg0'),
    pointer: await characterEvidence(join(hostDevRoot, 'hidg1'), '/dev/hidg1'),
  };
  const host = {
    command: await exactById(join(hostDevRoot, 'serial', 'by-id'), () => true, runId, 'host CDC'),
    keyboard: await exactById(
      join(hostDevRoot, 'input', 'by-id'),
      (name) => name.endsWith('-event-kbd'),
      runId,
      'host keyboard',
    ),
    pointer: await exactById(
      join(hostDevRoot, 'input', 'by-id'),
      (name) => name.endsWith('-event-mouse'),
      runId,
      'host pointer',
    ),
  };
  return Object.freeze({ gadget, host });
}

export function validateDriverContract(snapshotValue) {
  const hostCommand = snapshotValue?.host?.command?.driverPath || '';
  const hostKeyboard = snapshotValue?.host?.keyboard?.driverPath || '';
  const hostPointer = snapshotValue?.host?.pointer?.driverPath || '';
  if (!/(?:^|\/)cdc_acm$/.test(hostCommand)) {
    throw new TypeError('host command interface is not bound to cdc_acm');
  }
  for (const [label, driver] of [
    ['keyboard', hostKeyboard],
    ['pointer', hostPointer],
  ]) {
    if (!/(?:^|\/)(?:usbhid|hid-generic)$/.test(driver)) {
      throw new TypeError(`host ${label} interface is not bound to the USB HID stack`);
    }
  }
  const devices = [
    ...Object.values(snapshotValue.gadget || {}),
    ...Object.values(snapshotValue.host || {}),
  ];
  if (devices.length !== 6 ||
      new Set(devices.map(({ sysfsDevice }) => sysfsDevice)).size !== 6) {
    throw new TypeError('virtual USB device identities are not exactly six distinct nodes');
  }
  return true;
}

function stableIdentity(value) {
  return JSON.stringify(value);
}

export async function discoverDevices({
  runId,
  receiptPath,
  hostDevRoot = '/host-dev',
  now = new Date(),
  wait = (durationMs) => new Promise((resolveDelay) => setTimeout(resolveDelay, durationMs)),
}) {
  const first = await snapshot(runId, hostDevRoot);
  await wait(250);
  const second = await snapshot(runId, hostDevRoot);
  if (stableIdentity(first) !== stableIdentity(second)) {
    throw new TypeError('virtual USB topology changed between independent observations');
  }
  validateDriverContract(second);
  const receipt = {
    ...receiptBase('prepare', runId, now),
    stableObservations: 2,
    observationIntervalMs: 250,
    deviceIdentityCount: 6,
    driverContractVerified: true,
    gadget: second.gadget,
    host: second.host,
  };
  await atomicJson(receiptPath, receipt);
  return Object.freeze(receipt);
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  discoverDevices({
    runId: required('HM_VUSB_RUN_ID'),
    receiptPath: required('HM_VUSB_PREPARE_RECEIPT'),
    hostDevRoot: process.env.HM_VUSB_HOST_DEV_ROOT || '/host-dev',
  }).catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-discover',
      code: 'PURITY_FAIL',
      message: error.message,
    })}\n`);
    process.exitCode = 1;
  });
}
