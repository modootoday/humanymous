import { createHash, randomBytes } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { createReadStream, createWriteStream } from 'node:fs';
import {
  chmod,
  lstat,
  mkdir,
  readFile,
  rm,
} from 'node:fs/promises';
import { resolve } from 'node:path';
import { pipeline } from 'node:stream/promises';
import { pathToFileURL } from 'node:url';
import { createGzip, constants as zlibConstants } from 'node:zlib';

import {
  inspectRuntimeImages,
  selectedRuntimeBuildPlan,
} from './external-input-vusb-images.mjs';
import {
  cellRuntimeImageKeys,
  createOuterReceipt,
  exactRuntimeImageArguments,
  validateEmbeddedRunnerIdentity,
  validateLocalImageIdentity,
  validateOuterOutputDirectory,
} from '../test/externalinput/kernel-runner/outer.mjs';
import {
  createKernelRunnerSeed,
} from '../test/externalinput/kernel-runner/seed.mjs';
import {
  writeKernelRunnerSourceBundle,
} from '../test/externalinput/kernel-runner/source-bundle.mjs';
import { atomicJson } from '../test/externalinput/vusb/common.mjs';
import { parseStrictJson } from '../test/externalinput/vusb/strict-json.mjs';

const ROOT = resolve(import.meta.dirname, '..');
const COMPOSE = resolve(ROOT, 'deployments/compose/external-input-kernel-runner.yaml');
const RUNNER_TAG = 'humanymous/external-input-kernel-runner:local';
const RUNNER_IDENTITY_PATH =
  '/opt/humanymous-kernel/runner-identity.json';
const MAXIMUM_RUNNER_IMAGE_BYTES = 288 * 1024 * 1024;
const MAXIMUM_IMAGE_ARCHIVE_BYTES = 672 * 1024 * 1024;
const MAXIMUM_SOURCE_BUNDLE_BYTES = 8 * 1024 * 1024;

function usage() {
  return [
    'usage: node scripts/external-input-kernel-runner.mjs',
    '  --model reference-relative-v1 [--browser chromium|firefox]',
    '  [--mode 3v|4v]',
    '  [--strategy-seed <bounded-seed>] [--run-id <bounded-run-id>]',
    '  [--no-build]',
  ].join('\n');
}

export function parseArguments(argv) {
  const options = {
    browser: 'chromium',
    mode: '3v',
  };
  const supplied = new Set();
  for (let index = 0; index < argv.length; index += 1) {
    const name = argv[index];
    if (name === '--help') return Object.freeze({ help: true });
    if (name === '--no-build') {
      if (options.noBuild) {
        throw new TypeError('--no-build was supplied more than once');
      }
      options.noBuild = true;
      continue;
    }
    if (![
      '--model',
      '--browser',
      '--mode',
      '--strategy-seed',
      '--run-id',
    ].includes(name)) {
      throw new TypeError(`unknown kernel runner argument: ${name}`);
    }
    const value = argv[++index];
    if (!value || value.startsWith('--')) {
      throw new TypeError(`${name} requires a value`);
    }
    const key = {
      '--model': 'modelId',
      '--browser': 'browser',
      '--mode': 'mode',
      '--strategy-seed': 'strategySeed',
      '--run-id': 'runId',
    }[name];
    if (supplied.has(key)) {
      throw new TypeError(`${name} was supplied more than once`);
    }
    supplied.add(key);
    options[key] = value;
  }
  if (options.modelId !== 'reference-relative-v1') {
    throw new TypeError('kernel runner requires the allowlisted reference-relative-v1 model');
  }
  if (!['3v', '4v'].includes(options.mode)) {
    throw new TypeError('kernel runner mode must be 3v or 4v');
  }
  if (!['chromium', 'firefox'].includes(options.browser)) {
    throw new TypeError('kernel runner browser must be chromium or firefox');
  }
  if (options.strategySeed &&
      !/^[A-Za-z0-9._:-]{1,128}$/.test(options.strategySeed)) {
    throw new TypeError('kernel runner strategy seed is invalid');
  }
  if (options.runId &&
      !/^[a-z0-9][a-z0-9-]{5,47}$/.test(options.runId)) {
    throw new TypeError('kernel runner run ID is invalid');
  }
  return Object.freeze(options);
}

function executeDocker(arguments_, {
  environment = process.env,
  encoding = 'utf8',
} = {}) {
  return execFileSync(process.env.DOCKER || 'docker', arguments_, {
    cwd: ROOT,
    env: environment,
    encoding,
    stdio: encoding === 'utf8' ? ['ignore', 'pipe', 'inherit'] : ['ignore', 'ignore', 'inherit'],
    windowsHide: true,
  });
}

