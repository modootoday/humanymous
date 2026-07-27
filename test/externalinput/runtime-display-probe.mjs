import { spawnSync } from 'node:child_process';
import { lstat, writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import {
  DISPLAY_PROBE_SCHEMA,
  validateDisplayProbe,
} from './runtime-purity.mjs';

const sleep = (ms) => new Promise((resolveDelay) => setTimeout(resolveDelay, ms));

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

async function exists(path) {
  return lstat(path).then(() => true).catch((error) => {
    if (error.code === 'ENOENT') return false;
    throw error;
  });
}

async function readXtestState() {
  const deadline = Date.now() + 30_000;
  let last = '';
  do {
    const result = spawnSync('xdpyinfo', ['-ext', 'XTEST'], {
      encoding: 'utf8',
      maxBuffer: 256 * 1024,
      windowsHide: true,
      env: process.env,
    });
    last = `${result.stdout || ''}\n${result.stderr || ''}`;
    if (result.status === 0) return /XTEST version/i.test(last);
    await sleep(100);
  } while (Date.now() < deadline);
  throw new Error(`display did not become probeable: ${last.trim().slice(0, 512)}`);
}

export async function probeDisplayRuntime({
  runId,
  profileId,
  destination,
}) {
  const evidence = validateDisplayProbe({
    schemaVersion: DISPLAY_PROBE_SCHEMA,
    runId,
    profileId,
    xtestEnabled: await readXtestState(),
    uinputPresent: await exists('/dev/uinput'),
  }, { runId, profileId });
  await writeFile(destination, `${JSON.stringify(evidence, null, 2)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o600,
  });
  return evidence;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  probeDisplayRuntime({
    runId: required('HM_EXTERNAL_RUN_ID'),
    profileId: required('HM_EXTERNAL_MODE'),
    destination: required('HM_EXTERNAL_DISPLAY_PROBE_PATH'),
  }).catch((error) => {
    process.stderr.write(`display runtime probe failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
