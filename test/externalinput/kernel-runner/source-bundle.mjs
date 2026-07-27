import { createHash } from 'node:crypto';
import {
  lstat,
  open,
  readdir,
  readFile,
} from 'node:fs/promises';
import { join, relative, sep } from 'node:path';

const MAXIMUM_ENTRIES = 4096;
const MAXIMUM_FILE_BYTES = 16 * 1024 * 1024;
const MAXIMUM_BUNDLE_BYTES = 8 * 1024 * 1024;

const EXACT_FILES = Object.freeze([
  'configs/dev.env',
  'deployments/compose.yaml',
  'deployments/compose/bots.yaml',
  'deployments/compose/defenders.yaml',
  'deployments/compose/external-input-bots.yaml',
  'deployments/compose/external-input-dom.yaml',
  'deployments/compose/external-input-firefox.yaml',
  'deployments/compose/external-input-vusb.yaml',
  'deployments/compose/external-input-vusb-manifest.yaml',
  'deployments/compose/networks.yaml',
  'deployments/bots/external-input-run.sh',
  'test/e2e/external-input-runner.mjs',
  'test/externalinput/kernel-runner/runtime-images.mjs',
  'test/externalinput/kernel-runner/seed.mjs',
]);

const DIRECTORY_RULES = Object.freeze([
  {
    root: 'deployments/external-input',
    include: (path) => !path.startsWith('deployments/external-input/kernel-runner/'),
  },
  {
    root: 'test/externalinput',
    include: (path) => !path.startsWith('test/externalinput/kernel-runner/'),
  },
]);

const FILE_RULES = Object.freeze([
  {
    root: 'scripts',
    include: (path) =>
      /^scripts\/(?:assert-)?external-input(?:-[a-z0-9-]+)?\.(?:mjs|sh)$/.test(path) &&
      !/^scripts\/external-input-kernel-runner\.(?:mjs|sh)$/.test(path),
  },
  {
    root: 'test/redteam',
    include: (path) => /^test\/redteam\/external_input_[a-z0-9_]+\.mjs$/.test(path),
  },
]);

function portable(path) {
  return path.split(sep).join('/');
}

async function collectDirectory(projectRoot, rule, found) {
  const absoluteRoot = join(projectRoot, ...rule.root.split('/'));
  async function visit(directory) {
    for (const entry of (await readdir(directory, { withFileTypes: true }))
      .sort((left, right) => left.name.localeCompare(right.name))) {
      const absolute = join(directory, entry.name);
      const path = portable(relative(projectRoot, absolute));
      if (entry.isDirectory()) {
        await visit(absolute);
      } else if (entry.isFile()) {
        if (rule.include(path)) found.add(path);
      } else {
        throw new TypeError(`source bundle contains a special entry: ${path}`);
      }
    }
  }
  await visit(absoluteRoot);
}

export async function collectKernelRunnerSource(projectRoot) {
  const found = new Set(EXACT_FILES);
  for (const rule of [...DIRECTORY_RULES, ...FILE_RULES]) {
    await collectDirectory(projectRoot, rule, found);
  }
  const paths = [...found].sort();
  if (paths.length < 1 || paths.length > MAXIMUM_ENTRIES) {
    throw new TypeError('kernel source bundle entry count is invalid');
  }
  for (const path of paths) {
    if (path.startsWith('/') || path.includes('\\') ||
        path.split('/').some((part) => part === '' || part === '.' || part === '..')) {
      throw new TypeError(`kernel source bundle path is unsafe: ${path}`);
    }
    const stat = await lstat(join(projectRoot, ...path.split('/')));
    if (!stat.isFile() || stat.isSymbolicLink() ||
        stat.size > MAXIMUM_FILE_BYTES) {
      throw new TypeError(`kernel source bundle file is invalid: ${path}`);
    }
  }
  return Object.freeze(paths);
}

function octal(value, width) {
  const text = value.toString(8);
  if (text.length > width - 1) throw new TypeError('ustar numeric field overflow');
  return `${text.padStart(width - 1, '0')}\0`;
}

function tarPath(path) {
  const raw = Buffer.from(path);
  if (raw.length <= 100) return { name: path, prefix: '' };
  const separators = [...path.matchAll(/\//g)].map((match) => match.index).reverse();
  for (const offset of separators) {
    const prefix = path.slice(0, offset);
    const name = path.slice(offset + 1);
    if (Buffer.byteLength(prefix) <= 155 && Buffer.byteLength(name) <= 100) {
      return { name, prefix };
    }
  }
  throw new TypeError(`kernel source path exceeds ustar bounds: ${path}`);
}

function field(header, offset, width, value) {
  const raw = Buffer.from(value);
  if (raw.length > width) throw new TypeError('ustar text field overflow');
  raw.copy(header, offset);
}

function header(path, stat) {
  const value = Buffer.alloc(512);
  const split = tarPath(path);
  field(value, 0, 100, split.name);
  field(value, 100, 8, octal((stat.mode & 0o111) === 0 ? 0o644 : 0o755, 8));
  field(value, 108, 8, octal(0, 8));
  field(value, 116, 8, octal(0, 8));
  field(value, 124, 12, octal(stat.size, 12));
  field(value, 136, 12, octal(0, 12));
  value.fill(0x20, 148, 156);
  value[156] = 0x30;
  field(value, 257, 6, 'ustar\0');
  field(value, 263, 2, '00');
  field(value, 265, 32, 'root');
  field(value, 297, 32, 'root');
  field(value, 345, 155, split.prefix);
  const checksum = value.reduce((sum, byte) => sum + byte, 0);
  field(value, 148, 8, `${checksum.toString(8).padStart(6, '0')}\0 `);
  return value;
}

export async function writeKernelRunnerSourceBundle({
  projectRoot,
  destination,
  paths,
}) {
  const selectedPaths = paths || await collectKernelRunnerSource(projectRoot);
  const output = await open(destination, 'wx', 0o600);
  const digest = createHash('sha256');
  let bytes = 0;
  const append = async (value) => {
    bytes += value.length;
    if (bytes > MAXIMUM_BUNDLE_BYTES) {
      throw new TypeError('kernel source bundle exceeds its byte budget');
    }
    digest.update(value);
    await output.write(value);
  };
  try {
    for (const path of selectedPaths) {
      const absolute = join(projectRoot, ...path.split('/'));
      const stat = await lstat(absolute);
      if (!stat.isFile() || stat.isSymbolicLink() ||
          stat.size > MAXIMUM_FILE_BYTES) {
        throw new TypeError(`kernel source changed during bundling: ${path}`);
      }
      const content = await readFile(absolute);
      if (content.length !== stat.size) {
        throw new TypeError(`kernel source size changed during bundling: ${path}`);
      }
      await append(header(path, stat));
      await append(content);
      const padding = (512 - (content.length % 512)) % 512;
      if (padding) await append(Buffer.alloc(padding));
    }
    await append(Buffer.alloc(1024));
  } finally {
    await output.close();
  }
  return Object.freeze({
    path: destination,
    sha256: `sha256:${digest.digest('hex')}`,
    bytes,
    entries: selectedPaths.length,
  });
}
