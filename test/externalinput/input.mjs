import { CapabilityUnavailableError, ContractViolationError } from './errors.mjs';

const SHA256 = /^[a-f0-9]{64}$/;
const HEX_ID = /^[a-f0-9]{4}$/;
const USB_ATTESTATION_FIELDS = new Set([
  'vid', 'pid', 'serialSha256', 'descriptorSha256', 'topologySha256',
  'firmwareSha256', 'dedicatedSeat', 'seatEventObserved', 'physicalUsb',
  'uinputPresent', 'interfaceSet', 'exclusiveAssignment', 'emergencyStopReady',
  'deadManReleaseMs',
]);
const VIRTUAL_USB_ATTESTATION_FIELDS = new Set([
  'contractVersion', 'runId', 'modelId', 'authority', 'catalogRevision', 'catalogSha256',
  'imageIndexDigest', 'platformManifestDigest', 'configDigest',
  'profileManifestSha256', 'descriptorSetId', 'protocolContractSha256',
  'hidGatewayImageDigest', 'descriptorSha256', 'topologySha256',
  'kernelConfigSha256', 'exclusiveAssignment',
  'uinputPresent', 'emulationAttested', 'physicalAttested', 'physicalUsb',
  'kernelEmulated', 'transport', 'deadManReleaseMs',
  'canonical', 'baselineEligible', 'releaseAttested', 'seedSha256',
]);

export function assertUsbAttestation(attestation) {
  if (!attestation || typeof attestation !== 'object') {
    throw new CapabilityUnavailableError('usb-hid', 'physical USB HID attestation is missing');
  }
  for (const field of Object.keys(attestation)) {
    if (!USB_ATTESTATION_FIELDS.has(field)) {
      throw new CapabilityUnavailableError('usb-attestation', `undeclared USB attestation field: ${field}`);
    }
  }
  const hashes = ['serialSha256', 'descriptorSha256', 'topologySha256', 'firmwareSha256'];
  if (!HEX_ID.test(attestation.vid || '') || !HEX_ID.test(attestation.pid || '')) {
    throw new CapabilityUnavailableError('usb-hid', 'USB VID/PID attestation is invalid');
  }
  if (hashes.some((field) => !SHA256.test(attestation[field] || ''))) {
    throw new CapabilityUnavailableError('usb-hid', 'USB identity or firmware hash is invalid');
  }
  if (attestation.dedicatedSeat !== true || attestation.seatEventObserved !== true) {
    throw new CapabilityUnavailableError('dedicated-seat', 'USB HID is not proven on an exclusive seat');
  }
  if (attestation.physicalUsb !== true || attestation.uinputPresent === true) {
    throw new CapabilityUnavailableError('physical-usb', 'virtual input cannot satisfy USB modes');
  }
  if (attestation.interfaceSet !== 'command+keyboard+pointer') {
    throw new CapabilityUnavailableError('usb-interfaces', 'USB interface set is not exact');
  }
  if (attestation.exclusiveAssignment !== true || attestation.emergencyStopReady !== true ||
      !Number.isInteger(attestation.deadManReleaseMs) ||
      attestation.deadManReleaseMs < 100 || attestation.deadManReleaseMs > 2_000) {
    throw new CapabilityUnavailableError(
      'usb-safety',
      'USB exclusive assignment, emergency stop, or dead-man release is not proven',
    );
  }
  return Object.freeze({ ...attestation });
}

