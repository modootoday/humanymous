import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { RUNTIME_IMAGE_FIELDS } from './catalog.mjs';
import {
  createAttemptManifest,
  createLadderManifest,
  loadLadderManifest,
  validateAttemptManifest,
  writeLadderManifest,
} from './manifest.mjs';

const digest = (character) => `sha256:${character.repeat(64)}`;
const runtimeImages = Object.fromEntries(
  RUNTIME_IMAGE_FIELDS.map((name, index) => [name, digest(String((index % 9) + 1))]),
);

function ladder() {
  return createLadderManifest({
    ladderId: 'vusb-ladder-test-0001',
    modelId: 'reference-relative-v1',
    catalogSha256: digest('a'),
    runtimeImages,
  });
}

test('ladder manifest freezes the complete canonical control and IME matrices', () => {
  const value = ladder();
  assert.equal(value.controlCells.length, 8);
  assert.equal(value.imeCells.length, 6);
  assert.equal(value.controlCells[0].profileId, 'external_input_virtual');
  assert.equal(value.controlCells.at(-1).profileId, 'external_input_dom_vusb');
  assert.equal(value.imeCells[0].locale, 'ko-KR');
  assert.match(value.matrixSha256, /^sha256:[a-f0-9]{64}$/);
});

test('attempt manifest derives backend semantics from one canonical matrix cell', () => {
  const value = createAttemptManifest({
    ladder: ladder(),
    ladderManifestSha256: digest('b'),
    runId: 'vusb-attempt-test-0001',
    axis: 'control',
    browser: 'firefox',
    sequence: 4,
    profileId: 'external_input_dom_vusb',
    childProject: 'hmn-ext-vusb-attempt-test-0001-m4',
    parentProject: 'hmn-vusb-parent-vusb-attempt-test-0001',
  });
  assert.equal(value.observation, 'dom+framebuffer');
  assert.equal(value.inputBackend, 'usb-hid-emulated');
  assert.equal(value.usbOrigin, 'kernel-emulated');
  assert.equal(value.domRequired, true);
  assert.equal(value.selectedBrowserImageDigest, runtimeImages.browserFirefoxDom);
});

test('IME attempts select the isolated browser IME image', () => {
  const value = createAttemptManifest({
    ladder: ladder(),
    ladderManifestSha256: digest('b'),
    runId: 'vusb-attempt-ime-test-0001',
    axis: 'ime',
    browser: 'chromium',
    sequence: 3,
    profileId: 'external_input_vusb',
    locale: 'ko-KR',
    childProject: 'hmn-ext-vusb-attempt-ime-test-0001-m3',
    parentProject: 'hmn-vusb-parent-vusb-attempt-ime-test-0001',
  });
  assert.equal(
    value.selectedBrowserImageDigest,
    runtimeImages.browserChromiumIme,
  );
});

test('attempt manifest rejects matrix substitution and missing USB parent authority', () => {
  assert.throws(() => createAttemptManifest({
    ladder: ladder(),
    ladderManifestSha256: digest('b'),
    runId: 'vusb-attempt-test-0002',
    axis: 'control',
    browser: 'chromium',
    sequence: 3,
    profileId: 'external_input_virtual',
    childProject: 'hmn-ext-vusb-attempt-test-0002-m3',
    parentProject: 'hmn-vusb-parent-vusb-attempt-test-0002',
  }), /canonical matrix cell/);
  assert.throws(() => createAttemptManifest({
    ladder: ladder(),
    ladderManifestSha256: digest('b'),
    runId: 'vusb-attempt-test-0003',
    axis: 'ime',
    browser: 'chromium',
    sequence: 3,
    profileId: 'external_input_vusb',
    locale: 'ko-KR',
    childProject: 'hmn-ext-vusb-attempt-test-0003-m3',
  }), /parent project/);
  const valid = createAttemptManifest({
    ladder: ladder(),
    ladderManifestSha256: digest('b'),
    runId: 'vusb-attempt-test-0004',
    axis: 'control',
    browser: 'chromium',
    sequence: 3,
    profileId: 'external_input_vusb',
    childProject: 'hmn-ext-vusb-attempt-test-0004-m3',
    parentProject: 'hmn-vusb-parent-vusb-attempt-test-0004',
  });
  assert.throws(() => validateAttemptManifest({
    ...valid,
    selectedBrowserImageDigest: runtimeImages.browserFirefox,
  }, ladder()), /selected browser image/);
});

test('published ladder manifest is immutable and its raw hash is stable', async (t) => {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-vusb-manifest-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const destination = join(root, 'ladder.json');
  await writeLadderManifest({
    ladderId: 'vusb-ladder-test-0002',
    modelId: 'reference-relative-v1',
    catalogSha256: digest('a'),
    runtimeImages,
    destination,
  });
  const first = await loadLadderManifest(destination);
  assert.match(first.sha256, /^sha256:[a-f0-9]{64}$/);
  await assert.rejects(() => writeLadderManifest({
    ladderId: 'vusb-ladder-test-0002',
    modelId: 'reference-relative-v1',
    catalogSha256: digest('a'),
    runtimeImages,
    destination,
  }), /EEXIST/);
  assert.equal((await readFile(destination, 'utf8')).includes('"matrixSha256"'), true);
});
