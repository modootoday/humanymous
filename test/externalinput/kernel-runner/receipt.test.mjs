import assert from 'node:assert/strict';
import test from 'node:test';

import { validateGuestReceipt, validateRunnerReceipt } from './receipt.mjs';

const guest = {
  schemaVersion: 1,
  kind: 'kernel-vusb-smoke',
  status: 'PASS',
  kernel: '6.12.96+deb13-amd64',
  transport: 'dummy_hcd',
  deviceNodeCount: 6,
  deviceNodes: [
    '/dev/ttyGS0', '/dev/hidg0', '/dev/hidg1',
    '/dev/ttyACM0', '/dev/input/event0', '/dev/input/event1',
  ],
  drivers: { command: 'cdc_acm', keyboard: 'hid-generic', pointer: 'hid-generic' },
  neutralRelease: true,
  configfsCleanup: true,
  physicalUsb: false,
};

const runner = {
  schemaVersion: 1,
  kind: 'kernel-runner',
  status: 'PASS',
  accelerator: 'kvm',
  qemuVersion: '10.0.0',
  kernelSha256: `sha256:${'a'.repeat(64)}`,
  initramfsSha256: `sha256:${'b'.repeat(64)}`,
  guestReceipt: 'guest-smoke.json',
  consoleLog: 'console.log',
};

test('accepts an independent-kernel six-device smoke receipt', () => {
  assert.equal(validateGuestReceipt(guest), true);
  assert.equal(validateRunnerReceipt(runner), true);
});

test('rejects a missing device and physical-evidence overclaim', () => {
  assert.throws(
    () => validateGuestReceipt({
      ...guest,
      deviceNodeCount: 5,
      deviceNodes: guest.deviceNodes.slice(0, 5),
      physicalUsb: true,
    }),
    /exactly six/,
  );
});

test('rejects mutable or malformed runner identities', () => {
  assert.throws(
    () => validateRunnerReceipt({ ...runner, kernelSha256: 'latest' }),
    /artifact identity/,
  );
});
