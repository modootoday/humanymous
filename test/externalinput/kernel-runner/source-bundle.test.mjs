import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import {
  collectKernelRunnerSource,
  writeKernelRunnerSourceBundle,
} from './source-bundle.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-kernel-source-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const files = [
    'configs/dev.env',
    'deployments/compose.yaml',
    'deployments/compose/bots.yaml',
    'deployments/compose/defenders.yaml',
    'deployments/compose/external-input-bots.yaml',
    'deployments/compose/external-input-dom.yaml',
    'deployments/compose/external-input-firefox.yaml',
    'deployments/compose/external-input-vusb.yaml',
    'deployments/compose/external-input-vusb-manifest.yaml',
    'deployments/compose/networks.yaml',
    'deployments/bots/external-input-run.sh',
    'deployments/external-input/browser-entrypoint.sh',
    'scripts/external-input-kernel-guest.sh',
    'scripts/assert-external-input-vusb.mjs',
    'test/e2e/external-input-runner.mjs',
    'test/externalinput/contracts.mjs',
    'test/externalinput/kernel-runner/runtime-images.mjs',
    'test/externalinput/kernel-runner/seed.mjs',
    'test/redteam/external_input_usb.mjs',
  ];
  for (const path of files) {
    const destination = join(root, ...path.split('/'));
    await mkdir(join(destination, '..'), { recursive: true });
    await writeFile(destination, `${path}\n`);
  }
  return root;
}

test('writes a deterministic bounded ustar source bundle', async (t) => {
  const root = await fixture(t);
  const paths = await collectKernelRunnerSource(root);
  const first = await writeKernelRunnerSourceBundle({
    projectRoot: root,
    destination: join(root, 'first.tar'),
    paths,
  });
  const second = await writeKernelRunnerSourceBundle({
    projectRoot: root,
    destination: join(root, 'second.tar'),
    paths,
  });
  assert.equal(first.sha256, second.sha256);
  assert.equal(first.bytes % 512, 0);
  assert.equal(first.entries, paths.length);
  assert.deepEqual(await readFile(join(root, 'first.tar')), await readFile(join(root, 'second.tar')));
});

test('rejects a symlink in a selected source tree', async (t) => {
  const root = await fixture(t);
  await symlink(
    join(root, 'test/externalinput/contracts.mjs'),
    join(root, 'test/externalinput/substituted.mjs'),
  );
  await assert.rejects(
    collectKernelRunnerSource(root),
    /special entry/,
  );
});
