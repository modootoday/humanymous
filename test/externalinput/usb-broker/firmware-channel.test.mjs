import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import test from 'node:test';
import { FirmwareChannel } from './firmware-channel.mjs';
import { attestation, safety } from './test-fixtures.mjs';

class FakeTransport extends EventEmitter {
  writes = [];
  destroyed = false;

  write(value) {
    this.writes.push(JSON.parse(value));
    return true;
  }

  respond(value) {
    this.emit('data', Buffer.from(`${JSON.stringify(value)}\n`));
  }

  destroy() {
    this.destroyed = true;
  }
}

function identity() {
  return {
    vid: attestation.vid,
    pid: attestation.pid,
    serialSha256: attestation.serialSha256,
    descriptorSha256: attestation.descriptorSha256,
    topologySha256: attestation.topologySha256,
    firmwareSha256: attestation.firmwareSha256,
    interfaceSet: attestation.interfaceSet,
  };
}

function ackFor(command, changes = {}) {
  return {
    protocolVersion: '1.0.0',
    kind: 'ack',
    sessionId: command.sessionId,
    sequence: command.sequence,
    commandId: command.commandId,
    accepted: true,
    releasedAll: command.action.kind === 'releaseAll',
    safety,
    ...changes,
  };
}

test('startup handshake is nonce-bound and releases all HID state before readiness', async () => {
  const transport = new FakeTransport();
  const channel = new FirmwareChannel(transport, attestation);
  const pending = channel.handshake();
  await new Promise((resolve) => setImmediate(resolve));
  const hello = transport.writes[0];
  transport.respond({
    protocolVersion: '1.0.0',
    kind: 'helloAck',
    sessionId: hello.sessionId,
    nonce: hello.nonce,
    identity: identity(),
    safety,
  });
  await new Promise((resolve) => setImmediate(resolve));
  const release = transport.writes[1];
  assert.equal(release.action.kind, 'releaseAll');
  assert.equal(release.sequence, 1);
  transport.respond(ackFor(release));
  await pending;
  await channel.close({ release: false });
});

test('each command requires a matching sequence-bound acknowledgement', async () => {
  const transport = new FakeTransport();
  const channel = new FirmwareChannel(transport, attestation);
  const pending = channel.command(
    { kind: 'pointerClick', button: 'left' },
    {
      commandId: '3f1982f2-924d-4d0d-8198-f5d78c6bde6a',
      deadlineUnixMs: Date.now() + 1_000,
    },
  );
  await new Promise((resolve) => setImmediate(resolve));
  const command = transport.writes[0];
  transport.respond(ackFor(command));
  await pending;

  const replayed = channel.command(
    { kind: 'releaseAll' },
    {
      commandId: '541a0bb7-6214-4859-9cdb-e12b7044c788',
      deadlineUnixMs: Date.now() + 1_000,
    },
  );
  await new Promise((resolve) => setImmediate(resolve));
  transport.respond(ackFor(command));
  await assert.rejects(replayed, /sequence mismatch or replay/);
  await assert.rejects(() => channel.close({ release: true, timeoutMs: 10 }));
});

test('runtime dead-man or emergency readiness loss fails closed', async () => {
  const transport = new FakeTransport();
  const channel = new FirmwareChannel(transport, attestation);
  const pending = channel.command(
    { kind: 'releaseAll' },
    {
      commandId: '54dcd2bd-f921-4f58-93a3-a9eb7ef3f2c2',
      deadlineUnixMs: Date.now() + 1_000,
    },
  );
  await new Promise((resolve) => setImmediate(resolve));
  const command = transport.writes[0];
  transport.respond(ackFor(command, {
    safety: { ...safety, deadManArmed: false },
  }));
  await assert.rejects(pending, /not ready/);
});

test('unsolicited, oversized, and timed-out firmware responses fail closed', async () => {
  const unsolicitedTransport = new FakeTransport();
  const unsolicited = new FirmwareChannel(unsolicitedTransport, attestation);
  unsolicitedTransport.respond({ kind: 'ack' });
  await assert.rejects(
    () => unsolicited.command(
      { kind: 'releaseAll' },
      {
        commandId: '54dcd2bd-f921-4f58-93a3-a9eb7ef3f2c2',
        deadlineUnixMs: Date.now() + 10,
        timeoutMs: 10,
      },
    ),
    /closed/,
  );

  const timeoutTransport = new FakeTransport();
  const timed = new FirmwareChannel(timeoutTransport, attestation);
  await assert.rejects(
    () => timed.command(
      { kind: 'releaseAll' },
      {
        commandId: '54dcd2bd-f921-4f58-93a3-a9eb7ef3f2c2',
        deadlineUnixMs: Date.now() + 10,
        timeoutMs: 10,
      },
    ),
    /timed out/,
  );
});
