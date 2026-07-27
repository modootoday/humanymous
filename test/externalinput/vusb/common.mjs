import { createHash } from 'node:crypto';
import { link, lstat, readFile, readdir, rename, unlink, writeFile } from 'node:fs/promises';
import { dirname, join, relative, sep } from 'node:path';

export const SHA256 = /^sha256:[a-f0-9]{64}$/;
export const HEX_SHA256 = /^[a-f0-9]{64}$/;
export const MODEL_ID = /^[a-z][a-z0-9-]{2,63}$/;
export const RUN_ID = /^[a-z0-9][a-z0-9-]{5,63}$/;

export function exactObject(value, fields, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  const expected = new Set(fields);
  for (const field of Object.keys(value)) {
    if (!expected.has(field)) throw new TypeError(`${label} has unknown field: ${field}`);
  }
  for (const field of expected) {
    if (!Object.hasOwn(value, field)) throw new TypeError(`${label} is missing field: ${field}`);
  }
  return value;
}

export function boundedInteger(value, minimum, maximum, label) {
  if (!Number.isInteger(value) || value < minimum || value > maximum) {
    throw new TypeError(`${label} must be an integer from ${minimum} through ${maximum}`);
  }
  return value;
}

export function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

export function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

export async function sha256File(path, maximumBytes = 1024 * 1024) {
  const stat = await lstat(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > maximumBytes) {
    throw new TypeError(`${path} must be a bounded regular file`);
  }
  return sha256(await readFile(path));
}

export async function atomicJson(path, value, { replace = false } = {}) {
  const temporary = `${path}.tmp-${process.pid}`;
  await writeFile(temporary, `${JSON.stringify(value, null, 2)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o600,
  });
  try {
    if (replace) {
      await rename(temporary, path);
    } else {
      // link(2) is an atomic no-clobber publication on the same filesystem.
      // Receipts therefore cannot be silently rewritten by a retry.
      await link(temporary, path);
      await unlink(temporary);
    }
  } catch (error) {
    await unlink(temporary).catch(() => {});
    throw error;
  }
}

export async function walkFinalTree(root, { maximumEntries = 16 } = {}) {
  const found = [];
  async function visit(path) {
    const stat = await lstat(path);
    const rel = relative(root, path).split(sep).join('/');
    if (rel) found.push({ path: rel, stat });
    if (found.length > maximumEntries) throw new TypeError('profile image has too many filesystem entries');
    if (stat.isSymbolicLink()) throw new TypeError(`symbolic link is forbidden: /${rel}`);
    if (stat.nlink > 1 && stat.isFile()) throw new TypeError(`hard link is forbidden: /${rel}`);
    if (stat.isDirectory()) {
      for (const name of (await readdir(path)).sort()) await visit(join(path, name));
    }
  }
  await visit(root);
  return found;
}

export function receiptBase(kind, runId, now = new Date()) {
  if (!RUN_ID.test(runId || '')) throw new TypeError('run ID is invalid');
  return {
    schemaVersion: 'humanymous.virtual-usb-receipt/v1',
    kind,
    runId,
    recordedAt: now.toISOString(),
  };
}

export function assertPathBelow(root, path, label) {
  const rel = relative(root, path);
  if (!rel || rel.startsWith('..') || rel.includes(`..${sep}`)) {
    throw new TypeError(`${label} must be below its bounded root`);
  }
  return path;
}

export function parentDirectory(path) {
  return dirname(path);
}
