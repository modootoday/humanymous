import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import {
  atomicJson,
  canonicalJson,
  exactObject,
  receiptBase,
  SHA256,
  sha256,
  sha256File,
} from './common.mjs';
import { loadVerifiedCatalog, resolveModel } from './catalog.mjs';
import { verifyReleaseEvidence } from './release-evidence.mjs';
import { parseStrictJson } from './strict-json.mjs';
import { loadLadderManifest, validateAttemptManifest } from './manifest.mjs';

const ATTESTATION_FIELDS = [
  'schemaVersion', 'imageIndexDigest', 'platform', 'platformManifestDigest',
  'configDigest', 'profileManifestSha256', 'sourceRevision', 'builderId',
  'signatureIdentity', 'signatureIssuer', 'sbomSha256', 'licenseSha256',
  'noticeBundleSha256', 'conformanceSuiteSha256', 'vulnerabilityPolicySha256',
  'revocationSnapshotSha256', 'validUntil',
];

function validateAttestation(bundle, model, now) {
  exactObject(bundle, ATTESTATION_FIELDS, 'offline attestation bundle');
  if (bundle.schemaVersion !== 'humanymous.virtual-usb-attestation/v1') {
    throw new TypeError('attestation schema version mismatch');
  }
  for (const field of [
    'imageIndexDigest', 'platformManifestDigest', 'configDigest',
    'profileManifestSha256', 'sbomSha256', 'licenseSha256',
    'noticeBundleSha256', 'conformanceSuiteSha256',
    'vulnerabilityPolicySha256', 'revocationSnapshotSha256',
  ]) {
    if (!SHA256.test(bundle[field] || '')) throw new TypeError(`${field} is invalid`);
  }
  for (const field of [
    'imageIndexDigest', 'platformManifestDigest', 'configDigest',
    'sbomSha256', 'noticeBundleSha256', 'conformanceSuiteSha256',
  ]) {
    if (bundle[field] !== model[field]) throw new TypeError(`attestation ${field} mismatch`);
  }
  if (bundle.platform !== model.platform ||
      bundle.profileManifestSha256 !== model.profileManifestSha256 ||
      bundle.sourceRevision !== model.sourceRevision) {
    throw new TypeError('attestation platform, source, or profile digest mismatch');
  }
  if (bundle.builderId !== 'https://github.com/modootoday/humanymous/.github/workflows/release.yml' ||
      bundle.signatureIdentity !== 'https://github.com/modootoday/humanymous/.github/workflows/release.yml@refs/heads/main' ||
      bundle.signatureIssuer !== 'https://token.actions.githubusercontent.com') {
    throw new TypeError('attestation builder or signature identity is not approved');
  }
  if (now.getTime() >= Date.parse(bundle.validUntil)) throw new TypeError('offline attestation bundle expired');
  return Object.freeze(structuredClone(bundle));
}

export async function admitVirtualUsbProfile({
  catalogPath,
  catalogSignaturePath,
  catalogPublicKeyPath,
  modelId,
  attestationPath,
  sbomPath,
  vulnerabilityPolicyPath,
  revocationsPath,
  ladderManifestPath,
  attemptManifestPath,
  receiptPath,
  runId,
  now = new Date(),
}) {
  const verifiedCatalog = await loadVerifiedCatalog({
    catalogPath,
    signaturePath: catalogSignaturePath,
    publicKeyPath: catalogPublicKeyPath,
    now,
  });
  const model = resolveModel(verifiedCatalog.catalog, modelId);
  const attestationRaw = await readFile(attestationPath, 'utf8');
  if (`sha256:${sha256(attestationRaw)}` !== model.attestationBundleSha256) {
    throw new TypeError('offline attestation bundle digest mismatch');
  }
  const attestation = validateAttestation(
    parseStrictJson(attestationRaw, 'offline attestation bundle'),
    model,
    now,
  );
  const releaseEvidence = await verifyReleaseEvidence({
    sbomPath,
    vulnerabilityPolicyPath,
    revocationsPath,
    attestation,
    model,
    runtimeImages: verifiedCatalog.catalog.runtimeImages,
    now,
  });
  const ladder = await loadLadderManifest(ladderManifestPath);
  const attemptRaw = await readFile(attemptManifestPath, 'utf8');
  const attempt = validateAttemptManifest(
    parseStrictJson(attemptRaw, 'attempt manifest'),
    ladder.value,
  );
  if (ladder.value.modelId !== model.modelId ||
      ladder.value.catalogSha256 !== verifiedCatalog.catalog.catalogSha256 ||
      canonicalJson(ladder.value.runtimeImages) !==
        canonicalJson(verifiedCatalog.catalog.runtimeImages) ||
      attempt.ladderManifestSha256 !== ladder.sha256 ||
      attempt.runId !== runId) {
    throw new TypeError('attempt manifest is not bound to this admission');
  }
  const receipt = {
    ...receiptBase('admission', runId, now),
    canonical: true,
    modelId: model.modelId,
    authority: model.authority,
    catalogRevision: verifiedCatalog.catalog.catalogRevision,
    catalogSha256: verifiedCatalog.catalog.catalogSha256,
    catalogFileSha256: verifiedCatalog.rawSha256,
    catalogSignaturePolicyId: verifiedCatalog.catalog.catalogSignaturePolicyId,
    runtimeImages: verifiedCatalog.catalog.runtimeImages,
    imageIndexDigest: model.imageIndexDigest,
    platform: model.platform,
    platformManifestDigest: model.platformManifestDigest,
    configDigest: model.configDigest,
    profileManifestSha256: model.profileManifestSha256,
    descriptorSetId: model.descriptorSetId,
    protocolContractSha256: model.protocolContractSha256,
    attestationBundleSha256: model.attestationBundleSha256,
    ladderManifestSha256: ladder.sha256,
    attemptManifestSha256: `sha256:${sha256(attemptRaw)}`,
    selectedBrowserImageDigest: attempt.selectedBrowserImageDigest,
    ...releaseEvidence,
    validatorSourceSha256: `sha256:${await sha256File(new URL(import.meta.url))}`,
  };
  if (receiptPath) await atomicJson(receiptPath, receipt);
  return Object.freeze(receipt);
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

export async function main() {
  await admitVirtualUsbProfile({
    catalogPath: resolve(required('HM_VUSB_CATALOG')),
    catalogSignaturePath: resolve(required('HM_VUSB_CATALOG_SIGNATURE')),
    catalogPublicKeyPath: resolve(required('HM_VUSB_CATALOG_PUBLIC_KEY')),
    modelId: required('HM_VUSB_MODEL_ID'),
    attestationPath: resolve(required('HM_VUSB_ATTESTATION_BUNDLE')),
    sbomPath: resolve(required('HM_VUSB_SBOM')),
    vulnerabilityPolicyPath: resolve(required('HM_VUSB_VULNERABILITY_POLICY')),
    revocationsPath: resolve(required('HM_VUSB_REVOCATIONS')),
    ladderManifestPath: resolve(required('HM_VUSB_LADDER_MANIFEST')),
    attemptManifestPath: resolve(required('HM_VUSB_ATTEMPT_MANIFEST')),
    receiptPath: resolve(required('HM_VUSB_ADMISSION_RECEIPT')),
    runId: required('HM_VUSB_RUN_ID'),
  });
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-admission',
      code: 'ADMISSION_REJECTED',
      message: error.message,
    })}\n`);
    process.exitCode = 2;
  });
}
