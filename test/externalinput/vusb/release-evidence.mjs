import { readFile } from 'node:fs/promises';
import { exactObject, SHA256, sha256 } from './common.mjs';
import { parseStrictJson } from './strict-json.mjs';

const RELEASE_FILE_FIELDS = Object.freeze({
  sbom: 'sbomSha256',
  vulnerabilityPolicy: 'vulnerabilityPolicySha256',
  revocations: 'revocationSnapshotSha256',
});

function digest(raw) {
  return `sha256:${sha256(raw)}`;
}

function validateSpdx(value) {
  exactObject(value, [
    'spdxVersion', 'dataLicense', 'SPDXID', 'name', 'documentNamespace',
    'creationInfo', 'packages',
  ], 'SPDX document');
  if (value.spdxVersion !== 'SPDX-2.3' ||
      value.dataLicense !== 'CC0-1.0' ||
      value.SPDXID !== 'SPDXRef-DOCUMENT' ||
      !Array.isArray(value.packages) ||
      value.packages.length !== 1) {
    throw new TypeError('SPDX document is not the bounded reference-profile inventory');
  }
  const profilePackage = value.packages[0];
  if (profilePackage?.primaryPackagePurpose !== 'DATA' ||
      profilePackage?.licenseDeclared !== 'Apache-2.0' ||
      profilePackage?.licenseConcluded !== 'Apache-2.0' ||
      profilePackage?.filesAnalyzed !== false) {
    throw new TypeError('SPDX profile package policy mismatch');
  }
}

function validateVulnerabilityPolicy(value, now) {
  exactObject(value, [
    'schemaVersion', 'profileContentType', 'denyKnownExploited',
    'maximumCritical', 'maximumHigh', 'scannerDatabaseSnapshot', 'note',
  ], 'vulnerability policy');
  if (value.schemaVersion !== 'humanymous.virtual-usb-vulnerability-policy/v1' ||
      value.profileContentType !== 'data-only' ||
      value.denyKnownExploited !== true ||
      value.maximumCritical !== 0 ||
      value.maximumHigh !== 0 ||
      !/^\d{4}-\d{2}-\d{2}$/.test(value.scannerDatabaseSnapshot || '')) {
    throw new TypeError('vulnerability policy is not fail-closed');
  }
  const snapshot = Date.parse(`${value.scannerDatabaseSnapshot}T23:59:59.999Z`);
  if (!Number.isFinite(snapshot) || snapshot > now.getTime()) {
    throw new TypeError('vulnerability scanner database snapshot is invalid or future-dated');
  }
}

function validateRevocations(value, model, runtimeImages) {
  exactObject(value, [
    'schemaVersion', 'revision', 'revokedModelIds', 'revokedImageDigests',
  ], 'revocation snapshot');
  if (value.schemaVersion !== 'humanymous.virtual-usb-revocations/v1' ||
      !/^\d{4}-\d{2}-\d{2}\.\d+$/.test(value.revision || '') ||
      !Array.isArray(value.revokedModelIds) ||
      !Array.isArray(value.revokedImageDigests) ||
      value.revokedModelIds.some((id) => typeof id !== 'string') ||
      value.revokedImageDigests.some((valueDigest) => !SHA256.test(valueDigest || ''))) {
    throw new TypeError('revocation snapshot is invalid');
  }
  if (value.revokedModelIds.includes(model.modelId)) {
    throw new TypeError('selected virtual USB model is revoked');
  }
  const selectedDigests = new Set([
    model.imageIndexDigest,
    model.platformManifestDigest,
    model.configDigest,
    ...Object.values(runtimeImages),
  ]);
  if (value.revokedImageDigests.some((valueDigest) => selectedDigests.has(valueDigest))) {
    throw new TypeError('selected virtual USB image is revoked');
  }
}

export async function verifyReleaseEvidence({
  sbomPath,
  vulnerabilityPolicyPath,
  revocationsPath,
  attestation,
  model,
  runtimeImages,
  now = new Date(),
}) {
  const raw = Object.fromEntries(await Promise.all([
    ['sbom', sbomPath],
    ['vulnerabilityPolicy', vulnerabilityPolicyPath],
    ['revocations', revocationsPath],
  ].map(async ([name, path]) => [name, await readFile(path, 'utf8')])));
  for (const [name, field] of Object.entries(RELEASE_FILE_FIELDS)) {
    if (digest(raw[name]) !== attestation[field]) {
      throw new TypeError(`${name} release evidence digest mismatch`);
    }
  }
  const sbom = parseStrictJson(raw.sbom, 'SPDX document');
  const vulnerabilityPolicy = parseStrictJson(
    raw.vulnerabilityPolicy,
    'vulnerability policy',
  );
  const revocations = parseStrictJson(raw.revocations, 'revocation snapshot');
  validateSpdx(sbom);
  validateVulnerabilityPolicy(vulnerabilityPolicy, now);
  validateRevocations(revocations, model, runtimeImages);
  return Object.freeze({
    sbomSha256: attestation.sbomSha256,
    vulnerabilityPolicySha256: attestation.vulnerabilityPolicySha256,
    revocationSnapshotSha256: attestation.revocationSnapshotSha256,
    revocationRevision: revocations.revision,
    scannerDatabaseSnapshot: vulnerabilityPolicy.scannerDatabaseSnapshot,
  });
}