export function assertVirtualUsbAttestation(attestation) {
  if (!attestation || typeof attestation !== 'object' || Array.isArray(attestation)) {
    throw new CapabilityUnavailableError('virtual-usb-hid', 'virtual USB attestation is missing');
  }
  for (const field of Object.keys(attestation)) {
    if (!VIRTUAL_USB_ATTESTATION_FIELDS.has(field)) {
      throw new CapabilityUnavailableError(
        'virtual-usb-attestation',
        `undeclared virtual USB attestation field: ${field}`,
      );
    }
  }
  for (const field of [
    'catalogSha256', 'imageIndexDigest', 'platformManifestDigest', 'configDigest',
    'profileManifestSha256', 'protocolContractSha256', 'hidGatewayImageDigest',
    'descriptorSha256', 'topologySha256', 'kernelConfigSha256',
  ]) {
    if (!/^sha256:[a-f0-9]{64}$/.test(attestation[field] || '')) {
      throw new CapabilityUnavailableError('virtual-usb-hid', `${field} is invalid`);
    }
  }
  const developmentAuthority =
    attestation.authority === 'seed-bound-development' &&
    attestation.canonical === false &&
    attestation.baselineEligible === false &&
    attestation.releaseAttested === false &&
    /^sha256:[a-f0-9]{64}$/.test(attestation.seedSha256 || '');
  const releaseAuthority = attestation.authority === 'project-reference' &&
    !Object.hasOwn(attestation, 'canonical') &&
    !Object.hasOwn(attestation, 'baselineEligible') &&
    !Object.hasOwn(attestation, 'releaseAttested') &&
    !Object.hasOwn(attestation, 'seedSha256');
  if (attestation.contractVersion !== 'humanymous.virtual-usb-profile/v1' ||
      !/^[a-z0-9][a-z0-9-]{5,63}$/.test(attestation.runId || '') ||
      (!developmentAuthority && !releaseAuthority) ||
      attestation.descriptorSetId !== 'reference-relative-v1' ||
      attestation.transport !== 'dummy-hcd' ||
      attestation.kernelEmulated !== true ||
      attestation.emulationAttested !== true ||
      attestation.physicalAttested !== false ||
      attestation.physicalUsb !== false ||
      attestation.uinputPresent !== false ||
      attestation.exclusiveAssignment !== true ||
      !Number.isInteger(attestation.deadManReleaseMs) ||
      attestation.deadManReleaseMs < 100 || attestation.deadManReleaseMs > 500) {
    throw new CapabilityUnavailableError(
      'virtual-usb-hid',
      'kernel-emulated USB topology, exclusive mapping, or safety proof is incomplete',
    );
  }
  return Object.freeze({ ...attestation });
}

function baseInput({
  backend,
  usbPhysical,
  send,
  release,
  firewall,
  attestation = null,
  sleep = (durationMs) => new Promise((resolve) => setTimeout(resolve, durationMs)),
}) {
  if (typeof send !== 'function' || typeof release !== 'function') {
    throw new TypeError('input adapter requires send and release functions');
  }
  if (!firewall || typeof firewall.validate !== 'function') {
    throw new TypeError('input adapter requires an action firewall');
  }
  let released = false;
  let events = 0;

  return Object.freeze({
    backend,
    usbPhysical,
    usbEmulated: optionsUsbEmulated(backend),
    attestation,
    get events() { return events; },
    async perform(action) {
      if (released) throw new ContractViolationError('input adapter was already released');
      const safeAction = firewall.validate(action);
      if (safeAction.kind === 'pause') {
        await sleep(safeAction.durationMs);
        return;
      }
      await send(safeAction);
      events += 1;
    },
    async releaseAll() {
      if (released) return;
      await release(Object.freeze({ kind: 'releaseAll' }));
      released = true;
    },
  });
}

function optionsUsbEmulated(backend) {
  return backend === 'usb-hid-emulated';
}

export function createVirtualInputAdapter(options) {
  if (options?.usbAttestation) {
    throw new ContractViolationError('virtual mode must not receive USB attestation');
  }
  return baseInput({
    ...options,
    backend: 'rfb-xtest',
    usbPhysical: false,
  });
}

export function createUsbInputAdapter(options) {
  const attestation = assertUsbAttestation(options?.usbAttestation);
  if (options?.rfbInputEnabled === true || options?.xtestEnabled === true) {
    throw new ContractViolationError('USB mode must reject RFB/XTEST input');
  }
  return baseInput({
    ...options,
    backend: 'usb-hid',
    usbPhysical: true,
    attestation,
  });
}

export function createEmulatedUsbInputAdapter(options) {
  const attestation = assertVirtualUsbAttestation(options?.usbAttestation);
  if (options?.rfbInputEnabled === true || options?.xtestEnabled === true) {
    throw new ContractViolationError('virtual USB mode must reject RFB/XTEST input');
  }
  return baseInput({
    ...options,
    backend: 'usb-hid-emulated',
    usbPhysical: false,
    attestation,
  });
}
