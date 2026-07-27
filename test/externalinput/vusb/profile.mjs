import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import {
  boundedInteger,
  exactObject,
  MODEL_ID,
  sha256,
  walkFinalTree,
} from './common.mjs';
import { parseStrictJson } from './strict-json.mjs';

export const PROFILE_CONTRACT = 'humanymous.virtual-usb-profile/v1';
export const PROFILE_PATH = 'profile/virtual-usb-profile.json';
const TOP_FIELDS = [
  'contractVersion', 'modelId', 'descriptorSetId', 'protocolVersion',
  'usbOrigin', 'physicalCapable', 'limits',
];
const LIMIT_FIELDS = [
  'maxReportsPerSecond', 'maxRelativeStep', 'maxKeyDwellMs',
  'maxPointerDwellMs', 'deadManReleaseMs',
];
const ALLOWED_FILES = new Set([
  'profile',
  PROFILE_PATH,
  'profile/LICENSE',
  'profile/NOTICE',
]);

export function validateProfile(profile) {
  exactObject(profile, TOP_FIELDS, 'virtual USB profile');
  if (profile.contractVersion !== PROFILE_CONTRACT) throw new TypeError('profile contract version mismatch');
  if (!MODEL_ID.test(profile.modelId || '')) throw new TypeError('profile model ID is invalid');
  if (profile.descriptorSetId !== 'reference-relative-v1') {
    throw new TypeError('only the fixed relative-pointer descriptor set is allowed');
  }
  if (profile.protocolVersion !== '1.0.0') throw new TypeError('profile protocol version mismatch');
  if (profile.usbOrigin !== 'kernel-emulated' || profile.physicalCapable !== false) {
    throw new TypeError('profile must declare kernel emulation and no physical capability');
  }
  exactObject(profile.limits, LIMIT_FIELDS, 'profile limits');
  boundedInteger(profile.limits.maxReportsPerSecond, 1, 120, 'maxReportsPerSecond');
  boundedInteger(profile.limits.maxRelativeStep, 1, 127, 'maxRelativeStep');
  boundedInteger(profile.limits.maxKeyDwellMs, 20, 250, 'maxKeyDwellMs');
  boundedInteger(profile.limits.maxPointerDwellMs, 20, 250, 'maxPointerDwellMs');
  boundedInteger(profile.limits.deadManReleaseMs, 100, 500, 'deadManReleaseMs');
  return Object.freeze(structuredClone(profile));
}

export async function verifyProfileRoot(root) {
  const entries = await walkFinalTree(root);
  for (const { path, stat } of entries) {
    if (!ALLOWED_FILES.has(path)) throw new TypeError(`profile image has forbidden path: /${path}`);
    if (stat.isFile()) {
      if ((stat.mode & 0o111) !== 0) throw new TypeError(`profile file is executable: /${path}`);
      if (stat.size > 64 * 1024) throw new TypeError(`profile file exceeds size cap: /${path}`);
    } else if (!stat.isDirectory()) {
      throw new TypeError(`profile image has a special file: /${path}`);
    }
  }
  for (const required of ['profile', PROFILE_PATH, 'profile/LICENSE', 'profile/NOTICE']) {
    if (!entries.some(({ path }) => path === required)) throw new TypeError(`profile image is missing /${required}`);
  }
  const raw = await readFile(join(root, PROFILE_PATH), 'utf8');
  const profile = validateProfile(parseStrictJson(raw, 'profile manifest'));
  return Object.freeze({
    profile,
    profileManifestSha256: `sha256:${sha256(raw)}`,
  });
}

