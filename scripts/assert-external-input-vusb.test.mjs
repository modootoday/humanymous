import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { assertVirtualUsbLadder } from './assert-external-input-vusb.mjs';
import {
  IME_AXIS_VERSION,
  IME_LOCALES,
  vectorFor,
} from '../test/externalinput/vusb/ime-contract.mjs';
import { sha256 } from '../test/externalinput/vusb/common.mjs';
import { RUNTIME_IMAGE_FIELDS } from '../test/externalinput/vusb/catalog.mjs';
import {
  createAttemptManifest,
  createLadderManifest,
} from '../test/externalinput/vusb/manifest.mjs';

const digest = (character) => `sha256:${character.repeat(64)}`;

function virtualAttestation(runId) {
  return {
    contractVersion: 'humanymous.virtual-usb-profile/v1',
    runId,
    modelId: 'reference-relative-v1',
    authority: 'project-reference',
    catalogRevision: '2026-07-26.1',
    catalogSha256: digest('1'),
    imageIndexDigest: digest('2'),
    platformManifestDigest: digest('2'),
    configDigest: digest('3'),
    profileManifestSha256: digest('4'),
    descriptorSetId: 'reference-relative-v1',
    protocolContractSha256: digest('5'),
    hidGatewayImageDigest: digest('6'),
    descriptorSha256: digest('7'),
    topologySha256: digest('8'),
    kernelConfigSha256: digest('9'),
    exclusiveAssignment: true,
    uinputPresent: false,
    emulationAttested: true,
    physicalAttested: false,
    physicalUsb: false,
    kernelEmulated: true,
    transport: 'dummy-hcd',
    deadManReleaseMs: 500,
  };
}

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-vusb-ladder-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const runtimeImages = Object.fromEntries(
    RUNTIME_IMAGE_FIELDS.map((name, index) =>
      [name, digest(String((index % 9) + 1))]),
  );
  const ladder = createLadderManifest({
    ladderId: 'vusb-aggregate-test-0001',
    modelId: 'reference-relative-v1',
    catalogSha256: digest('a'),
    runtimeImages,
  });
  const ladderRaw = `${JSON.stringify(ladder, null, 2)}\n`;
  await mkdir(join(root, 'manifest'));
  await writeFile(join(root, 'manifest', 'ladder.json'), ladderRaw);
  async function writeAttempt({
    axis,
    engine,
    sequence,
    profileId,
    runId,
    locale = '',
  }) {
    const episode = axis === 'control' ? `m${sequence}` : `ime-${locale.split('-')[0]}`;
    const directory = join(root, `${engine}-${episode}`, 'manifest');
    await mkdir(directory, { recursive: true });
    const attempt = createAttemptManifest({
      ladder,
      ladderManifestSha256: `sha256:${sha256(ladderRaw)}`,
      runId,
      axis,
      browser: engine,
      sequence,
      profileId,
      locale,
      childProject: `hmn-ext-${runId}-m${sequence}`.slice(0, 63),
      parentProject: sequence >= 3 ? `hmn-vusb-parent-${runId}`.slice(0, 63) : '',
    });
    const raw = `${JSON.stringify(attempt, null, 2)}\n`;
    await writeFile(join(directory, 'attempt.json'), raw);
    return `sha256:${sha256(raw)}`;
  }
  const modes = [
    ['external_input_virtual', 'framebuffer', 'rfb-xtest'],
    ['external_input_dom_virtual', 'dom+framebuffer', 'rfb-xtest'],
    ['external_input_vusb', 'framebuffer', 'usb-hid-emulated'],
    ['external_input_dom_vusb', 'dom+framebuffer', 'usb-hid-emulated'],
  ];
  for (const engine of ['chromium', 'firefox']) {
    for (let index = 0; index < modes.length; index += 1) {
      const sequence = index + 1;
      const [profileId, observation, inputBackend] = modes[index];
      const runId = `vusb-test-${engine}-${sequence}`;
      const usb = sequence <= 2
        ? { required: false, attested: false }
        : { required: true, ...virtualAttestation(runId) };
      const result = {
        schemaVersion: '2.0.0',
        runId,
        sequence,
        profileId,
        browser: { engine },
        control: { observation, inputBackend },
        purity: {
          uinputPresent: false,
          mixedInputBackends: false,
          xtestEnabled: sequence <= 2,
          usbAssigned: sequence > 2,
        },
        usb,
        measurement: { verdict: 'CHALLENGE' },
        status: 'PASS',
      };
      const resultRaw = `${JSON.stringify(result)}\n`;
      await writeFile(join(root, `${engine}-m${sequence}.result.json`), resultRaw);
      const attemptManifestSha256 = await writeAttempt({
        axis: 'control',
        engine,
        sequence,
        profileId,
        runId,
      });
      if (sequence > 2) {
        await writeFile(
          join(root, `${engine}-m${sequence}.terminal.json`),
          `${JSON.stringify({
            schemaVersion: 'humanymous.virtual-usb-receipt/v1',
            kind: 'terminal',
            runId,
            canonical: true,
            status: 'PASS',
            profileId,
            axis: 'control',
            browserEngine: engine,
            locale: '',
            vectorId: '',
            resultSha256: `sha256:${sha256(resultRaw)}`,
            selectedBrowserImageDigest:
              ladder.runtimeImages[
                engine === 'chromium'
                  ? sequence === 4 ? 'browserChromiumDom' : 'browserChromium'
                  : sequence === 4 ? 'browserFirefoxDom' : 'browserFirefox'
              ],
            evidence: {
              ladderManifestSha256: `sha256:${sha256(ladderRaw)}`,
              attemptManifestSha256,
            },
          })}\n`,
        );
      }
    }
    for (const locale of IME_LOCALES) {
      const imeRunId = `ime-${engine}-${locale.toLowerCase()}`;
      const imeRaw = `${JSON.stringify({
          axisVersion: IME_AXIS_VERSION,
          runId: imeRunId,
          browser: engine,
          locale,
          vectorId: vectorFor(locale).vectorId,
          inputBackend: 'usb-hid-emulated',
          actorRole: 'input-only',
          hidUsageCount: vectorFor(locale).usages.length + vectorFor(locale).commit.length,
          status: 'ACTED',
        })}\n`;
      await writeFile(join(root, `${engine}-${locale}.ime.result.json`), imeRaw);
      const attemptManifestSha256 = await writeAttempt({
        axis: 'ime',
        engine,
        sequence: 3,
        profileId: 'external_input_vusb',
        runId: imeRunId,
        locale,
      });
      await writeFile(
        join(root, `${engine}-${locale}.ime.terminal.json`),
        `${JSON.stringify({
          schemaVersion: 'humanymous.virtual-usb-receipt/v1',
          kind: 'terminal',
          runId: imeRunId,
          canonical: true,
          status: 'PASS',
          measurementVerdict: 'NOT_APPLICABLE',
          profileId: 'ime-composition-vusb',
          axis: 'ime-composition-vusb',
          browserEngine: engine,
          locale,
          vectorId: vectorFor(locale).vectorId,
          resultSha256: `sha256:${sha256(imeRaw)}`,
          selectedBrowserImageDigest:
            ladder.runtimeImages[
              engine === 'chromium' ? 'browserChromiumIme' : 'browserFirefoxIme'
            ],
          evidence: {
            ladderManifestSha256: `sha256:${sha256(ladderRaw)}`,
            attemptManifestSha256,
          },
        })}\n`,
      );
    }
  }
  return root;
}

