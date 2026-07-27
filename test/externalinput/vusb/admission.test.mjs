import assert from 'node:assert/strict';
import { generateKeyPairSync, sign } from 'node:crypto';
import {
  chmod, link, mkdir, mkdtemp, readFile, rm, unlink, writeFile,
} from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { admitVirtualUsbProfile } from './admission.mjs';
import { catalogContentHash } from './catalog.mjs';
import { canonicalJson, sha256 } from './common.mjs';
import { verifyProfileRoot } from './profile.mjs';
import { parseStrictJson } from './strict-json.mjs';
import { createAttemptManifest, createLadderManifest } from './manifest.mjs';

const DIGEST = (character) => `sha256:${character.repeat(64)}`;

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-vusb-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const profileRoot = join(root, 'image');
  await mkdir(join(profileRoot, 'profile'), { recursive: true });
  const profile = {
    contractVersion: 'humanymous.virtual-usb-profile/v1',
    modelId: 'reference-relative-v1',
    descriptorSetId: 'reference-relative-v1',
    protocolVersion: '1.0.0',
    usbOrigin: 'kernel-emulated',
    physicalCapable: false,
    limits: {
      maxReportsPerSecond: 120,
      maxRelativeStep: 127,
      maxKeyDwellMs: 250,
      maxPointerDwellMs: 250,
      deadManReleaseMs: 500,
    },
  };
  const profileRaw = `${JSON.stringify(profile, null, 2)}\n`;
  await writeFile(join(profileRoot, 'profile', 'virtual-usb-profile.json'), profileRaw);
  await writeFile(join(profileRoot, 'profile', 'LICENSE'), 'Apache License 2.0\n');
  await writeFile(join(profileRoot, 'profile', 'NOTICE'), 'humanymous virtual USB profile\n');
  const sbomRaw = `${JSON.stringify({
    spdxVersion: 'SPDX-2.3',
    dataLicense: 'CC0-1.0',
    SPDXID: 'SPDXRef-DOCUMENT',
    name: 'humanymous-reference-relative-v1',
    documentNamespace: 'https://example.invalid/spdx/reference-relative-v1',
    creationInfo: {
      created: '2026-07-26T00:00:00Z',
      creators: ['Organization: humanymous'],
    },
    packages: [{
      name: 'reference-relative-v1-profile',
      SPDXID: 'SPDXRef-Profile',
      versionInfo: '1.0.0',
      downloadLocation: 'NOASSERTION',
      filesAnalyzed: false,
      licenseConcluded: 'Apache-2.0',
      licenseDeclared: 'Apache-2.0',
      copyrightText: 'NOASSERTION',
      primaryPackagePurpose: 'DATA',
    }],
  }, null, 2)}\n`;
  const vulnerabilityPolicyRaw = `${JSON.stringify({
    schemaVersion: 'humanymous.virtual-usb-vulnerability-policy/v1',
    profileContentType: 'data-only',
    denyKnownExploited: true,
    maximumCritical: 0,
    maximumHigh: 0,
    scannerDatabaseSnapshot: '2026-07-26',
    note: 'bounded fixture policy',
  }, null, 2)}\n`;
  const revocationsRaw = `${JSON.stringify({
    schemaVersion: 'humanymous.virtual-usb-revocations/v1',
    revision: '2026-07-26.1',
    revokedModelIds: [],
    revokedImageDigests: [],
  }, null, 2)}\n`;
  const sbomPath = join(root, 'reference.spdx.json');
  const vulnerabilityPolicyPath = join(root, 'vulnerability-policy.json');
  const revocationsPath = join(root, 'revocations.json');
  await Promise.all([
    writeFile(sbomPath, sbomRaw),
    writeFile(vulnerabilityPolicyPath, vulnerabilityPolicyRaw),
    writeFile(revocationsPath, revocationsRaw),
  ]);
  const attestation = {
    schemaVersion: 'humanymous.virtual-usb-attestation/v1',
    imageIndexDigest: DIGEST('1'),
    platform: 'linux/amd64',
    platformManifestDigest: DIGEST('2'),
    configDigest: DIGEST('3'),
    profileManifestSha256: `sha256:${sha256(profileRaw)}`,
    sourceRevision: 'a'.repeat(40),
    builderId: 'https://github.com/modootoday/humanymous/.github/workflows/release.yml',
    signatureIdentity: 'https://github.com/modootoday/humanymous/.github/workflows/release.yml@refs/heads/main',
    signatureIssuer: 'https://token.actions.githubusercontent.com',
    sbomSha256: `sha256:${sha256(sbomRaw)}`,
    licenseSha256: DIGEST('5'),
    noticeBundleSha256: DIGEST('6'),
    conformanceSuiteSha256: DIGEST('7'),
    vulnerabilityPolicySha256: `sha256:${sha256(vulnerabilityPolicyRaw)}`,
    revocationSnapshotSha256: `sha256:${sha256(revocationsRaw)}`,
    validUntil: '2099-01-01T00:00:00.000Z',
  };
  const attestationRaw = `${JSON.stringify(attestation, null, 2)}\n`;
  const attestationPath = join(root, 'attestation.json');
  await writeFile(attestationPath, attestationRaw);
  const catalog = {
    catalogRevision: '2026-07-26.1',
    catalogSha256: '',
    catalogSignaturePolicyId: 'humanymous-catalog-v1',
    runtimeImages: Object.fromEntries([
      'labCore', 'pki', 'display', 'browserChromium', 'browserChromiumDom',
      'browserChromiumIme', 'browserFirefox', 'browserFirefoxDom',
      'browserFirefoxIme', 'controller', 'lifecycle', 'gateway', 'profile',
    ].map((name, index) => [name, DIGEST(String((index % 9) + 1))])),
    models: [{
      modelId: profile.modelId,
      displayName: 'Reference relative-pointer model',
      contractVersion: profile.contractVersion,
      authority: 'project-reference',
      imageIndexDigest: attestation.imageIndexDigest,
      platform: attestation.platform,
      platformManifestDigest: attestation.platformManifestDigest,
      configDigest: attestation.configDigest,
      sourceRevision: attestation.sourceRevision,
      profileManifestSha256: attestation.profileManifestSha256,
      descriptorSetId: profile.descriptorSetId,
      descriptorContractSha256: DIGEST('a'),
      protocolContractSha256: DIGEST('b'),
      conformanceSuiteSha256: attestation.conformanceSuiteSha256,
      signaturePolicyId: 'humanymous-release-v1',
      sbomSha256: attestation.sbomSha256,
      licenseExpression: 'Apache-2.0',
      noticeBundleSha256: attestation.noticeBundleSha256,
      attestationBundleSha256: `sha256:${sha256(attestationRaw)}`,
      reviewedAt: '2026-07-26T00:00:00.000Z',
      reviewExpiresAt: '2099-01-01T00:00:00.000Z',
      revoked: false,
    }],
  };
  catalog.catalogSha256 = catalogContentHash(catalog);
  const catalogRaw = `${JSON.stringify(catalog, null, 2)}\n`;
  const catalogPath = join(root, 'profiles.lock.json');
  const signaturePath = join(root, 'profiles.lock.sig');
  const publicKeyPath = join(root, 'catalog-public.pem');
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  await writeFile(catalogPath, catalogRaw);
  await writeFile(signaturePath, `${sign(null, Buffer.from(catalogRaw), privateKey).toString('base64')}\n`);
  await writeFile(publicKeyPath, publicKey.export({ type: 'spki', format: 'pem' }));
  const ladder = createLadderManifest({
    ladderId: 'vusb-admission-ladder-0001',
    modelId: profile.modelId,
    catalogSha256: catalog.catalogSha256,
    runtimeImages: catalog.runtimeImages,
  });
  const ladderRaw = `${JSON.stringify(ladder, null, 2)}\n`;
  const ladderManifestPath = join(root, 'ladder.json');
  const attemptManifestPath = join(root, 'attempt.json');
  await writeFile(ladderManifestPath, ladderRaw);
  await writeFile(attemptManifestPath, `${JSON.stringify(createAttemptManifest({
    ladder,
    ladderManifestSha256: `sha256:${sha256(ladderRaw)}`,
    runId: 'vusb-test-run-0001',
    axis: 'control',
    browser: 'chromium',
    sequence: 3,
    profileId: 'external_input_vusb',
    childProject: 'hmn-ext-vusb-test-run-0001-m3',
    parentProject: 'hmn-vusb-parent-vusb-test-run-0001',
  }), null, 2)}\n`);
  return {
    root, profileRoot, profile, catalog, catalogPath, signaturePath,
    publicKeyPath, attestationPath, sbomPath, vulnerabilityPolicyPath,
    revocationsPath,
    ladderManifestPath,
    attemptManifestPath,
  };
}

