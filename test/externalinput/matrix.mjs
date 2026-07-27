import { VIRTUAL_USB_MODES } from './contracts.mjs';
import { IME_LOCALES } from './vusb/ime-contract.mjs';

export const BROWSERS = Object.freeze(['chromium', 'firefox']);

export const VIRTUAL_USB_CONTROL_CELLS = Object.freeze(
  BROWSERS.flatMap((browser) => VIRTUAL_USB_MODES.map((mode) => Object.freeze({
    browser,
    sequence: mode.sequence,
    profileId: mode.profileId,
    observation: mode.observation,
    inputBackend: mode.inputBackend,
    domRequired: mode.domRequired,
    usbOrigin: mode.usbOrigin || 'none',
  }))),
);

export const IME_CELLS = Object.freeze(
  BROWSERS.flatMap((browser) => IME_LOCALES.map((locale) => Object.freeze({
    browser,
    locale,
    profileId: 'external_input_vusb',
    sequence: 3,
    observation: 'framebuffer',
    inputBackend: 'usb-hid-emulated',
    usbOrigin: 'kernel-emulated',
  }))),
);

export function matrixRows(axis) {
  if (axis === 'control') return VIRTUAL_USB_CONTROL_CELLS;
  if (axis === 'ime') return IME_CELLS;
  throw new TypeError('matrix axis must be control or ime');
}
