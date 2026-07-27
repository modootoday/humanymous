import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { unlink } from 'node:fs/promises';
import net from 'node:net';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { UsbBroker } from './broker.mjs';

function socketPath(name) {
  return process.platform === 'win32'
    ? `\\\\.\\pipe\\hmn-usb-${process.pid}-${name}`
    : join(tmpdir(), `hmn-usb-${process.pid}-${name}.sock`);
}

function request(path, envelope) {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection({ path });
    let output = '';
    socket.on('connect', () => socket.write(`${JSON.stringify(envelope)}\n`));
    socket.on('data', (chunk) => { output += chunk.toString('utf8'); });
    socket.on('end', () => resolve(JSON.parse(output)));
    socket.on('error', reject);
  });
}

function envelope(sequenceId = randomUUID(), action = { kind: 'releaseAll' }) {
  return {
    protocolVersion: '1.0.0',
    sequenceId,
    deadlineUnixMs: Date.now() + 1_000,
    action,
  };
}

class FakeFirmware {
  commands = [];
  closes = 0;

  async command(action, options) {
    this.commands.push({ action, options });
  }

  async close() {
    this.closes += 1;
  }
}

test('broker forwards only validated typed actions and preserves command identity/deadline', async () => {
  const path = socketPath('ok');
  const firmware = new FakeFirmware();
  const broker = new UsbBroker({ socketPath: path, firmware });
  await broker.start();
  try {
    const input = envelope(randomUUID(), { kind: 'pointerClick', button: 'left' });
    const response = await request(path, input);
    assert.equal(response.sequenceId, input.sequenceId);
    assert.equal(response.accepted, true);
    assert.deepEqual(firmware.commands[0].action, input.action);
    assert.equal(firmware.commands[0].options.commandId, input.sequenceId);
    assert.equal(firmware.commands[0].options.deadlineUnixMs, input.deadlineUnixMs);
  } finally {
    await broker.stop();
  }
  assert.equal(firmware.closes, 1);
  if (process.platform !== 'win32') await unlink(path).catch(() => {});
});

test('broker rejects replay, forbidden action, stale deadline, and multiple frames', async () => {
  const path = socketPath('reject');
  const firmware = new FakeFirmware();
  const broker = new UsbBroker({ socketPath: path, firmware });
  await broker.start();
  try {
    const id = randomUUID();
    assert.equal((await request(path, envelope(id))).accepted, true);
    assert.match((await request(path, envelope(id))).error, /replayed sequence ID/);
    assert.match(
      (await request(path, envelope(randomUUID(), { kind: 'shell', value: 'id' }))).error,
      /action kind is forbidden/,
    );
    assert.match(
      (await request(path, {
        ...envelope(),
        deadlineUnixMs: Date.now() - 1,
      })).error,
      /deadline expired/,
    );

    const multi = await new Promise((resolve, reject) => {
      const socket = net.createConnection({ path });
      let output = '';
      socket.on('connect', () => socket.write(
        `${JSON.stringify(envelope())}\n${JSON.stringify(envelope())}\n`,
      ));
      socket.on('data', (chunk) => { output += chunk.toString('utf8'); });
      socket.on('end', () => resolve(JSON.parse(output)));
      socket.on('error', reject);
    });
    assert.match(multi.error, /multiple requests/);
  } finally {
    await broker.stop();
  }
});

test('firmware failure is returned as a rejected command, never accepted', async () => {
  const path = socketPath('failure');
  const firmware = new FakeFirmware();
  firmware.command = async () => {
    throw new Error('firmware acknowledgement timed out');
  };
  const broker = new UsbBroker({ socketPath: path, firmware });
  await broker.start();
  try {
    const response = await request(path, envelope());
    assert.equal(response.accepted, false);
    assert.match(response.error, /timed out/);
  } finally {
    await broker.stop();
  }
});

test('episode policy rejects an otherwise valid text action before firmware dispatch', async () => {
  const path = socketPath('episode-policy');
  const firmware = new FakeFirmware();
  const actionPolicy = {
    validate(action) {
      if (action.kind === 'typeText') throw new TypeError('IME text actions are forbidden');
    },
  };
  const broker = new UsbBroker({ socketPath: path, firmware, actionPolicy });
  await broker.start();
  try {
    const response = await request(path, envelope(randomUUID(), {
      kind: 'typeText',
      text: 'humanymous synthetic task',
    }));
    assert.equal(response.accepted, false);
    assert.match(response.error, /IME text actions are forbidden/);
    assert.equal(firmware.commands.length, 0);
  } finally {
    await broker.stop();
  }
});