function admissionArgs(input, overrides = {}) {
  return {
    catalogPath: input.catalogPath,
    catalogSignaturePath: input.signaturePath,
    catalogPublicKeyPath: input.publicKeyPath,
    modelId: input.profile.modelId,
    profileRoot: input.profileRoot,
    attestationPath: input.attestationPath,
    sbomPath: input.sbomPath,
    vulnerabilityPolicyPath: input.vulnerabilityPolicyPath,
    revocationsPath: input.revocationsPath,
    ladderManifestPath: input.ladderManifestPath,
    attemptManifestPath: input.attemptManifestPath,
    ...overrides,
  };
}

test('strict JSON rejects duplicate keys instead of taking the last value', () => {
  assert.throws(
    () => parseStrictJson('{"modelId":"approved","modelId":"attacker"}', 'manifest'),
    /duplicate key/,
  );
});

test('whole-rootfs profile verification admits only the bounded declarative tree', async (t) => {
  const input = await fixture(t);
  const verified = await verifyProfileRoot(input.profileRoot);
  assert.equal(verified.profile.modelId, 'reference-relative-v1');

  await writeFile(join(input.profileRoot, 'entrypoint.sh'), '#!/bin/sh\n');
  await assert.rejects(() => verifyProfileRoot(input.profileRoot), /forbidden path/);
});