async function saveCellImages(references, destination) {
  const child = spawn(process.env.DOCKER || 'docker', [
    'image',
    'save',
    '--platform=linux/amd64',
    ...references,
  ], {
    cwd: ROOT,
    env: process.env,
    stdio: ['ignore', 'pipe', 'inherit'],
    windowsHide: true,
  });
  const exited = new Promise((accept, reject) => {
    child.once('error', reject);
    child.once('close', (code, signal) => {
      if (code === 0) {
        accept();
      } else {
        reject(new Error(
          `docker image save failed with code ${code} signal ${signal || 'none'}`,
        ));
      }
    });
  });
  try {
    await Promise.all([
      pipeline(
        child.stdout,
        createGzip({
          level: zlibConstants.Z_BEST_COMPRESSION,
          mtime: 0,
        }),
        createWriteStream(destination, { flags: 'wx', mode: 0o600 }),
      ),
      exited,
    ]);
  } catch (error) {
    child.kill();
    await rm(destination, { force: true });
    throw error;
  }
}

function inspectImage(reference) {
  return parseStrictJson(
    executeDocker(['image', 'inspect', reference, '--format', '{{json .}}']),
    `Docker image inspection ${reference}`,
  );
}

function reclaimBuildIntermediates() {
  executeDocker(
    ['buildx', 'prune', '--all', '--force'],
    { encoding: null },
  );
}

function provisionRuntimeImages(imageKeys) {
  const environment = {
    ...process.env,
    BUILDX_NO_DEFAULT_ATTESTATIONS: '1',
    DOCKER_DEFAULT_PLATFORM: 'linux/amd64',
    SOURCE_DATE_EPOCH: '0',
  };
  for (const [dockerfile, tag, target] of selectedRuntimeBuildPlan(imageKeys)) {
    const arguments_ = [
      'buildx',
      'build',
      '--load',
      '--platform', 'linux/amd64',
      '--provenance=false',
      '--sbom=false',
      '-f', dockerfile,
      '-t', tag,
    ];
    if (target) arguments_.push('--target', target);
    arguments_.push('.');
    executeDocker(arguments_, { environment, encoding: null });
    // The loaded final image is the cell input. BuildKit's intermediate copy is
    // not evidence and must not accumulate across the eight selected images.
    reclaimBuildIntermediates();
  }
}

async function sha256BoundedFile(path, maximumBytes) {
  const stat = await lstat(path);
  if (!stat.isFile() || stat.isSymbolicLink() ||
      stat.size < 1 || stat.size > maximumBytes) {
    throw new TypeError(`${path} is not a bounded regular file`);
  }
  const digest = createHash('sha256');
  let bytes = 0;
  await new Promise((accept, reject) => {
    const stream = createReadStream(path);
    stream.on('data', (chunk) => {
      bytes += chunk.length;
      if (bytes > maximumBytes) {
        stream.destroy(new TypeError(`${path} exceeded its byte budget`));
        return;
      }
      digest.update(chunk);
    });
    stream.on('end', accept);
    stream.on('error', reject);
  });
  if (bytes !== stat.size) throw new TypeError(`${path} changed while hashing`);
  return Object.freeze({
    sha256: `sha256:${digest.digest('hex')}`,
    bytes,
  });
}

function runnerEnvironment({ seedDirectory, outputDirectory, runnerImage }) {
  return {
    ...process.env,
    HM_KERNEL_RUNNER_SEED_DIR: seedDirectory,
    HM_KERNEL_RUNNER_OUTPUT_DIR: outputDirectory,
    HM_KERNEL_RUNNER_IMAGE: runnerImage,
    HM_KERNEL_CPUS: '4',
    HM_KERNEL_MEMORY: '4096M',
  };
}

function composeArguments(project, command) {
  return [
    'compose',
    '--project-directory', resolve(ROOT, 'deployments'),
    '-f', COMPOSE,
    '-p', project,
    ...command,
  ];
}

function assertProjectVolumesAbsent(project) {
  const remaining = executeDocker([
    'volume',
    'ls',
    '--filter', `label=com.docker.compose.project=${project}`,
    '--format', '{{.Name}}',
  ]).trim();
  if (remaining) {
    throw new TypeError(`kernel runner work volume teardown failed: ${remaining}`);
  }
}

async function embeddedRunnerIdentity(runnerDigest, temporaryDirectory) {
  const containerName = `hmn-kr-identity-${process.pid}-${randomBytes(4).toString('hex')}`;
  const destination = resolve(temporaryDirectory, 'runner-identity.json');
  let created = false;
  try {
    executeDocker([
      'create',
      '--name', containerName,
      '--entrypoint', '/bin/true',
      runnerDigest,
    ]);
    created = true;
    executeDocker([
      'cp',
      `${containerName}:${RUNNER_IDENTITY_PATH}`,
      destination,
    ]);
  } finally {
    if (created) {
      executeDocker(['rm', '-f', containerName]);
    }
  }
  const stat = await lstat(destination);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 64 * 1024) {
    throw new TypeError('embedded runner identity is not a bounded regular file');
  }
  return validateEmbeddedRunnerIdentity(parseStrictJson(
    await readFile(destination, 'utf8'),
    'embedded kernel runner identity',
  ));
}

