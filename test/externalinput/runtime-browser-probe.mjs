import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { lstat, readFile, readdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { modeFor } from './contracts.mjs';
import {
  BROWSER_PROBE_SCHEMA,
  argvHasProcessType,
  argvHasForbiddenCapability,
  packageInventoryHasAutomationDependency,
  parseListeningSockets,
  sandboxStatusIsBounded,
  validateBrowserProbe,
} from './runtime-purity.mjs';

const MAX_TEXT = 256 * 1024;
const sleep = (ms) => new Promise((resolveDelay) => setTimeout(resolveDelay, ms));

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

async function boundedRead(path, maximum = MAX_TEXT) {
  const value = await readFile(path);
  if (value.length > maximum) throw new TypeError(`${path} exceeds its evidence bound`);
  return value;
}

async function hashFile(path) {
  return createHash('sha256').update(await boundedRead(path)).digest('hex');
}

async function hashTree(root) {
  const names = [];
  async function visit(directory, prefix = '') {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory()) await visit(join(directory, entry.name), relative);
      else if (entry.isFile()) names.push(relative);
      else throw new TypeError('DOM observer tree contains a non-regular entry');
      if (names.length > 256) throw new TypeError('DOM observer tree exceeds its file bound');
    }
  }
  await visit(root);
  names.sort();
  const hash = createHash('sha256');
  for (const name of names) {
    hash.update(name);
    hash.update('\0');
    hash.update(await boundedRead(join(root, ...name.split('/'))));
    hash.update('\0');
  }
  return hash.digest('hex');
}

function fixedExec(command, args) {
  return execFileSync(command, args, {
    encoding: 'utf8',
    maxBuffer: MAX_TEXT,
    windowsHide: true,
  });
}

async function processArgv(pid = 1) {
  const raw = await boundedRead(`/proc/${pid}/cmdline`, 64 * 1024);
  return raw.toString('utf8').split('\0').filter(Boolean).slice(0, 128);
}

async function findSandboxStatus(engine) {
  const deadline = Date.now() + 30_000;
  let lastRendererStatus = '';
  do {
    const pids = (await readdir('/proc'))
      .filter((name) => /^\d+$/.test(name))
      .slice(0, 4096);
    for (const pid of pids) {
      const status = await boundedRead(`/proc/${pid}/status`, 64 * 1024)
        .then((value) => value.toString('utf8'))
        .catch(() => '');
      if (!status) continue;
      if (engine === 'chromium') {
        const argv = await processArgv(pid).catch(() => []);
        if (!argvHasProcessType(argv, 'renderer')) {
          continue;
        }
      } else if (!/^Name:\s+Web Content$/m.test(status)) {
        continue;
      }
      // Firefox names a content process before that child has finished
      // installing its own seccomp filter. Do not publish the transient
      // pre-sandbox status; wait for the engine-specific terminal shape.
      lastRendererStatus = status;
      if (sandboxStatusIsBounded(status, engine)) return status;
    }
    await sleep(100);
  } while (Date.now() < deadline);
  if (lastRendererStatus) return lastRendererStatus;
  throw new Error('browser renderer sandbox process did not become observable');
}

async function exists(path) {
  return lstat(path).then(() => true).catch((error) => {
    if (error.code === 'ENOENT') return false;
    throw error;
  });
}

async function socketExists(path) {
  return lstat(path).then((status) => status.isSocket()).catch((error) => {
    if (error.code === 'ENOENT') return false;
    throw error;
  });
}

async function waitForDomSocket(requiredByMode) {
  if (!requiredByMode) return socketExists('/run/dom-observer/observer.sock');
  const deadline = Date.now() + 30_000;
  do {
    if (await socketExists('/run/dom-observer/observer.sock')) return true;
    await sleep(100);
  } while (Date.now() < deadline);
  return false;
}

export async function probeBrowserRuntime({
  runId,
  profileId,
  engine,
  destination,
}) {
  const mode = modeFor(profileId);
  const binary = engine === 'chromium'
    ? '/usr/lib/chromium/chromium'
    : '/usr/lib/firefox-esr/firefox-esr';
  const argv = await processArgv(1);
  const sandboxStatus = await findSandboxStatus(engine);
  const packages = fixedExec('dpkg-query', ['-W', '-f=${binary:Package}\n']);
  const listeners = [
    ...parseListeningSockets((await boundedRead('/proc/1/net/tcp')).toString('utf8')),
    ...parseListeningSockets((await boundedRead('/proc/1/net/tcp6')).toString('utf8')),
  ];
  const domRoot = '/opt/external-input/dom-observer';
  const domImagePresent = await exists(domRoot);
  const domSocketPresent = await waitForDomSocket(mode.domRequired);
  const domObserverPresent = domImagePresent && domSocketPresent;
  const evidence = validateBrowserProbe({
    schemaVersion: BROWSER_PROBE_SCHEMA,
    runId,
    profileId,
    engine,
    version: fixedExec(binary, ['--version']).trim(),
    binary,
    binarySha256: fixedExec('sha256sum', [binary]).trim().split(/\s+/)[0],
    argv,
    forbiddenArgv: argvHasForbiddenCapability(argv),
    automationDependency: packageInventoryHasAutomationDependency(packages),
    debugPortListening: listeners.length > 0,
    sandboxed: sandboxStatusIsBounded(sandboxStatus, engine),
    sandboxStatus: sandboxStatus
      .split(/\r?\n/)
      .filter((line) => /^(Uid|NSpid|CapEff|NoNewPrivs|Seccomp|Seccomp_filters):/.test(line))
      .slice(0, 16),
    domObserverPresent,
    domImagePresent,
    domSocketPresent,
    domExtensionSha256: domImagePresent
      ? await hashTree(join(domRoot, 'extension'))
      : '',
    domManifestSha256: domImagePresent
      ? await hashFile(join(domRoot, 'native-host-manifest.json'))
      : '',
    uinputPresent: await exists('/dev/uinput'),
  }, { runId, profileId, engine });
  if (evidence.domObserverPresent !== mode.domRequired) {
    throw new TypeError('browser probe DOM presence differs from the canonical mode');
  }
  await writeFile(destination, `${JSON.stringify(evidence, null, 2)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o600,
  });
  return evidence;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  probeBrowserRuntime({
    runId: required('HM_EXTERNAL_RUN_ID'),
    profileId: required('HM_EXTERNAL_MODE'),
    engine: required('HM_EXTERNAL_BROWSER'),
    destination: required('HM_EXTERNAL_BROWSER_PROBE_PATH'),
  }).catch((error) => {
    process.stderr.write(`browser runtime probe failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
