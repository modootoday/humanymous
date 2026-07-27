import assert from 'node:assert/strict';
import test from 'node:test';
import { sysfsDeviceKey, validateDriverContract } from './discover.mjs';

test('sysfs character-device keys use decimal major and minor numbers', () => {
  assert.equal(sysfsDeviceKey('0a:1f'), '10:31');
  assert.equal(sysfsDeviceKey('00:00'), '0:0');
  assert.throws(() => sysfsDeviceKey('10'), /identifier is invalid/);
});

test('driver contract requires the CDC and USB HID stacks on six distinct nodes', () => {
  const record = (sysfsDevice, driverPath) => ({ sysfsDevice, driverPath });
  const snapshot = {
    gadget: {
      command: record('240:1', ''),
      keyboard: record('240:2', ''),
      pointer: record('240:3', ''),
    },
    host: {
      command: record('166:0', '/sys/bus/usb/drivers/cdc_acm'),
      keyboard: record('13:64', '/sys/bus/hid/drivers/hid-generic'),
      pointer: record('13:65', '/sys/bus/usb/drivers/usbhid'),
    },
  };
  assert.equal(validateDriverContract(snapshot), true);
  snapshot.host.pointer.driverPath = '/sys/bus/input/drivers/evdev';
  assert.throws(() => validateDriverContract(snapshot), /USB HID stack/);
});
