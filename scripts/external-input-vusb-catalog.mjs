// Non-measurement release/provisioning utility. The signing private key is
// supplied by the release environment and is never written into the repository.
import { createPrivateKey, createPublicKey, sign } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import {
  catalogContentHash,
  validateCatalog,
} from '../test/externalinput/vusb/catalog.mjs';
import { SHA256, sha256 } from '../test/externalinput/vusb/common.mjs';
import { validateProfile } from '../test/externalinput/vusb/profile.mjs';
import { parseStrictJson } from '../test/externalinput/vusb/strict-json.mjs';

const ROOT = resolve(import.meta.dirname, '..');
const VUSB = resolve(ROOT, 'deployments/external-input/vusb');

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

async function raw(relative) {
  return readFile(resolve(ROOT, relative));
}

export async function generateCatalog({
  outputDirectory = VUSB,
  signingKeyPath,
  trustRootPath,
  attestationBundlePath,
  sourceRevision,
  runtimeImages,
  profileManifestDigest,
  profileConfigDigest,
  now = new Date(),
  expiresAt = new Date('2027-07-26T00:00:00.000Z'),
}) {
  if (!/^[a-f0-9]{40,64}$/.test(sourceRevision || '')) throw new TypeError('source revision is invalid');
  for (const digest of [
    profileManifestDigest, profileConfigDigest, ...Object.values(runtimeImages),
  ]) {
    if (!SHA256.test(digest || '')) throw new TypeError('runtime/profile image digest is invalid');
  }
  const [
    profileRaw, noticeRaw, conformanceRaw, protocolRaw, lifecycleRaw,
    sbomRaw, vulnerabilityPolicyRaw, revocationsRaw, attestationRawBuffer,
  ] = await Promise.all([
    raw('deployments/external-input/vusb/profile/virtual-usb-profile.json'),
    raw('deployments/external-input/vusb/profile/NOTICE'),
    raw('deployments/external-input/vusb/conformance.json'),
    raw('test/externalinput/usb-broker/protocol.mjs'),
    raw('test/externalinput/vusb/lifecycle.sh'),
    raw('deployments/external-input/vusb/reference.spdx.json'),
    raw('deployments/external-input/vusb/vulnerability-policy.json'),
    raw('deployments/external-input/vusb/revocations.json'),
    readFile(attestationBundlePath),
  ]);
  const profile = validateProfile(parseStrictJson(profileRaw.toString(), 'reference profile'));
  const digest = (value) => `sha256:${sha256(value)}`;
  const attestationRaw = attestationRawBuffer.toString('utf8');
  const attestation = parseStrictJson(attestationRaw, 'release attestation bundle');
  if (attestation.schemaVersion !== 'humanymous.virtual-usb-attestation/v1' ||
      attestation.imageIndexDigest !== profileManifestDigest ||
      attestation.platform !== 'linux/amd64' ||
      attestation.platformManifestDigest !== profileManifestDigest ||
      attestation.configDigest !== profileConfigDigest ||
      attestation.profileManifestSha256 !== digest(profileRaw) ||
      attestation.sourceRevision !== sourceRevision ||
      attestation.builderId !== 'https://github.com/modootoday/humanymous/.github/workflows/release.yml' ||
      attestation.signatureIdentity !== 'https://github.com/modootoday/humanymous/.github/workflows/release.yml@refs/heads/main' ||
      attestation.signatureIssuer !== 'https://token.actions.githubusercontent.com' ||
      attestation.sbomSha256 !== digest(sbomRaw) ||
      attestation.vulnerabilityPolicySha256 !== digest(vulnerabilityPolicyRaw) ||
      attestation.revocationSnapshotSha256 !== digest(revocationsRaw) ||
      attestation.noticeBundleSha256 !== digest(noticeRaw) ||
      attestation.conformanceSuiteSha256 !== digest(conformanceRaw) ||
      Date.parse(attestation.validUntil) !== expiresAt.getTime()) {
    throw new TypeError('release attestation bundle is incomplete, mismatched, or not canonical');
  }
  const date = now.toISOString().slice(0, 10);
  const catalog = {
    catalogRevision: `${date}.1`,
    catalogSha256: '',
    catalogSignaturePolicyId: 'humanymous-catalog-v1',
    runtimeImages,
    models: [{
      modelId: profile.modelId,
      displayName: 'Reference relative-pointer model',
      contractVersion: profile.contractVersion,
      authority: 'project-reference',
      imageIndexDigest: profileManifestDigest,
      platform: 'linux/amd64',
      platformManifestDigest: profileManifestDigest,
      configDigest: profileConfigDigest,
      sourceRevision,
      profileManifestSha256: digest(profileRaw),
      descriptorSetId: profile.descriptorSetId,
      descriptorContractSha256: digest(lifecycleRaw),
      protocolContractSha256: digest(protocolRaw),
      conformanceSuiteSha256: attestation.conformanceSuiteSha256,
      signaturePolicyId: 'humanymous-release-v1',
      sbomSha256: attestation.sbomSha256,
      licenseExpression: 'Apache-2.0',
      noticeBundleSha256: attestation.noticeBundleSha256,
      attestationBundleSha256: digest(attestationRaw),
      reviewedAt: now.toISOString(),
      reviewExpiresAt: expiresAt.toISOString(),
      revoked: false,
    }],
  };
  catalog.catalogSha256 = catalogContentHash(catalog);
  validateCatalog(catalog, now);
  const catalogRaw = `${JSON.stringify(catalog, null, 2)}\n`;
  const privateKey = createPrivateKey(await readFile(signingKeyPath));
  const derivedPublic = createPublicKey(privateKey).export({ type: 'spki', format: 'pem' });
  const pinnedPublic = await readFile(trustRootPath);
  if (!Buffer.from(derivedPublic).equals(Buffer.from(pinnedPublic))) {
    throw new TypeError('catalog signing key does not match the pinned project trust root');
  }
  const signature = sign(null, Buffer.from(catalogRaw), privateKey);
  await mkdir(outputDirectory, { recursive: true });
  await Promise.all([
    writeFile(resolve(outputDirectory, 'reference.attestation.json'), attestationRaw, { mode: 0o444 }),
    writeFile(resolve(outputDirectory, 'reference.spdx.json'), sbomRaw, { mode: 0o444 }),
    writeFile(
      resolve(outputDirectory, 'vulnerability-policy.json'),
      vulnerabilityPolicyRaw,
      { mode: 0o444 },
    ),
    writeFile(resolve(outputDirectory, 'revocations.json'), revocationsRaw, { mode: 0o444 }),
    writeFile(resolve(outputDirectory, 'profiles.lock.json'), catalogRaw, { mode: 0o444 }),
    writeFile(resolve(outputDirectory, 'profiles.lock.sig'), `${signature.toString('base64')}\n`, { mode: 0o444 }),
  ]);
  return { catalog, attestation };
}

async function main() {
  const runtimeImages = parseStrictJson(
    await readFile(resolve(required('HM_VUSB_RUNTIME_IMAGES_JSON')), 'utf8'),
    'runtime image lock input',
  );
  await generateCatalog({
    signingKeyPath: resolve(required('HM_VUSB_CATALOG_SIGNING_KEY')),
    trustRootPath: resolve(
      process.env.HM_VUSB_CATALOG_TRUST_ROOT ||
      'deployments/external-input/trust/catalog-ed25519.pub',
    ),
    attestationBundlePath: resolve(required('HM_VUSB_ATTESTATION_BUNDLE_INPUT')),
    sourceRevision: required('HM_VUSB_SOURCE_REVISION'),
    runtimeImages,
    profileManifestDigest: required('HM_VUSB_PROFILE_MANIFEST_DIGEST'),
    profileConfigDigest: required('HM_VUSB_PROFILE_CONFIG_DIGEST'),
  });
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`virtual USB catalog generation failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
