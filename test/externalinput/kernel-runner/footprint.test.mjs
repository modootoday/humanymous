import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const root = new URL('../../../', import.meta.url);

async function source(path) {
  return readFile(new URL(path, root), 'utf8');
}

test('full runner ships a bounded compressed guest instead of a fixed raw disk', async () => {
  const dockerfile = await source('build/external-input-kernel-runner.Dockerfile');

  assert.doesNotMatch(dockerfile, /GUEST_DISK_MIB=12288/);
  assert.doesNotMatch(dockerfile, /\bdebootstrap\b/);
  assert.match(dockerfile, /GUEST_ROOT_HEADROOM_MIB=256/);
  assert.match(dockerfile, /MAXIMUM_GUEST_BASE_BYTES=201326592/);
  assert.match(
    dockerfile,
    /mke2fs -q -t ext4 -U 6b8e7d20-536f-42f0-8b1a-000000000042/,
  );
  assert.match(dockerfile, /qemu-img convert[^]*-O qcow2 -c/);
  assert.match(dockerfile, /\/out\/guest-root\.qcow2/);
  assert.doesNotMatch(dockerfile, /\/out\/guest-root\.ext4/);
  assert.match(
    dockerfile,
    /! -name "config-\$kernel" -exec rm -rf '\{\}' \+/,
  );
  assert.match(dockerfile, /test -r "\/guest-root\/boot\/config-\$kernel"/);
  assert.match(dockerfile, /configfs overlay bridge br_netfilter veth nf_nat/);
  assert.doesNotMatch(dockerfile, /ca-certificates iproute2 iptables/);
  assert.match(
    dockerfile,
    /\/usr\/local\/libexec\/docker\/cli-plugins\/docker-buildx/,
  );
});

