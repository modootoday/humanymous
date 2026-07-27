import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { modeFor } from './contracts.mjs';
import {
  createPreProjectProof,
  evaluateRuntimePurity,
  validatePreProjectProof,
  validateResetProof,
} from './runtime-purity.mjs';
import { validatePixelManifest } from './pixel-locator.mjs';
import { parseStrictJson } from './vusb/strict-json.mjs';

const MAX_COMPOSE_BYTES = 512 * 1024;
const MAX_PROBE_BYTES = 128 * 1024;

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

async function boundedRead(path, maximum, label) {
  const raw = await readFile(path);
  if (raw.length > maximum) throw new TypeError(`${label} exceeds its byte bound`);
  return raw.toString('utf8');
}

async function hashFile(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex');
}

async function hashTree(root) {
  const names = [];
  async function visit(directory, prefix = '') {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory()) await visit(join(directory, entry.name), relative);
      else if (entry.isFile()) names.push(relative);
      else throw new TypeError('evaluator DOM tree contains a non-regular entry');
      if (names.length > 256) throw new TypeError('evaluator DOM tree exceeds its file bound');
    }
  }
  await visit(root);
  names.sort();
  const hash = createHash('sha256');
  for (const name of names) {
    hash.update(name);
    hash.update('\0');
    hash.update(await readFile(join(root, ...name.split('/'))));
    hash.update('\0');
  }
  return hash.digest('hex');
}

async function exclusiveJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o640,
  });
}

async function exclusiveText(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, value, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o640,
  });
}

async function readPreProjectInputs({
  runId,
  profileId,
  composeProject,
  composePath,
  containerInventoryPath,
  networkInventoryPath,
  volumeInventoryPath,
}) {
  const [composeRaw, containersRaw, networksRaw, volumesRaw] = await Promise.all([
    boundedRead(composePath, MAX_COMPOSE_BYTES, 'Compose config'),
    boundedRead(containerInventoryPath, MAX_PROBE_BYTES, 'pre-project container inventory'),
    boundedRead(networkInventoryPath, MAX_PROBE_BYTES, 'pre-project network inventory'),
    boundedRead(volumeInventoryPath, MAX_PROBE_BYTES, 'pre-project volume inventory'),
  ]);
  const config = parseStrictJson(composeRaw, 'Compose config');
  if (config.name !== composeProject) {
    throw new TypeError('Compose config project does not match the selected project');
  }
  const proof = createPreProjectProof({
    runId,
    profileId,
    composeProject,
    composeRaw,
    containersRaw,
    networksRaw,
    volumesRaw,
  });
  return { composeRaw, proof };
}

export async function publishPreProjectProof(options) {
  const { proof } = await readPreProjectInputs(options);
  await exclusiveJson(options.preProjectProofPath, proof);
  return proof;
}

