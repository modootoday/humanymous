import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const root = new URL('../../', import.meta.url);

async function source(path) {
  return readFile(new URL(path, root), 'utf8');
}

function workflowJob(workflow, name, nextName) {
  const start = workflow.indexOf(`  ${name}:`);
  assert.notEqual(start, -1, `missing ${name} workflow job`);
  const end = nextName === undefined
    ? workflow.length
    : workflow.indexOf(`  ${nextName}:`, start + 1);
  assert.notEqual(end, -1, `missing ${nextName} workflow job`);
  return workflow.slice(start, end);
}

test('bots runtime installs only locked headful Chromium and Firefox', async () => {
  const dockerfile = await source('build/bots.Dockerfile');
  const goCopy = dockerfile.indexOf('COPY --from=gobuild /out/redteam');
  const browserInstall = dockerfile.indexOf(
    './node_modules/.bin/playwright-core install chromium firefox',
  );

  assert.match(
    dockerfile,
    /^FROM node:22-bookworm-slim@sha256:6c74791e557ce11fc957704f6d4fe134a7bc8d6f5ca4403205b2966bd488f6b3$/m,
  );
  assert.match(
    dockerfile,
    /^FROM golang:1\.25-alpine@sha256:56961d79ea8129efddcc0b8643fd8a5416b4e6228cfd477e3fd61deb2672c587 AS gobuild$/m,
  );
  assert.match(
    dockerfile,
    /^FROM lwthiker\/curl-impersonate:0\.6-chrome@sha256:4039d9d38b0182dc2b8550b1320da2fd0e7b9720f21f299fa0e40cbfba95cc64 AS curl-impersonate$/m,
  );
  assert.doesNotMatch(dockerfile, /mcr\.microsoft\.com\/playwright/);
  assert.match(dockerfile, /ENV PLAYWRIGHT_BROWSERS_PATH=\/ms-playwright/);
  assert.match(dockerfile, /npm ci --no-audit --no-fund/);
  // Chromium's headless shell must stay installed: ten catalog profiles launch
  // it directly (selenium, puppeteer, playwright, undetected-chromedriver,
  // direct-cdp, the stealth trio, rebrowser-cdp-stripped, browser-use-cdp).
  // Run 30239746911 built the image with --no-shell and every one of them died
  // on "Executable doesn't exist at /ms-playwright/chromium_headless_shell-*",
  // which attack-assert reported as 10 errored/skipped catalog records. Saving
  // image bytes must never silently retire red coverage.
  assert.match(
    dockerfile,
    /\.\/node_modules\/\.bin\/playwright-core install chromium firefox/,
  );
  assert.doesNotMatch(dockerfile, /playwright-core install[^\n]*--no-shell/);
  assert.match(dockerfile, /apt-get install -y --no-install-recommends/);
  assert.match(
    dockerfile,
    /dpkg-divert --local --rename --add \/usr\/bin\/update-mime-database/,
  );
  assert.match(
    dockerfile,
    /dpkg-divert --local --rename --remove \/usr\/bin\/update-mime-database/,
  );
  assert.doesNotMatch(dockerfile, /playwright-core install[^\n]*\bwebkit\b/);
  assert.doesNotMatch(dockerfile, /playwright-core install[^\n]*--with-deps/);
  assert.doesNotMatch(dockerfile, /--only-shell/);
  assert.ok(goCopy > 0 && browserInstall > goCopy);
  for (const removed of [
    '/var/cache/apt/*',
    '/var/lib/apt/lists/*',
    '/usr/share/doc/*',
    '/usr/share/man/*',
  ]) {
    assert.match(dockerfile, new RegExp(removed.replaceAll('/', '\\/').replace('*', '\\*')));
  }

  for (const preserved of [
    'HM_REDTEAM_BIN=/app/bin/redteam',
    'HM_TLSPARROT_BIN=/app/bin/tlsparrot',
    'HM_CURL_IMPERSONATE_DIR=/app/curl-impersonate/bin',
    'HM_CURL_IMPERSONATE_LIB=/app/curl-impersonate/lib',
    'HM_BROWSER_CHANNEL=chromium',
    'NODE_TLS_REJECT_UNAUTHORIZED=0',
    'LD_LIBRARY_PATH=/app/curl-impersonate/lib',
  ]) {
    assert.match(dockerfile, new RegExp(preserved.replaceAll('/', '\\/')));
  }
});

test('standard compose exports minimum image caches', async () => {
  const compose = await source('deployments/compose.ci.yaml');

  assert.equal((compose.match(/cache_to:/g) || []).length, 3);
  assert.equal((compose.match(/mode=min/g) || []).length, 3);
  assert.doesNotMatch(compose, /mode=max/);
});

