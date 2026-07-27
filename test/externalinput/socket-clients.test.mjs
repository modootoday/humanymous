import assert from 'node:assert/strict';
import { unlink } from 'node:fs/promises';
import net from 'node:net';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { queryDomSocket } from './dom-client.mjs';
import { PROTOCOL_VERSION as DOM_PROTOCOL_VERSION } from './dom-observer/extension/protocol.mjs';
import { sendUsbBrokerCommand } from './usb-client.mjs';

function socketPath(name) {
  return process.platform === 'win32'
    ? `\\\\.\\pipe\\hmn-${process.pid}-${name}`
    : join(tmpdir(), `hmn-${process.pid}-${name}.sock`);
}

async function lineServer(path, respond) {
  if (process.platform !== 'win32') await unlink(path).catch(() => {});
  const server = net.createServer((socket) => {
    let input = '';
    socket.on('data', (chunk) => {
      input += chunk.toString('utf8');
      const newline = input.indexOf('\n');
      if (newline < 0) return;
      const request = JSON.parse(input.slice(0, newline));
      socket.end(`${JSON.stringify(respond(request))}\n`);
    });
  });
  await new Promise((resolve) => server.listen(path, resolve));
  return async () => {
    await new Promise((resolve) => server.close(resolve));
    if (process.platform !== 'win32') await unlink(path).catch(() => {});
  };
}

test('DOM socket protocol binds response to request sequence and deadline', async () => {
  const path = socketPath('dom');
  let envelope;
  const close = await lineServer(path, (request) => {
    envelope = request;
    return {
      sequenceId: request.sequenceId,
      result: {
        token: 'choice-correct',
        rect: { x: 1, y: 2, width: 3, height: 4 },
      },
    };
  });
  try {
    const result = await queryDomSocket(path, { method: 'findByTextToken', token: 'choice-correct' });
    assert.equal(envelope.protocolVersion, DOM_PROTOCOL_VERSION);
    assert.equal(envelope.deadlineUnixMs > Date.now(), true);
    assert.equal(result.token, 'choice-correct');
  } finally {
    await close();
  }
});

test('USB broker command requires an acknowledgement with the same sequence', async () => {
  const path = socketPath('usb');
  let envelope;
  const close = await lineServer(path, (request) => {
    envelope = request;
    return { sequenceId: request.sequenceId, accepted: true };
  });
  try {
    await sendUsbBrokerCommand(path, { kind: 'releaseAll' });
    assert.equal(envelope.protocolVersion, '1.0.0');
    assert.deepEqual(envelope.action, { kind: 'releaseAll' });
  } finally {
    await close();
  }
});

test('USB broker rejects a stale or replayed acknowledgement', async () => {
  const path = socketPath('usb-replay');
  const close = await lineServer(path, () => ({ sequenceId: 'stale', accepted: true }));
  try {
    await assert.rejects(
      () => sendUsbBrokerCommand(path, { kind: 'releaseAll' }),
      /sequence mismatch/,
    );
  } finally {
    await close();
  }
});
