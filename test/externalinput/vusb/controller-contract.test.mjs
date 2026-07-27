import assert from 'node:assert/strict';
import test from 'node:test';
import {
  VIRTUAL_USB_MODES,
  assertVirtualUsbSequence,
  modeFor,
} from '../contracts.mjs';
import { createActionFirewall } from '../firewall.mjs';
import {
  assertVirtualUsbAttestation,
  createEmulatedUsbInputAdapter,
} from '../input.mjs';

const digest = (character) => `sha256:${character.repeat(64)}`;
const attestation = Object.freeze({
  contractVersion: 'humanymous.virtual-usb-profile/v1',
  runId: 'vusb-controller-test',
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
});

test('virtual USB ladder preserves the four requested control combinations', () => {
  assert.deepEqual(VIRTUAL_USB_MODES.map(({ profileId }) => profileId), [
    'external_input_virtual',
    'external_input_dom_virtual',
    'external_input_vusb',
    'external_input_dom_vusb',
  ]);
  assert.equal(modeFor('external_input_vusb').inputBackend, 'usb-hid-emulated');
  assert.doesNotThrow(() => assertVirtualUsbSequence(
    VIRTUAL_USB_MODES.map(({ sequence, profileId }) => ({ sequence, profileId })),
  ));
});

test('emulated USB adapter is negative physical evidence and uses the typed firewall', async () => {
  const sent = [];
  const input = createEmulatedUsbInputAdapter({
    usbAttestation: attestation,
    rfbInputEnabled: false,
    xtestEnabled: false,
    firewall: createActionFirewall({ width: 1280, height: 720 }),
    send: async (action) => sent.push(action),
    release: async (action) => sent.push(action),
  });
  assert.equal(input.backend, 'usb-hid-emulated');
  assert.equal(input.usbPhysical, false);
  assert.equal(input.usbEmulated, true);
  await input.perform({ kind: 'pointerMove', x: 100, y: 100, durationMs: 100 });
  await input.perform({ kind: 'pointerClick', button: 'left', dwellMs: 60 });
  await input.releaseAll();
  assert.deepEqual(sent.map(({ kind }) => kind), ['pointerMove', 'pointerClick', 'releaseAll']);
});

test('virtual attestation rejects physical claims and missing kernel emulation', () => {
  assert.throws(
    () => assertVirtualUsbAttestation({ ...attestation, physicalUsb: true }),
    /kernel-emulated USB topology/,
  );
  assert.throws(
    () => assertVirtualUsbAttestation({ ...attestation, kernelEmulated: false }),
    /kernel-emulated USB topology/,
  );
});

test('seed-bound development attestation is accepted only with noncanonical truth flags', () => {
  const development = {
    ...attestation,
    authority: 'seed-bound-development',
    canonical: false,
    baselineEligible: false,
    releaseAttested: false,
    seedSha256: digest('a'),
  };
  assert.doesNotThrow(() => assertVirtualUsbAttestation(development));
  assert.throws(
    () => assertVirtualUsbAttestation({ ...development, baselineEligible: true }),
    /kernel-emulated USB topology/,
  );
  assert.throws(
    () => assertVirtualUsbAttestation({
      ...attestation,
      canonical: false,
      baselineEligible: false,
      releaseAttested: false,
      seedSha256: digest('a'),
    }),
    /kernel-emulated USB topology/,
  );
});
