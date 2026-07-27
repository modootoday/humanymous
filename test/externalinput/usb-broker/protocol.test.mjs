import assert from 'node:assert/strict';
import test from 'node:test';
import {
  PROTOCOL_VERSION,
  validateClientEnvelope,
  validateCommandAck,
  validateHelloAck,
  validateHostAttestation,
} from './protocol.mjs';
import { attestation, safety } from './test-fixtures.mjs';

test('host attestation requires exact physical, dedicated-seat, and safety evidence', () => {
  assert.deepEqual(validateHostAttestation(attestation), attestation);
  assert.throws(
    () => validateHostAttestation({ ...attestation, uinputPresent: true }),
    /host\/seat attestation is incomplete/,
  );
  assert.throws(
    () => validateHostAttestation({ ...attestation, rawSerial: 'secret' }),
    /unknown field/,
  );
});

test('client actions are typed, bounded, deadline-bound, and shortcut-free', () => {
  const base = {
    protocolVersion: PROTOCOL_VERSION,
    sequenceId: '3f1982f2-924d-4d0d-8198-f5d78c6bde6a',
    deadlineUnixMs: Date.now() + 1_000,
  };
  assert.equal(validateClientEnvelope({
    ...base,
    action: { kind: 'pointerMove', x: 200, y: 300, durationMs: 120 },
  }).action.kind, 'pointerMove');
  assert.throws(
    () => validateClientEnvelope({
      ...base,
      action: { kind: 'keyStroke', key: 'L', modifiers: ['Control'], dwellMs: 60, flightMs: 60 },
    }),
    /shortcut modifier is forbidden/,
  );
  assert.throws(
    () => validateClientEnvelope({ ...base, action: { kind: 'shell', value: 'id' } }),
    /action kind is forbidden/,
  );
  assert.throws(
    () => validateClientEnvelope({ ...base, deadlineUnixMs: Date.now() - 1, action: { kind: 'releaseAll' } }),
    /deadline expired/,
  );
});

test('fresh hello must match every pinned identity and safety field', () => {
  const expected = {
    sessionId: 'f9309bf5-aa73-4237-a30e-f4a3b9d2f5e8',
    nonce: '53291d28-611e-4f25-bae0-cf38c30cbb57',
    attestation,
  };
  const ack = {
    protocolVersion: PROTOCOL_VERSION,
    kind: 'helloAck',
    sessionId: expected.sessionId,
    nonce: expected.nonce,
    identity: {
      vid: attestation.vid,
      pid: attestation.pid,
      serialSha256: attestation.serialSha256,
      descriptorSha256: attestation.descriptorSha256,
      topologySha256: attestation.topologySha256,
      firmwareSha256: attestation.firmwareSha256,
      interfaceSet: attestation.interfaceSet,
    },
    safety,
  };
  assert.equal(validateHelloAck(ack, expected).safety.deadManArmed, true);
  assert.throws(
    () => validateHelloAck({
      ...ack,
      identity: { ...ack.identity, firmwareSha256: '5'.repeat(64) },
    }, expected),
    /firmware identity mismatch/,
  );
  assert.throws(
    () => validateHelloAck({
      ...ack,
      safety: { ...safety, emergencyStopEngaged: true },
    }, expected),
    /not ready/,
  );
});

test('command acknowledgement is bound to session, sequence, command, and release state', () => {
  const expected = {
    sessionId: 'f9309bf5-aa73-4237-a30e-f4a3b9d2f5e8',
    sequence: 3,
    commandId: '3f1982f2-924d-4d0d-8198-f5d78c6bde6a',
    action: { kind: 'releaseAll' },
    attestation,
  };
  const ack = {
    protocolVersion: PROTOCOL_VERSION,
    kind: 'ack',
    sessionId: expected.sessionId,
    sequence: 3,
    commandId: expected.commandId,
    accepted: true,
    releasedAll: true,
    safety,
  };
  assert.equal(validateCommandAck(ack, expected).accepted, true);
  assert.throws(() => validateCommandAck({ ...ack, sequence: 2 }, expected), /replay/);
  assert.throws(() => validateCommandAck({ ...ack, releasedAll: false }, expected), /did not confirm/);
});
