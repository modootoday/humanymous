import { timingSafeEqual } from 'node:crypto';

export const PROTOCOL_VERSION = '1.0.0';
export const MAX_FRAME_BYTES = 4 * 1024;
export const MAX_DEADLINE_AHEAD_MS = 5_000;

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const SHA256 = /^[a-f0-9]{64}$/;
const HEX_ID = /^[a-f0-9]{4}$/;
const ALLOWED_KEYS = new Set([
  'Tab', 'Enter', 'Space', 'Escape', 'Backspace', 'Delete',
  'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight',
  ...'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.split(''),
]);
const SYNTHETIC_TEXTS = new Set(['humanymous synthetic task']);

function object(value, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function exactFields(value, fields, label) {
  for (const field of Object.keys(value)) {
    if (!fields.has(field)) throw new TypeError(`${label} has unknown field: ${field}`);
  }
}

function boundedInteger(value, minimum, maximum, label) {
  if (!Number.isInteger(value) || value < minimum || value > maximum) {
    throw new TypeError(`${label} is outside its integer bound`);
  }
  return value;
}

function fixedStringEqual(left, right) {
  if (typeof left !== 'string' || typeof right !== 'string') return false;
  const a = Buffer.from(left);
  const b = Buffer.from(right);
  return a.length === b.length && timingSafeEqual(a, b);
}

export function parseFrame(line, label = 'frame') {
  if (typeof line !== 'string' || Buffer.byteLength(line) > MAX_FRAME_BYTES) {
    throw new TypeError(`${label} exceeds the frame bound`);
  }
  let value;
  try {
    value = JSON.parse(line);
  } catch {
    throw new TypeError(`${label} is not valid JSON`);
  }
  return object(value, label);
}

export function encodeFrame(value) {
  const frame = `${JSON.stringify(value)}\n`;
  if (Buffer.byteLength(frame) > MAX_FRAME_BYTES) {
    throw new TypeError('encoded frame exceeds the frame bound');
  }
  return frame;
}

export function validateDeadline(deadlineUnixMs, now = Date.now()) {
  boundedInteger(deadlineUnixMs, 1, Number.MAX_SAFE_INTEGER, 'deadline');
  if (deadlineUnixMs <= now) throw new TypeError('deadline expired');
  if (deadlineUnixMs - now > MAX_DEADLINE_AHEAD_MS) {
    throw new TypeError('deadline exceeds broker bound');
  }
  return deadlineUnixMs;
}

export function validateAction(action) {
  object(action, 'action');
  switch (action.kind) {
    case 'pointerMove':
      exactFields(action, new Set(['kind', 'x', 'y', 'durationMs']), 'pointerMove');
      boundedInteger(action.x, 0, 1279, 'pointerMove.x');
      boundedInteger(action.y, 0, 719, 'pointerMove.y');
      boundedInteger(action.durationMs ?? 0, 0, 2_000, 'pointerMove.durationMs');
      break;
    case 'pointerClick':
      exactFields(action, new Set(['kind', 'button', 'dwellMs']), 'pointerClick');
      if (action.button !== 'left') throw new TypeError('only the left pointer button is allowed');
      boundedInteger(action.dwellMs ?? 60, 20, 250, 'pointerClick.dwellMs');
      break;
    case 'scroll':
      exactFields(action, new Set(['kind', 'dx', 'dy']), 'scroll');
      if (action.dx !== 0) throw new TypeError('horizontal scroll is forbidden');
      boundedInteger(action.dy, -1_200, 1_200, 'scroll.dy');
      break;
    case 'keyStroke': {
      exactFields(action, new Set(['kind', 'key', 'modifiers', 'dwellMs', 'flightMs']), 'keyStroke');
      if (!ALLOWED_KEYS.has(action.key)) throw new TypeError('key is not allowlisted');
      const modifiers = action.modifiers ?? [];
      if (!Array.isArray(modifiers) || modifiers.length > 1 ||
          modifiers.some((modifier) => modifier !== 'Shift')) {
        throw new TypeError('shortcut modifier is forbidden');
      }
      boundedInteger(action.dwellMs ?? 60, 20, 250, 'keyStroke.dwellMs');
      boundedInteger(action.flightMs ?? 60, 20, 250, 'keyStroke.flightMs');
      break;
    }
    case 'typeText':
      exactFields(action, new Set(['kind', 'text']), 'typeText');
      if (!SYNTHETIC_TEXTS.has(action.text)) {
        throw new TypeError('text is not an allowlisted synthetic task value');
      }
      break;
    case 'releaseAll':
      exactFields(action, new Set(['kind']), 'releaseAll');
      break;
    default:
      throw new TypeError(`action kind is forbidden: ${String(action.kind)}`);
  }
  return Object.freeze(structuredClone(action));
}

export function validateClientEnvelope(envelope, now = Date.now()) {
  object(envelope, 'client envelope');
  exactFields(
    envelope,
    new Set(['protocolVersion', 'sequenceId', 'deadlineUnixMs', 'action']),
    'client envelope',
  );
  if (envelope.protocolVersion !== PROTOCOL_VERSION) {
    throw new TypeError('protocol version mismatch');
  }
  if (!UUID.test(envelope.sequenceId || '')) throw new TypeError('sequence ID is invalid');
  validateDeadline(envelope.deadlineUnixMs, now);
  return Object.freeze({
    protocolVersion: PROTOCOL_VERSION,
    sequenceId: envelope.sequenceId,
    deadlineUnixMs: envelope.deadlineUnixMs,
    action: validateAction(envelope.action),
  });
}

export function validateHostAttestation(attestation) {
  object(attestation, 'USB attestation');
  const fields = new Set([
    'vid', 'pid', 'serialSha256', 'descriptorSha256', 'topologySha256',
    'firmwareSha256', 'dedicatedSeat', 'seatEventObserved', 'physicalUsb',
    'uinputPresent', 'interfaceSet', 'exclusiveAssignment', 'emergencyStopReady',
    'deadManReleaseMs',
  ]);
  exactFields(attestation, fields, 'USB attestation');
  if (!HEX_ID.test(attestation.vid || '') || !HEX_ID.test(attestation.pid || '')) {
    throw new TypeError('USB VID/PID attestation is invalid');
  }
  for (const field of ['serialSha256', 'descriptorSha256', 'topologySha256', 'firmwareSha256']) {
    if (!SHA256.test(attestation[field] || '')) throw new TypeError(`${field} is invalid`);
  }
  if (attestation.dedicatedSeat !== true || attestation.seatEventObserved !== true ||
      attestation.physicalUsb !== true || attestation.uinputPresent !== false ||
      attestation.interfaceSet !== 'command+keyboard+pointer' ||
      attestation.exclusiveAssignment !== true ||
      attestation.emergencyStopReady !== true) {
    throw new TypeError('USB host/seat attestation is incomplete');
  }
  boundedInteger(attestation.deadManReleaseMs, 100, 2_000, 'deadManReleaseMs');
  return Object.freeze(structuredClone(attestation));
}

export function validateHelloAck(frame, expected) {
  object(frame, 'firmware hello acknowledgement');
  exactFields(
    frame,
    new Set(['protocolVersion', 'kind', 'sessionId', 'nonce', 'identity', 'safety']),
    'firmware hello acknowledgement',
  );
  if (frame.protocolVersion !== PROTOCOL_VERSION || frame.kind !== 'helloAck' ||
      !fixedStringEqual(frame.sessionId, expected.sessionId) ||
      !fixedStringEqual(frame.nonce, expected.nonce)) {
    throw new TypeError('firmware hello acknowledgement is not bound to this session');
  }

  const identity = object(frame.identity, 'firmware identity');
  exactFields(
    identity,
    new Set([
      'vid', 'pid', 'serialSha256', 'descriptorSha256', 'topologySha256',
      'firmwareSha256', 'interfaceSet',
    ]),
    'firmware identity',
  );
  for (const field of [
    'vid', 'pid', 'serialSha256', 'descriptorSha256', 'topologySha256',
    'firmwareSha256', 'interfaceSet',
  ]) {
    if (!fixedStringEqual(identity[field], expected.attestation[field])) {
      throw new TypeError(`firmware identity mismatch: ${field}`);
    }
  }

  validateSafety(frame.safety, expected.attestation);
  return Object.freeze({ identity: structuredClone(identity), safety: structuredClone(frame.safety) });
}

export function validateSafety(safety, attestation) {
  object(safety, 'firmware safety state');
  exactFields(
    safety,
    new Set(['emergencyStopReady', 'emergencyStopEngaged', 'deadManArmed', 'deadManReleaseMs']),
    'firmware safety state',
  );
  if (safety.emergencyStopReady !== true || safety.emergencyStopEngaged !== false ||
      safety.deadManArmed !== true ||
      safety.deadManReleaseMs !== attestation.deadManReleaseMs) {
    throw new TypeError('firmware emergency-stop/dead-man state is not ready');
  }
  return safety;
}

export function validateCommandAck(frame, expected) {
  object(frame, 'firmware command acknowledgement');
  exactFields(
    frame,
    new Set([
      'protocolVersion', 'kind', 'sessionId', 'sequence', 'commandId',
      'accepted', 'releasedAll', 'safety',
    ]),
    'firmware command acknowledgement',
  );
  if (frame.protocolVersion !== PROTOCOL_VERSION || frame.kind !== 'ack' ||
      !fixedStringEqual(frame.sessionId, expected.sessionId) ||
      frame.sequence !== expected.sequence ||
      !fixedStringEqual(frame.commandId, expected.commandId)) {
    throw new TypeError('firmware acknowledgement sequence mismatch or replay');
  }
  if (frame.accepted !== true) throw new TypeError('firmware rejected command');
  validateSafety(frame.safety, expected.attestation);
  if (expected.action.kind === 'releaseAll' && frame.releasedAll !== true) {
    throw new TypeError('firmware did not confirm all HID state released');
  }
  if (expected.action.kind !== 'releaseAll' && frame.releasedAll !== false) {
    throw new TypeError('firmware acknowledgement state is inconsistent');
  }
  return frame;
}