test('whole-rootfs profile verification rejects hard-linked payloads', async (t) => {
  const input = await fixture(t);
  const notice = join(input.profileRoot, 'profile', 'NOTICE');
  await unlink(notice);
  await link(join(input.profileRoot, 'profile', 'LICENSE'), notice);
  await assert.rejects(() => verifyProfileRoot(input.profileRoot), /hard link/);
});

test('whole-rootfs profile verification rejects executable payloads', {
  skip: process.platform === 'win32' ? 'Windows does not expose POSIX executable mode bits' : false,
}, async (t) => {
  const input = await fixture(t);
  const notice = join(input.profileRoot, 'profile', 'NOTICE');
  await chmod(notice, 0o755);
  await assert.rejects(() => verifyProfileRoot(input.profileRoot), /executable/);
});

test('offline admission binds signed catalog, profile, attestation, and run receipt', async (t) => {
  const input = await fixture(t);
  const receiptPath = join(input.root, 'admission.json');
  const receipt = await admitVirtualUsbProfile(admissionArgs(input, {
    receiptPath,
    runId: 'vusb-test-run-0001',
    now: new Date('2026-07-27T00:00:00.000Z'),
  }));
  assert.equal(receipt.kind, 'admission');
  assert.equal(receipt.canonical, true);
  assert.equal(receipt.catalogSha256, input.catalog.catalogSha256);
  assert.equal(receipt.authority, 'project-reference');
});

test('admission fails before mutation on catalog, bundle, or model substitution', async (t) => {
  const input = await fixture(t);
  await writeFile(input.signaturePath, `${Buffer.alloc(64).toString('base64')}\n`);
  await assert.rejects(() => admitVirtualUsbProfile(admissionArgs(input, {
    runId: 'vusb-test-run-0002',
    now: new Date('2026-07-27T00:00:00.000Z'),
  })), /signature/);
});

test('admission evaluates the actual SPDX, vulnerability, and revocation files', async (t) => {
  const input = await fixture(t);
  await writeFile(input.revocationsPath, `${JSON.stringify({
    schemaVersion: 'humanymous.virtual-usb-revocations/v1',
    revision: '2026-07-26.2',
    revokedModelIds: [input.profile.modelId],
    revokedImageDigests: [],
  }, null, 2)}\n`);
  await assert.rejects(() => admitVirtualUsbProfile(admissionArgs(input, {
    runId: 'vusb-test-run-0003',
    now: new Date('2026-07-27T00:00:00.000Z'),
  })), /digest mismatch|revoked/);
});

test('admission rejects selected-browser or attempt-manifest drift before mutation', async (t) => {
  const input = await fixture(t);
  const attempt = parseStrictJson(
    await readFile(input.attemptManifestPath, 'utf8'),
    'attempt manifest',
  );
  attempt.selectedBrowserImageDigest = DIGEST('f');
  await writeFile(input.attemptManifestPath, `${JSON.stringify(attempt, null, 2)}\n`);
  await assert.rejects(() => admitVirtualUsbProfile(admissionArgs(input, {
    runId: 'vusb-test-run-0001',
    now: new Date('2026-07-27T00:00:00.000Z'),
  })), /selected browser image/);
});

test('catalog canonical hash covers model ordering and every policy field', async (t) => {
  const input = await fixture(t);
  const changed = structuredClone(input.catalog);
  changed.models[0].authority = 'reviewed-comparison';
  assert.notEqual(catalogContentHash(changed), input.catalog.catalogSha256);
  assert.equal(canonicalJson({ b: 1, a: 2 }), '{"a":2,"b":1}');
});
