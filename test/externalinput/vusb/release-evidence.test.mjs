import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { sha256 } from './common.mjs';
import { verifyReleaseEvidence } from './release-evidence.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-release-evidence-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const values = {
    sbom: {
      spdxVersion: 'SPDX-2.3',
      dataLicense: 'CC0-1.0',
      SPDXID: 'SPDXRef-DOCUMENT',
      name: 'reference-relative-v1',
      documentNamespace: 'https://example.invalid/spdx/reference-relative-v1',
      creationInfo: { created: '2026-07-26T00:00:00Z', creators: ['Organization: humanymous'] },
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
    },
    vulnerabilityPolicy: {
      schemaVersion: 'humanymous.virtual-usb-vulnerability-policy/v1',
      profileContentType: 'data-only',
      denyKnownExploited: true,
      maximumCritical: 0,
      maximumHigh: 0,
      scannerDatabaseSnapshot: '2026-07-26',
      note: 'test',
    },
    revocations: {
      schemaVersion: 'humanymous.virtual-usb-revocations/v1',
      revision: '2026-07-26.1',
      revokedModelIds: [],
      revokedImageDigests: [],
    },
  };
  const paths = {};
  const raws = {};
  for (const [name, value] of Object.entries(values)) {
    const filename = name === 'sbom'
      ? 'reference.spdx.json'
      : name === 'vulnerabilityPolicy' ? 'vulnerability-policy.json' : 'revocations.json';
    paths[`${name}Path`] = join(root, filename);
    raws[name] = `${JSON.stringify(value, null, 2)}\n`;
    await writeFile(paths[`${name}Path`], raws[name]);
  }
  const attestation = {
    sbomSha256: `sha256:${sha256(raws.sbom)}`,
    vulnerabilityPolicySha256: `sha256:${sha256(raws.vulnerabilityPolicy)}`,
    revocationSnapshotSha256: `sha256:${sha256(raws.revocations)}`,
  };
  return { paths, values, raws, attestation };
}

test('release evidence evaluates the signed policy files, not caller labels', async (t) => {
  const input = await fixture(t);
  const result = await verifyReleaseEvidence({
    ...input.paths,
    attestation: input.attestation,
    model: {
      modelId: 'reference-relative-v1',
      imageIndexDigest: `sha256:${'1'.repeat(64)}`,
      platformManifestDigest: `sha256:${'2'.repeat(64)}`,
      configDigest: `sha256:${'3'.repeat(64)}`,
    },
    runtimeImages: { labCore: `sha256:${'4'.repeat(64)}` },
    now: new Date('2026-07-27T00:00:00Z'),
  });
  assert.equal(result.revocationRevision, '2026-07-26.1');
  assert.equal(result.scannerDatabaseSnapshot, '2026-07-26');
});

test('release evidence rejects a selected model or image in the revocation snapshot', async (t) => {
  const input = await fixture(t);
  input.values.revocations.revokedModelIds.push('reference-relative-v1');
  const raw = `${JSON.stringify(input.values.revocations, null, 2)}\n`;
  await writeFile(input.paths.revocationsPath, raw);
  input.attestation.revocationSnapshotSha256 = `sha256:${sha256(raw)}`;
  await assert.rejects(() => verifyReleaseEvidence({
    ...input.paths,
    attestation: input.attestation,
    model: {
      modelId: 'reference-relative-v1',
      imageIndexDigest: `sha256:${'1'.repeat(64)}`,
      platformManifestDigest: `sha256:${'2'.repeat(64)}`,
      configDigest: `sha256:${'3'.repeat(64)}`,
    },
    runtimeImages: {},
    now: new Date('2026-07-27T00:00:00Z'),
  }), /model is revoked/);
});
