import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { createRunReceipt } from './run-receipt.mjs';

const digest = (character) => `sha256:${character.repeat(64)}`;

test('run receipt binds emulated USB result to independent seat events', async (t) => {
  const root = await mkdtemp(join(tmpdir(), 'humanymous-run-receipt-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const runId = 'vusb-run-receipt';
  const resultPath = join(root, 'result.json');
  const seatEvidencePath = join(root, 'seat.json');
  const destination = join(root, 'run.json');
  await writeFile(resultPath, JSON.stringify({
    runId,
    profileId: 'external_input_vusb',
    status: 'PASS',
    measurement: { verdict: 'CHALLENGE' },
    control: { inputBackend: 'usb-hid-emulated' },
    purity: { xtestEnabled: false, usbAssigned: true, uinputPresent: false },
    usb: {
      required: true,
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
    },
  }));
  await writeFile(seatEvidencePath, JSON.stringify({
    schemaVersion: 'humanymous.virtual-usb-seat-evidence/v1',
    runId,
    devices: {
      keyboard: { target: 'vusb-keyboard', rdev: '1001' },
      pointer: { target: 'vusb-pointer', rdev: '1002' },
    },
    imePolicyFileSha256: '',
    keyboardTransitions: Array.from({ length: 20 }, (_, index) => ({
      code: 30,
      value: index % 2 === 0 ? 1 : 0,
    })),
    sequenceComplete: false,
    keyboardEvents: 20,
    pointerEvents: 20,
    syncEvents: 20,
    records: 60,
    eventStreamSha256: 'a'.repeat(64),
  }));
  const receipt = await createRunReceipt({
    runId,
    resultPath,
    seatEvidencePath,
    destination,
  });
  assert.equal(receipt.status, 'PASS');
  assert.match(receipt.seatEvidenceSha256, /^sha256:[a-f0-9]{64}$/);
});