test('assertion accepts the exact Chromium then Firefox four-mode virtual USB ladder', async (t) => {
  const root = await fixture(t);
  const result = await assertVirtualUsbLadder(root);
  assert.equal(result.canonical, true, result.failures.join('\n'));
  assert.equal(result.measured, 8);
  assert.equal(result.ime.pass, 6);
});

test('physical claims or missing terminal cleanup invalidate the ladder', async (t) => {
  const root = await fixture(t);
  const path = join(root, 'chromium-m3.result.json');
  const result = JSON.parse(await import('node:fs/promises').then(({ readFile }) => readFile(path, 'utf8')));
  result.usb.physicalUsb = true;
  await writeFile(path, `${JSON.stringify(result)}\n`);
  const asserted = await assertVirtualUsbLadder(root);
  assert.equal(asserted.canonical, false);
  assert.match(asserted.failures.join('\n'), /kernel-emulated USB topology/);
});

test('swapped run or input-method terminal evidence invalidates the ladder', async (t) => {
  const root = await fixture(t);
  const terminalPath = join(root, 'firefox-ja-JP.ime.terminal.json');
  const terminal = JSON.parse(
    await import('node:fs/promises').then(({ readFile }) => readFile(terminalPath, 'utf8')),
  );
  terminal.runId = 'ime-firefox-zh-cn';
  await writeFile(terminalPath, `${JSON.stringify(terminal)}\n`);
  const asserted = await assertVirtualUsbLadder(root);
  assert.equal(asserted.canonical, false);
  assert.match(asserted.ime.failures.join('\n'), /cleanup receipt mismatch/);
});
