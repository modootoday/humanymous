import { verify } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import {
  canonicalJson,
  exactObject,
  MODEL_ID,
  SHA256,
  sha256,
} from './common.mjs';
import { parseStrictJson } from './strict-json.mjs';

const CATALOG_FIELDS = [
  'catalogRevision', 'catalogSha256', 'catalogSignaturePolicyId', 'runtimeImages', 'models',
];
export const RUNTIME_IMAGE_FIELDS = Object.freeze([
  'labCore', 'pki', 'display', 'browserChromium', 'browserChromiumDom',
  'browserChromiumIme', 'browserFirefox', 'browserFirefoxDom',
  'browserFirefoxIme', 'controller', 'lifecycle', 'gateway', 'profile',
]);
const MODEL_FIELDS = [
  'modelId', 'displayName', 'contractVersion', 'authority',
  'imageIndexDigest', 'platform', 'platformManifestDigest', 'configDigest',
  'sourceRevision', 'profileManifestSha256', 'descriptorSetId', 'descriptorContractSha256',
  'protocolContractSha256', 'conformanceSuiteSha256', 'signaturePolicyId',
  'sbomSha256', 'licenseExpression', 'noticeBundleSha256',
  'attestationBundleSha256', 'reviewedAt', 'reviewExpiresAt', 'revoked',
];
const ALLOWED_AUTHORITIES = new Set([
  'project-reference', 'reviewed-comparison', 'untrusted-comparison', 'rejected',
]);

export function catalogContentHash(catalog) {
  const copy = structuredClone(catalog);
  copy.catalogSha256 = '';
  return `sha256:${sha256(canonicalJson(copy))}`;
}

export function validateCatalog(catalog, now = new Date()) {
  exactObject(catalog, CATALOG_FIELDS, 'profile catalog');
  if (!/^\d{4}-\d{2}-\d{2}\.\d+$/.test(catalog.catalogRevision || '')) {
    throw new TypeError('catalog revision is invalid');
  }
  if (catalog.catalogSignaturePolicyId !== 'humanymous-catalog-v1') {
    throw new TypeError('catalog signature policy is not approved');
  }
  if (catalog.catalogSha256 !== catalogContentHash(catalog)) {
    throw new TypeError('catalog canonical hash mismatch');
  }
  exactObject(catalog.runtimeImages, RUNTIME_IMAGE_FIELDS, 'runtime images');
  for (const [service, digest] of Object.entries(catalog.runtimeImages)) {
    if (!SHA256.test(digest || '')) throw new TypeError(`runtime image ${service} is invalid`);
  }
  if (!Array.isArray(catalog.models) || catalog.models.length < 1 || catalog.models.length > 32) {
    throw new TypeError('catalog models must contain 1..32 entries');
  }
  const seen = new Set();
  for (const model of catalog.models) {
    exactObject(model, MODEL_FIELDS, 'catalog model');
    if (!MODEL_ID.test(model.modelId || '') || seen.has(model.modelId)) {
      throw new TypeError('catalog model ID is invalid or duplicated');
    }
    seen.add(model.modelId);
    if (typeof model.displayName !== 'string' || model.displayName.length < 3 || model.displayName.length > 80) {
      throw new TypeError('catalog display name is invalid');
    }
    if (model.contractVersion !== 'humanymous.virtual-usb-profile/v1') {
      throw new TypeError('catalog profile contract version mismatch');
    }
    if (!ALLOWED_AUTHORITIES.has(model.authority)) throw new TypeError('catalog authority is invalid');
    for (const field of [
      'imageIndexDigest', 'platformManifestDigest', 'configDigest',
      'profileManifestSha256',
      'descriptorContractSha256', 'protocolContractSha256',
      'conformanceSuiteSha256', 'sbomSha256', 'noticeBundleSha256',
      'attestationBundleSha256',
    ]) {
      if (!SHA256.test(model[field] || '')) throw new TypeError(`${field} is invalid`);
    }
    if (model.platform !== 'linux/amd64') throw new TypeError('catalog platform is not canonical');
    if (!/^[a-f0-9]{40,64}$/.test(model.sourceRevision || '')) {
      throw new TypeError('catalog source revision is invalid');
    }
    if (model.descriptorSetId !== 'reference-relative-v1' ||
        model.signaturePolicyId !== 'humanymous-release-v1' ||
        model.licenseExpression !== 'Apache-2.0') {
      throw new TypeError('catalog model policy fields are invalid');
    }
    const reviewed = Date.parse(model.reviewedAt);
    const expires = Date.parse(model.reviewExpiresAt);
    if (!Number.isFinite(reviewed) || !Number.isFinite(expires) || expires <= reviewed) {
      throw new TypeError('catalog review interval is invalid');
    }
    if (model.revoked !== false || now.getTime() >= expires) {
      throw new TypeError('catalog model is revoked or expired');
    }
  }
  return Object.freeze(structuredClone(catalog));
}

export async function loadVerifiedCatalog({
  catalogPath,
  signaturePath,
  publicKeyPath,
  now = new Date(),
}) {
  const [raw, signature, publicKey] = await Promise.all([
    readFile(catalogPath),
    readFile(signaturePath, 'utf8'),
    readFile(publicKeyPath),
  ]);
  const decoded = Buffer.from(signature.trim(), 'base64');
  if (decoded.length !== 64 || !verify(null, raw, publicKey, decoded)) {
    throw new TypeError('catalog detached Ed25519 signature is invalid');
  }
  const catalog = validateCatalog(parseStrictJson(raw.toString('utf8'), 'profile catalog'), now);
  return Object.freeze({
    catalog,
    rawSha256: `sha256:${sha256(raw)}`,
  });
}

export function resolveModel(catalog, modelId, { canonical = true } = {}) {
  if (!MODEL_ID.test(modelId || '')) throw new TypeError('selected model ID is invalid');
  const model = catalog.models.find((entry) => entry.modelId === modelId);
  if (!model) throw new TypeError('selected model ID is not allowlisted');
  if (canonical && model.authority !== 'project-reference') {
    throw new TypeError('only a project-reference model is canonical');
  }
  return Object.freeze(structuredClone(model));
}
