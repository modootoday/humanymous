import { readFile } from 'node:fs/promises';
import { exactObject } from './common.mjs';
import { parseStrictJson } from './strict-json.mjs';

export function validateSeatEvidence(raw, runId, {
  minimumKeyboardEvents = 1,
  minimumPointerEvents = 1,
} = {}) {
  const evidence = parseStrictJson(raw, 'seat event evidence');
  exactObject(evidence, [
    'schemaVersion', 'runId', 'devices', 'imePolicyFileSha256',
    'keyboardTransitions', 'sequenceComplete',
    'keyboardEvents', 'pointerEvents',
    'syncEvents', 'records', 'eventStreamSha256',
  ], 'seat event evidence');
  exactObject(evidence.devices, ['keyboard', 'pointer'], 'seat devices');
  exactObject(evidence.devices.keyboard, ['target', 'rdev'], 'seat keyboard device');
  exactObject(evidence.devices.pointer, ['target', 'rdev'], 'seat pointer device');
  if (!Array.isArray(evidence.keyboardTransitions) ||
      evidence.keyboardTransitions.length > 128) {
    throw new TypeError('seat keyboard transitions are invalid');
  }
  for (const transition of evidence.keyboardTransitions) {
    exactObject(transition, ['code', 'value'], 'seat keyboard transition');
    if (!Number.isInteger(transition.code) || transition.code < 1 ||
        transition.code > 767 || ![0, 1, 2].includes(transition.value)) {
      throw new TypeError('seat keyboard transition is invalid');
    }
  }
  if (evidence.schemaVersion !== 'humanymous.virtual-usb-seat-evidence/v1' ||
      evidence.runId !== runId ||
      evidence.devices.keyboard.target !== 'vusb-keyboard' ||
      evidence.devices.pointer.target !== 'vusb-pointer' ||
      !/^[1-9][0-9]*$/.test(evidence.devices.keyboard.rdev || '') ||
      !/^[1-9][0-9]*$/.test(evidence.devices.pointer.rdev || '') ||
      evidence.devices.keyboard.rdev === evidence.devices.pointer.rdev ||
      !(/^sha256:[a-f0-9]{64}$/.test(evidence.imePolicyFileSha256 || '') ||
        evidence.imePolicyFileSha256 === '') ||
      typeof evidence.sequenceComplete !== 'boolean' ||
      !Number.isInteger(evidence.keyboardEvents) ||
      evidence.keyboardEvents < minimumKeyboardEvents ||
      evidence.keyboardTransitions.length !== evidence.keyboardEvents ||
      !Number.isInteger(evidence.pointerEvents) ||
      evidence.pointerEvents < minimumPointerEvents ||
      !Number.isInteger(evidence.syncEvents) || evidence.syncEvents < 1 ||
      !Number.isInteger(evidence.records) ||
      evidence.records < evidence.keyboardEvents + evidence.pointerEvents ||
      !/^[a-f0-9]{64}$/.test(evidence.eventStreamSha256 || '')) {
    throw new TypeError('independent seat event evidence is incomplete or unbound');
  }
  return Object.freeze(evidence);
}

export async function readSeatEvidence(path, runId, options = {}) {
  const deadline = Date.now() + (options.timeoutMs ?? 5_000);
  let lastError;
  do {
    try {
      const raw = await readFile(path, 'utf8');
      return Object.freeze({
        evidence: validateSeatEvidence(raw, runId, options),
        raw,
      });
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 50));
  } while (Date.now() < deadline);
  throw lastError;
}