test('standard images use immutable compact build and runtime bases', async () => {
  for (const path of ['build/core.Dockerfile', 'build/gate.Dockerfile']) {
    const dockerfile = await source(path);
    assert.match(
      dockerfile,
      /^FROM --platform=\$BUILDPLATFORM golang:1\.25-alpine@sha256:56961d79ea8129efddcc0b8643fd8a5416b4e6228cfd477e3fd61deb2672c587 AS builder$/m,
    );
    assert.match(
      dockerfile,
      /^FROM gcr\.io\/distroless\/static-debian12:nonroot@sha256:f5b485ea962d9bd1186b2f6b3a061191539b905b82ec395de78cbfae51f20e35$/m,
    );
  }
});

test('compact kernel smoke cannot silently expand into the full runner', async () => {
  const workflow = await source('.github/workflows/ci.yml');
  const smoke = workflowJob(
    workflow,
    'external-input-kernel-smoke',
    'detector-vs-bots',
  );

  assert.match(smoke, /target: smoke/);
  assert.match(smoke, /platforms: linux\/amd64/);
  assert.match(smoke, /provenance: false/);
  assert.match(smoke, /sbom: false/);
  assert.match(smoke, /test "\$bytes" -le 134217728/);
  assert.match(smoke, /test "\$delta" -le 536870912/);
  // A budget that fails without reporting what it measured cannot be acted on.
  assert.match(smoke, /echo "smoke_image_bytes=\$bytes"/);
  assert.match(smoke, /echo "smoke_storage_delta_bytes=\$delta"/);
  assert.match(
    smoke,
    /cache-to: type=gha,mode=min,scope=external-input-kernel-smoke/,
  );
  assert.doesNotMatch(smoke, /target: full/);
  assert.doesNotMatch(smoke, /external-input-kernel-runner:ci/);
});

test('detector job bounds disk growth and artifact upload inputs', async () => {
  const workflow = await source('.github/workflows/ci.yml');
  const detector = workflowJob(workflow, 'detector-vs-bots');

  const baseline = detector.indexOf('Capture detector storage baseline');
  const build = detector.indexOf(
    'Build canonical images under transient storage budget',
  );
  const postBuild = detector.indexOf('Diagnose post-build storage phase');
  const reclaim = detector.indexOf(
    'Reclaim build cache and enforce retained storage budget',
  );
  const finalReclaim = detector.indexOf('Reclaim completed test images');
  const final = detector.indexOf('Enforce final detector storage budget');
  const artifact = detector.indexOf('Enforce artifact upload input budget');
  const upload = detector.indexOf('Upload run artifacts');

  assert.ok(baseline < build);
  assert.ok(build < postBuild);
  assert.ok(postBuild < reclaim);
  assert.ok(reclaim < finalReclaim);
  assert.ok(finalReclaim < final);
  assert.ok(final < artifact);
  assert.ok(artifact < upload);
  const buildPhase = detector.slice(build, postBuild);
  assert.match(buildPhase, /--profile attack/);
  assert.match(buildPhase, /build core gate/);
  assert.match(buildPhase, /build bots/);
  assert.ok(
    buildPhase.indexOf('build core gate') <
      buildPhase.indexOf('docker buildx prune --all --force'),
  );
  assert.ok(
    buildPhase.indexOf('docker buildx prune --all --force') <
      buildPhase.indexOf('build bots'),
  );
  assert.match(buildPhase, /test "\$peak_delta" -le 6442450944/);
  assert.doesNotMatch(buildPhase, /--profile (gate-test|pass-test|swarm)/);
  assert.match(detector, /docker buildx prune --all --force/);
  assert.match(detector, /docker image prune --all --force/);
  assert.equal(
    (detector.match(/test "\$delta" -le 1879048192/g) || []).length,
    2,
  );
  // The final budget runs after a full image prune that also removes images the
  // runner shipped with, so its retained delta is legitimately negative and must
  // not carry a lower bound (run 30239251233: final_delta_bytes=-1784422400).
  // The earlier reclaim step prunes only this job's buildx cache and keeps its
  // lower bound, so this is scoped to the final step rather than the whole job.
  const finalBudget = detector.slice(final, artifact);
  assert.match(finalBudget, /test "\$delta" -le 1879048192/);
  assert.doesNotMatch(finalBudget, /test "\$delta" -ge 0/);
  assert.match(
    detector,
    /while ! test -e "\$HM_STORAGE_DONE"/,
  );
  assert.match(detector, /sleep 0\.25/);
  assert.match(detector, /whole_job_peak_delta_bytes/);
  assert.equal(
    (detector.match(/test "\$peak_delta" -le 6442450944/g) || []).length,
    2,
  );
  assert.match(detector, /test "\$bytes" -le 4194304/);
  assert.doesNotMatch(detector, /swarm\.log|\.agent-runs\/e2e/);
  assert.match(
    detector,
    /steps\.artifact-budget\.outcome == 'success'/,
  );
  assert.match(detector, /retention-days: 5/);
});