function generatedRunId() {
  const timestamp = new Date().toISOString().replaceAll(/[-:.TZ]/g, '').slice(0, 14);
  return `kernel-${timestamp}-${process.pid}-${randomBytes(3).toString('hex')}`;
}

export async function runKernelRunner(options) {
  const runId = options.runId || generatedRunId();
  const sequence = options.mode === '4v' ? 4 : 3;
  const strategySeed = options.strategySeed || `human-mimic-${runId}`;
  const imageArchiveKeys = cellRuntimeImageKeys(
    options.browser,
    sequence,
  );
  const artifactBase = resolve(
    process.env.HM_KERNEL_RUNNER_ARTIFACT_ROOT ||
    resolve(ROOT, 'deployments/artifacts/kernel-runner'),
  );
  const artifactRoot = resolve(artifactBase, runId);
  const seedDirectory = resolve(artifactRoot, 'seed');
  const outputDirectory = resolve(artifactRoot, 'output');
  await mkdir(artifactBase, { recursive: true });
  await mkdir(artifactRoot);
  await Promise.all([
    mkdir(seedDirectory),
    mkdir(outputDirectory),
  ]);
  await Promise.all([
    chmod(artifactRoot, 0o700),
    chmod(seedDirectory, 0o700),
    chmod(outputDirectory, 0o700),
  ]);

  try {
  if (!options.noBuild) {
    provisionRuntimeImages(imageArchiveKeys);

    const buildProject = `hmn-kr-build-${process.pid}`;
    const initialEnvironment = runnerEnvironment({
      seedDirectory,
      outputDirectory,
      runnerImage: RUNNER_TAG,
    });
    executeDocker(
      composeArguments(buildProject, ['build', 'external-kernel-runner']),
      { environment: initialEnvironment, encoding: null },
    );
    reclaimBuildIntermediates();
  }
  const runnerInspection = inspectImage(RUNNER_TAG);
  const runnerDigest = runnerInspection.Descriptor?.digest;
  validateLocalImageIdentity(runnerDigest, runnerInspection);
  if (!Number.isSafeInteger(runnerInspection.Size) ||
      runnerInspection.Size < 1 ||
      runnerInspection.Size > MAXIMUM_RUNNER_IMAGE_BYTES) {
    throw new TypeError('kernel runner image exceeds its 288 MiB byte budget');
  }
  const identity = await embeddedRunnerIdentity(runnerDigest, artifactRoot);
  const {
    schemaVersion: _identitySchemaVersion,
    ...seedRunnerIdentity
  } = identity;

  const runtimeImages = inspectRuntimeImages({ keys: imageArchiveKeys });
  for (const [name, expectedDigest] of Object.entries(runtimeImages)) {
    const inspected = inspectImage(expectedDigest);
    try {
      validateLocalImageIdentity(expectedDigest, inspected);
    } catch (error) {
      throw new TypeError(`runtime image ${name} failed exact identity validation`, {
        cause: error,
      });
    }
  }

  const bundle = await writeKernelRunnerSourceBundle({
    projectRoot: ROOT,
    destination: resolve(seedDirectory, 'bundle.tar'),
  });
  const imageArchivePath = resolve(seedDirectory, 'images.oci.tar');
  await saveCellImages(
    exactRuntimeImageArguments(runtimeImages, imageArchiveKeys),
    imageArchivePath,
  );
  const imageArchive = await sha256BoundedFile(
    imageArchivePath,
    MAXIMUM_IMAGE_ARCHIVE_BYTES,
  );
  const [profileManifest, protocolContract] = await Promise.all([
    sha256BoundedFile(
      resolve(ROOT, 'deployments/external-input/vusb/profile/virtual-usb-profile.json'),
      1024 * 1024,
    ),
    sha256BoundedFile(
      resolve(ROOT, 'test/externalinput/usb-broker/protocol.mjs'),
      1024 * 1024,
    ),
  ]);

  const seed = createKernelRunnerSeed({
    runId,
    runNonce: randomBytes(32).toString('hex'),
    modelId: options.modelId,
    runtimeImages,
    imageArchiveSha256: imageArchive.sha256,
    imageArchiveBytes: imageArchive.bytes,
    imageArchiveKeys,
    sourceBundleSha256: bundle.sha256,
    sourceBundleBytes: bundle.bytes,
    sourceBundleEntries: bundle.entries,
    runner: {
      imageDigest: runnerDigest,
      ...seedRunnerIdentity,
    },
    projectName: `hmn-${runId}`.slice(0, 63),
    browser: options.browser,
    sequence,
    strategySeed,
    profileManifestSha256: profileManifest.sha256,
    protocolContractSha256: protocolContract.sha256,
    budgets: {
      cpus: 4,
      memoryMiB: 4096,
      deadlineSeconds: 1800,
      outputBytes: 64 * 1024 * 1024,
    },
  });
  const seedPath = resolve(seedDirectory, 'seed.json');
  await atomicJson(seedPath, seed);
  await chmod(seedPath, 0o400);
  await chmod(resolve(seedDirectory, 'bundle.tar'), 0o400);
  await chmod(imageArchivePath, 0o400);
  const seedFile = await sha256BoundedFile(seedPath, 1024 * 1024);

  const runProject = `hmn-kr-${runId}`.slice(0, 63);
  const environment = runnerEnvironment({
    seedDirectory,
    outputDirectory,
    runnerImage: runnerDigest,
  });
  environment.HM_KERNEL_RUNNER_SEED_SHA256 = seedFile.sha256;
  let runFailure;
  try {
    executeDocker(
      composeArguments(runProject, [
        'run', '--rm', '--no-deps', 'external-kernel-runner',
      ]),
      { environment, encoding: null },
    );
  } catch (error) {
    runFailure = error;
  }
  let teardownFailure;
  try {
    executeDocker(
      composeArguments(runProject, [
        'down', '--volumes', '--remove-orphans',
      ]),
      { environment, encoding: null },
    );
    assertProjectVolumesAbsent(runProject);
  } catch (error) {
    teardownFailure = error;
  }
  if (runFailure || teardownFailure) {
    throw new AggregateError(
      [runFailure, teardownFailure].filter(Boolean),
      runFailure
        ? 'kernel guest execution failed'
        : 'kernel work volume teardown failed',
    );
  }
  const [seedAfter, imageArchiveAfter, bundleAfter] = await Promise.all([
    sha256BoundedFile(seedPath, 1024 * 1024),
    sha256BoundedFile(imageArchivePath, MAXIMUM_IMAGE_ARCHIVE_BYTES),
    sha256BoundedFile(
      resolve(seedDirectory, 'bundle.tar'),
      MAXIMUM_SOURCE_BUNDLE_BYTES,
    ),
  ]);
  if (seedAfter.sha256 !== seedFile.sha256 ||
      imageArchiveAfter.sha256 !== imageArchive.sha256 ||
      imageArchiveAfter.bytes !== imageArchive.bytes ||
      bundleAfter.sha256 !== bundle.sha256 ||
      bundleAfter.bytes !== bundle.bytes) {
    throw new TypeError('kernel seed artifacts changed during guest execution');
  }
  const output = await validateOuterOutputDirectory(outputDirectory, {
    seed,
    seedSha256: seedFile.sha256,
  });
  const outer = createOuterReceipt({
    runId,
    modelId: options.modelId,
    runnerImageDigest: runnerDigest,
    seedSha256: seedFile.sha256,
    imageArchiveSha256: imageArchive.sha256,
    runnerReceiptSha256: output.runnerReceiptSha256,
    guestTerminalSha256: output.guestTerminalSha256,
    measurementTerminalSha256: output.guest.terminalSha256,
    browserWasmSha256: output.guest.browserWasmSha256,
    cell: output.guest.cell,
    strategy: output.guest.strategy,
    coreMeasurement: {
      scorer: output.score.scorer,
      scoreRecomputed: output.score.scoreRecomputed,
      riskScore: output.score.riskScore,
      verdict: output.score.verdict,
    },
  });
  await atomicJson(resolve(outputDirectory, 'outer.json'), outer);
  return Object.freeze({
    status: 'PASS',
    runId,
    mode: options.mode,
    strategy: seed.strategy,
    artifactRoot,
    outer,
  });
  } finally {
    await Promise.all([
      rm(resolve(seedDirectory, 'images.oci.tar'), { force: true }),
      rm(resolve(seedDirectory, 'bundle.tar'), { force: true }),
    ]);
  }
}

async function main() {
  const options = parseArguments(process.argv.slice(2));
  if (options.help) {
    process.stdout.write(`${usage()}\n`);
    return;
  }
  const result = await runKernelRunner(options);
  process.stdout.write(`${JSON.stringify(result)}\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    const cause = error.cause instanceof Error ? `: ${error.cause.message}` : '';
    process.stderr.write(`${JSON.stringify({
      status: 'FAIL',
      component: 'external-input-kernel-runner',
      reason: `${error.message}${cause}`,
    })}\n`);
    process.exitCode = 1;
  });
}