test('run payload is cell-scoped, bounded, and removed after every attempt', async () => {
  const [orchestrator, qemu] = await Promise.all([
    source('scripts/external-input-kernel-runner.mjs'),
    source('deployments/external-input/kernel-runner/run-qemu.sh'),
  ]);

  assert.match(
    orchestrator,
    /MAXIMUM_IMAGE_ARCHIVE_BYTES = 672 \* 1024 \* 1024/,
  );
  assert.match(
    orchestrator,
    /MAXIMUM_RUNNER_IMAGE_BYTES = 288 \* 1024 \* 1024/,
  );
  assert.match(
    orchestrator,
    /runnerInspection\.Size > MAXIMUM_RUNNER_IMAGE_BYTES/,
  );
  assert.match(orchestrator, /--platform=linux\/amd64/);
  assert.match(orchestrator, /'--provenance=false'/);
  assert.match(orchestrator, /'--sbom=false'/);
  assert.match(
    orchestrator,
    /executeDocker\(\s*\['buildx', 'prune', '--all', '--force'\]/,
  );
  assert.match(
    orchestrator,
    /executeDocker\(arguments_, \{ environment, encoding: null \}\);[^]*?reclaimBuildIntermediates\(\)/,
  );
  assert.match(orchestrator, /createGzip/);
  assert.match(orchestrator, /finally \{/);
  assert.match(
    orchestrator,
    /rm\(resolve\(seedDirectory, 'images\.oci\.tar'\)/,
  );
  assert.match(qemu, /images\.oci\.tar/);
  assert.match(qemu, /kernel-docker-data\.ext4/);
  assert.match(qemu, /truncate -s 2048M/);
  assert.match(qemu, /mke2fs -q -t ext4 -m 0 -L humanymous-docker/);
  assert.match(qemu, /-F qcow2/);
});

test('English kernel cells can resolve Compose without enabling IME services', async () => {
  const compose = await source('deployments/compose/external-input-vusb.yaml');

  assert.doesNotMatch(
    compose,
    /\$\{HM_EXTERNAL_IME_LOCALE:\?[^}]*\}/,
  );
  assert.match(
    compose,
    /profiles: \[vusb-ime-policy\][^]*"\$\{HM_EXTERNAL_IME_LOCALE:-\}"/,
  );
  assert.match(
    compose,
    /profiles: \[vusb-ime-run\][^]*HM_EXTERNAL_IME_LOCALE: "\$\{HM_EXTERNAL_IME_LOCALE:-\}"/,
  );
});

test('nested browser and display keep bounded memory reservations', async () => {
  const [compose, display, smoke] = await Promise.all([
    source('deployments/compose/external-input-bots.yaml'),
    source('deployments/external-input/display-entrypoint.sh'),
    source('scripts/external-input-smoke.mjs'),
  ]);

  assert.match(compose, /shm_size: 256m/);
  assert.doesNotMatch(compose, /shm_size: 512m/);
  assert.match(display, /VideoRam 65536/);
  assert.doesNotMatch(display, /VideoRam 256000/);
  const probeStart = smoke.indexOf('// Evidence probes are independent');
  const probeEnd = smoke.indexOf("'external-runtime-purity-evaluator'", probeStart);
  assert.ok(probeStart > 0 && probeEnd > probeStart);
  assert.doesNotMatch(smoke.slice(probeStart, probeEnd), /Promise\.all/);
});

test('English browser targets stay IME-free and IME targets remain explicit', async () => {
  const dockerfile = await source('build/external-input-browser.Dockerfile');
  const commonStart = dockerfile.indexOf(
    'FROM debian:bookworm-slim AS browser-common',
  );
  const commonEnd = dockerfile.indexOf(
    'FROM browser-common AS browser-chromium-base',
    commonStart,
  );
  const browserCommon = dockerfile.slice(commonStart, commonEnd);
  const provision = await source('scripts/external-input-vusb-provision.sh');

  assert.equal(
    (
      dockerfile.match(
        /^FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818 AS /gm,
      ) || []
    ).length,
    2,
  );
  assert.doesNotMatch(
    browserCommon,
    /fonts-noto-cjk|ibus(?:-anthy|-gtk3|-hangul|-libpinyin)?|dbus-x11|procps/,
  );
  assert.doesNotMatch(dockerfile, /\bopenbox\b|x11-xserver-utils/);
  assert.match(
    dockerfile,
    /FROM browser-chromium-base AS browser-chromium-ime[^]*fonts-noto-cjk[^]*ibus-anthy[^]*ibus-hangul[^]*ibus-libpinyin/,
  );
  assert.match(
    dockerfile,
    /FROM browser-firefox-base AS browser-firefox-ime[^]*fonts-noto-cjk[^]*ibus-anthy[^]*ibus-hangul[^]*ibus-libpinyin/,
  );
  assert.match(provision, /browser-chromium-ime:vusb-local browser-chromium-ime/);
  assert.match(provision, /browser-firefox-ime:vusb-local browser-firefox-ime/);
});

test('external-input runtime bases are immutable', async () => {
  for (const path of [
    'build/external-input-browser.Dockerfile',
    'build/external-input-controller.Dockerfile',
    'build/external-input-pki.Dockerfile',
    'build/external-input-usb-broker.Dockerfile',
    'build/external-input-vusb-gateway.Dockerfile',
    'build/external-input-vusb-lifecycle.Dockerfile',
  ]) {
    const dockerfile = await source(path);
    for (const line of dockerfile.match(/^FROM .+$/gm) || []) {
      if (/^FROM browser-/.test(line)) continue;
      assert.match(line, /@sha256:[a-f0-9]{64}(?:\s+AS\s+\S+)?$/);
    }
  }
});

test('purity evidence resolves every Compose profile without widening execution', async () => {
  const supervisor = await source('scripts/external-input-vusb-docker.sh');
  const evidenceConfigs = supervisor.match(
    /--profile '\*' config --format json/g,
  ) || [];

  assert.equal(evidenceConfigs.length, 2);
  assert.match(
    supervisor,
    /docker compose "\$\{files\[@\]\}" --profile vusb-run -p "\$project" "\$@"/,
  );
});

test('gadget nodes bind to configfs identities while host nodes bind to USB serial', async () => {
  const lifecycle = await source('test/externalinput/vusb/lifecycle.sh');
  const gadgetStart = lifecycle.indexOf('find_run_gadget_node()');
  const gadgetEnd = lifecycle.indexOf('\nmodule_was_preloaded()', gadgetStart);
  const gadgetLookup = lifecycle.slice(gadgetStart, gadgetEnd);
  const hostStart = lifecycle.indexOf('find_run_link()');
  const hostEnd = lifecycle.indexOf('\nfind_run_gadget_node()', hostStart);
  const hostLookup = lifecycle.slice(hostStart, hostEnd);

  assert.match(gadgetLookup, /functions\/acm\.usb0\/port_num/);
  assert.match(gadgetLookup, /functions\/hid\.\$\{kind\}\/dev/);
  assert.doesNotMatch(gadgetLookup, /node_belongs_to_run/);
  assert.match(hostLookup, /node_belongs_to_run "\$target"/);
  assert.match(
    lifecycle,
    /printf '\\n' > "\$GADGET_ROOT\/UDC"/,
  );
  assert.match(lifecycle, /while \[ "\$attempts" -lt 40 \]/);
  assert.doesNotMatch(lifecycle, /printf '' > "\$GADGET_ROOT\/UDC"/);
  assert.doesNotMatch(lifecycle, /\[ ! -s "\$GADGET_ROOT\/UDC" \]/);
});

test('standard CI builds only the compact smoke target with an image cap', async () => {
  const workflow = await source('.github/workflows/ci.yml');

  assert.match(workflow, /external-input-kernel-smoke:/);
  assert.match(workflow, /target: smoke/);
  assert.match(workflow, /platforms: linux\/amd64/);
  assert.match(workflow, /provenance: false/);
  assert.match(workflow, /sbom: false/);
  assert.match(workflow, /test "\$bytes" -le 50331648/);
  assert.match(workflow, /humanymous-storage-before/);
  assert.match(workflow, /test "\$delta" -le 536870912/);
  assert.match(
    workflow,
    /cache-to: type=gha,mode=min,scope=external-input-kernel-smoke/,
  );
  assert.doesNotMatch(
    workflow,
    /target: full[^]*external-input-kernel-smoke:/,
  );
});
