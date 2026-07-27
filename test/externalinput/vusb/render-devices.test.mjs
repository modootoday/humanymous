import assert from 'node:assert/strict';
import test from 'node:test';
import { renderDeviceOverride } from './render-devices.mjs';

function entry(path, minor) {
  return {
    hostPath: path,
    deviceHex: `f0:${minor}`,
    sysfsDevice: `240:${Number.parseInt(minor, 16)}`,
    sysfsPath: `/sys/devices/test/${minor}`,
    driverPath: `/sys/bus/test/drivers/${minor}`,
  };
}

function receipt() {
  return {
    kind: 'prepare',
    stableObservations: 2,
    deviceIdentityCount: 6,
    driverContractVerified: true,
    gadget: {
      command: entry('/dev/ttyGS0', '1'),
      keyboard: entry('/dev/hidg0', '2'),
      pointer: entry('/dev/hidg1', '3'),
    },
    host: {
      command: entry('/dev/serial/by-id/vusb-command', '4'),
      keyboard: entry('/dev/input/by-id/vusb-event-kbd', '5'),
      pointer: entry('/dev/input/by-id/vusb-event-mouse', '6'),
    },
  };
}

test('generated Compose override contains exactly the six canonical mappings', () => {
  const override = renderDeviceOverride(receipt());
  assert.deepEqual(Object.keys(override.services), [
    'external-vusb-gateway', 'external-vusb-broker', 'external-display',
  ]);
  assert.equal(override.services['external-vusb-gateway'].devices.length, 3);
  assert.equal(override.services['external-vusb-broker'].devices.length, 1);
  assert.equal(override.services['external-display'].devices.length, 2);
  assert.equal(JSON.stringify(override).includes('cdi'), false);
});

test('duplicate or broad mappings fail closed', () => {
  const duplicated = receipt();
  duplicated.host.pointer = duplicated.host.keyboard;
  assert.throws(() => renderDeviceOverride(duplicated), /six device mappings must be distinct/);

  const broad = receipt();
  broad.host.keyboard.hostPath = '/host-dev/input';
  assert.throws(() => renderDeviceOverride(broad), /invalid path/);
});
