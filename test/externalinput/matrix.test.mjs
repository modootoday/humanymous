import assert from 'node:assert/strict';
import test from 'node:test';
import { BROWSERS, IME_CELLS, VIRTUAL_USB_CONTROL_CELLS } from './matrix.mjs';

test('canonical matrix owns the exact English and input-method denominators', () => {
  assert.deepEqual(BROWSERS, ['chromium', 'firefox']);
  assert.equal(VIRTUAL_USB_CONTROL_CELLS.length, 8);
  assert.equal(IME_CELLS.length, 6);
  assert.deepEqual(
    VIRTUAL_USB_CONTROL_CELLS.map(({ browser, sequence }) => `${browser}:${sequence}`),
    [
      'chromium:1', 'chromium:2', 'chromium:3', 'chromium:4',
      'firefox:1', 'firefox:2', 'firefox:3', 'firefox:4',
    ],
  );
});
