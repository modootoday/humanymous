import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { atomicJson } from './common.mjs';

test('receipt publication is atomic and refuses to overwrite prior evidence', async (t) => {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-vusb-receipt-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const destination = join(root, 'receipt.json');
  await atomicJson(destination, { sequence: 1 });
  await assert.rejects(
    () => atomicJson(destination, { sequence: 2 }),
    (error) => error?.code === 'EEXIST',
  );
  assert.deepEqual(JSON.parse(await readFile(destination, 'utf8')), { sequence: 1 });
});

test('explicit rolling evidence may replace its prior snapshot', async (t) => {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-vusb-evidence-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const destination = join(root, 'evidence.json');
  await atomicJson(destination, { sequence: 1 }, { replace: true });
  await atomicJson(destination, { sequence: 2 }, { replace: true });
  assert.deepEqual(JSON.parse(await readFile(destination, 'utf8')), { sequence: 2 });
});
