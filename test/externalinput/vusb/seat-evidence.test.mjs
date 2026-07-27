import assert from 'node:assert/strict';
import test from 'node:test';
import { validateSeatEvidence } from './seat-evidence.mjs';

const evidence = {
  schemaVersion: 'humanymous.virtual-usb-seat-evidence/v1',
  runId: 'vusb-seat-test',
  devices: {
    keyboard: { target: 'vusb-keyboard', rdev: '1001' },
    pointer: { target: 'vusb-pointer', rdev: '1002' },
  },
  imePolicyFileSha256: '',
  keyboardTransitions: Array.from({ length: 14 }, (_, index) => ({
    code: 30,
    value: index % 2 === 0 ? 1 : 0,
  })),
  sequenceComplete: false,
  keyboardEvents: 14,
  pointerEvents: 8,
  syncEvents: 10,
  records: 32,
  eventStreamSha256: 'a'.repeat(64),
};

test('seat evidence is run-bound and requires both keyboard and pointer events', () => {
  assert.equal(validateSeatEvidence(JSON.stringify(evidence), evidence.runId).records, 32);
  assert.throws(
    () => validateSeatEvidence(JSON.stringify({ ...evidence, pointerEvents: 0 }), evidence.runId),
    /incomplete or unbound/,
  );
});