export async function evaluateRuntimeFiles({
  runId,
  profileId,
  engine,
  composePath,
  browserProbePath,
  displayProbePath,
  purityPath,
  runtimeEvidencePath,
  composeProject,
  containerInventoryPath,
  networkInventoryPath,
  volumeInventoryPath,
  preProjectProofPath,
  visualManifestSource,
  visualManifestPath,
  resetProofPath,
}) {
  const mode = modeFor(profileId);
  const [{ composeRaw, proof }, savedProofRaw, browserRaw, displayRaw, visualManifestRaw] =
    await Promise.all([
      readPreProjectInputs({
        runId,
        profileId,
        composeProject,
        composePath,
        containerInventoryPath,
        networkInventoryPath,
        volumeInventoryPath,
      }),
      boundedRead(preProjectProofPath, MAX_PROBE_BYTES, 'pre-project proof'),
      boundedRead(browserProbePath, MAX_PROBE_BYTES, 'browser probe'),
      boundedRead(displayProbePath, MAX_PROBE_BYTES, 'display probe'),
      boundedRead(visualManifestSource, MAX_PROBE_BYTES, 'visual manifest'),
    ]);
  const savedProof = validatePreProjectProof(
    parseStrictJson(savedProofRaw, 'pre-project proof'),
    { runId, profileId, composeProject },
  );
  if (JSON.stringify(savedProof) !== JSON.stringify(proof)) {
    throw new TypeError('pre-project proof does not bind the current raw inputs');
  }
  validatePixelManifest(parseStrictJson(visualManifestRaw, 'visual manifest'));
  let expectedDom;
  if (mode.domRequired) {
    const root = '/app/test/externalinput/dom-observer';
    expectedDom = {
      extensionSha256: await hashTree(join(root, 'extension')),
      manifestSha256: await hashFile(join(root, 'native-host-manifest.json')),
    };
  }
  const result = evaluateRuntimePurity({
    config: parseStrictJson(composeRaw, 'Compose config'),
    browserProbe: parseStrictJson(browserRaw, 'browser runtime probe'),
    displayProbe: parseStrictJson(displayRaw, 'display runtime probe'),
    profileId,
    runId,
    engine,
    expectedDom,
    composeRaw,
  });
  await exclusiveJson(purityPath, result.purity);
  await exclusiveJson(runtimeEvidencePath, result.runtimeEvidence);
  await exclusiveText(visualManifestPath, visualManifestRaw);
  const resetProof = validateResetProof({
    schemaVersion: 'humanymous.external-input-reset-proof/v1',
    runId,
    profileId,
    composeProject,
    freshBrowserProfile: true,
    freshDetectorState: true,
    freshTaskState: true,
    composeConfigSha256: proof.composeConfigSha256,
    preProjectInventorySha256: proof.inventorySha256,
    visualManifestSha256:
      `sha256:${createHash('sha256').update(visualManifestRaw).digest('hex')}`,
  }, { runId, profileId, composeProject });
  await exclusiveJson(resetProofPath, resetProof);
  return { ...result, resetProof };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const options = {
    runId: required('HM_EXTERNAL_RUN_ID'),
    profileId: required('HM_EXTERNAL_MODE'),
    engine: required('HM_EXTERNAL_BROWSER'),
    composeProject: required('HM_EXTERNAL_COMPOSE_PROJECT'),
    composePath: required('HM_EXTERNAL_COMPOSE_CONFIG_PATH'),
    containerInventoryPath: required('HM_EXTERNAL_PREPROJECT_CONTAINERS'),
    networkInventoryPath: required('HM_EXTERNAL_PREPROJECT_NETWORKS'),
    volumeInventoryPath: required('HM_EXTERNAL_PREPROJECT_VOLUMES'),
    preProjectProofPath: required('HM_EXTERNAL_PREPROJECT_PROOF'),
  };
  const phase = process.env.HM_EXTERNAL_RUNTIME_EVALUATOR_PHASE || 'final';
  const operation = phase === 'preflight'
    ? publishPreProjectProof(options)
    : phase === 'final'
      ? evaluateRuntimeFiles({
          ...options,
          browserProbePath: required('HM_EXTERNAL_BROWSER_PROBE_PATH'),
          displayProbePath: required('HM_EXTERNAL_DISPLAY_PROBE_PATH'),
          purityPath: required('HM_EXTERNAL_PURITY_PATH'),
          runtimeEvidencePath: required('HM_EXTERNAL_RUNTIME_EVIDENCE'),
          visualManifestSource: required('HM_EXTERNAL_VISUAL_MANIFEST_SOURCE'),
          visualManifestPath: required('HM_EXTERNAL_VISUAL_MANIFEST'),
          resetProofPath: required('HM_EXTERNAL_RESET_PROOF'),
        })
      : Promise.reject(new TypeError('runtime evaluator phase must be preflight or final'));
  operation.catch((error) => {
    process.stderr.write(`runtime purity evaluator failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
