// SoT-41 canonical execution contract. This file deliberately contains no
// browser automation dependency and no detector/scoring knowledge.

export const SCHEMA_VERSION = '2.0.0';
export const EXECUTION_KIND = 'docker-external-input';
export const STRATEGY_VERSION = '1.0.0';
export const TASK_SUITE_VERSION = '1.0.0';

export const CANONICAL_MODES = Object.freeze([
  Object.freeze({
    sequence: 1,
    profileId: 'external_input_virtual',
    observation: 'framebuffer',
    inputBackend: 'rfb-xtest',
    domRequired: false,
    usbRequired: false,
  }),
  Object.freeze({
    sequence: 2,
    profileId: 'external_input_dom_virtual',
    observation: 'dom+framebuffer',
    inputBackend: 'rfb-xtest',
    domRequired: true,
    usbRequired: false,
  }),
  Object.freeze({
    sequence: 3,
    profileId: 'external_input_usb',
    observation: 'framebuffer',
    inputBackend: 'usb-hid',
    domRequired: false,
    usbRequired: true,
  }),
  Object.freeze({
    sequence: 4,
    profileId: 'external_input_dom_usb',
    observation: 'dom+framebuffer',
    inputBackend: 'usb-hid',
    domRequired: true,
    usbRequired: true,
  }),
]);

export const VIRTUAL_USB_MODES = Object.freeze([
  CANONICAL_MODES[0],
  CANONICAL_MODES[1],
  Object.freeze({
    sequence: 3,
    profileId: 'external_input_vusb',
    observation: 'framebuffer',
    inputBackend: 'usb-hid-emulated',
    domRequired: false,
    usbRequired: true,
    usbOrigin: 'kernel-emulated',
  }),
  Object.freeze({
    sequence: 4,
    profileId: 'external_input_dom_vusb',
    observation: 'dom+framebuffer',
    inputBackend: 'usb-hid-emulated',
    domRequired: true,
    usbRequired: true,
    usbOrigin: 'kernel-emulated',
  }),
]);

export const TASK_SUITE = Object.freeze([
  Object.freeze({ id: 'read-select', targetTokens: ['choice-correct'] }),
  Object.freeze({ id: 'form-correction', targetTokens: ['synthetic-form'] }),
  Object.freeze({ id: 'multi-step-navigation', targetTokens: ['nav-branch', 'nav-return'] }),
  Object.freeze({ id: 'idle-resume', targetTokens: ['resume-action'] }),
  Object.freeze({ id: 'visible-challenge', targetTokens: ['challenge-action'], optional: true }),
]);

const byId = new Map(
  [...CANONICAL_MODES, ...VIRTUAL_USB_MODES].map((mode) => [mode.profileId, mode]),
);

export function modeFor(profileId) {
  const mode = byId.get(profileId);
  if (!mode) throw new TypeError(`unknown external-input profile: ${profileId}`);
  return mode;
}

export function assertCanonicalSequence(results) {
  return assertSequence(results, CANONICAL_MODES);
}

export function assertVirtualUsbSequence(results) {
  return assertSequence(results, VIRTUAL_USB_MODES);
}

function assertSequence(results, modes) {
  if (!Array.isArray(results) || results.length !== modes.length) {
    throw new TypeError(`expected exactly ${modes.length} mode results`);
  }
  for (let index = 0; index < modes.length; index += 1) {
    const expected = modes[index];
    const actual = results[index];
    if (actual?.sequence !== expected.sequence || actual?.profileId !== expected.profileId) {
      throw new TypeError(
        `mode ${index + 1} must be ${expected.profileId} at sequence ${expected.sequence}`,
      );
    }
  }
}

export function assertAdapterPair(mode, observation, input) {
  if (!observation || observation.kind !== mode.observation) {
    throw new TypeError(`${mode.profileId} requires ${mode.observation} observation`);
  }
  if (!input || input.backend !== mode.inputBackend) {
    throw new TypeError(`${mode.profileId} requires ${mode.inputBackend} input`);
  }
  if (Boolean(observation.domEnabled) !== mode.domRequired) {
    throw new TypeError(`${mode.profileId} DOM presence does not match its canonical mode`);
  }
  if (mode.inputBackend === 'usb-hid' && input.usbPhysical !== true) {
    throw new TypeError(`${mode.profileId} requires physical USB input`);
  }
  if (mode.inputBackend === 'usb-hid-emulated' &&
      (input.usbPhysical !== false || input.usbEmulated !== true)) {
    throw new TypeError(`${mode.profileId} requires kernel-emulated USB input`);
  }
  if (mode.inputBackend === 'rfb-xtest' &&
      (input.usbPhysical !== false || input.usbEmulated === true)) {
    throw new TypeError(`${mode.profileId} must not receive USB input`);
  }
}

export function assertEngine(engine) {
  if (engine !== 'chromium' && engine !== 'firefox') {
    throw new TypeError('engine must be chromium or firefox');
  }
  return engine;
}
