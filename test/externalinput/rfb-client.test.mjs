import assert from 'node:assert/strict';
import net from 'node:net';
import test from 'node:test';
import { connectRfb } from './rfb-client.mjs';

function serverInit(width, height) {
  const name = Buffer.from('contract-rfb');
  const init = Buffer.alloc(24);
  init.writeUInt16BE(width, 0);
  init.writeUInt16BE(height, 2);
  init.writeUInt32BE(name.length, 20);
  return Buffer.concat([init, name]);
}

function rawUpdate(width, height, pixels) {
  const header = Buffer.alloc(4);
  header[0] = 0;
  header.writeUInt16BE(1, 2);
  const rect = Buffer.alloc(12);
  rect.writeUInt16BE(width, 4);
  rect.writeUInt16BE(height, 6);
  rect.writeInt32BE(0, 8);
  return Buffer.concat([header, rect, pixels]);
}

async function fakeRfbServer(width = 2, height = 2) {
  const messages = [];
  const pixels = Buffer.from([
    0, 0, 255, 0, 0, 255, 0, 0,
    255, 0, 0, 0, 255, 255, 255, 0,
  ]);
  const server = net.createServer((socket) => {
    let state = 'version';
    let buffer = Buffer.alloc(0);
    socket.write('RFB 003.008\n');
    socket.on('data', (chunk) => {
      buffer = Buffer.concat([buffer, chunk]);
      for (;;) {
        if (state === 'version' && buffer.length >= 12) {
          messages.push(buffer.subarray(0, 12));
          buffer = buffer.subarray(12);
          socket.write(Buffer.from([1, 1]));
          state = 'security';
        } else if (state === 'security' && buffer.length >= 1) {
          assert.equal(buffer[0], 1);
          buffer = buffer.subarray(1);
          socket.write(Buffer.alloc(4));
          state = 'client-init';
        } else if (state === 'client-init' && buffer.length >= 1) {
          assert.equal(buffer[0], 0);
          buffer = buffer.subarray(1);
          socket.write(serverInit(width, height));
          state = 'messages';
        } else if (state === 'messages' && buffer.length) {
          const type = buffer[0];
          const lengths = { 0: 20, 2: 8, 3: 10, 4: 8, 5: 6 };
          const length = lengths[type];
          if (!length || buffer.length < length) break;
          const message = buffer.subarray(0, length);
          buffer = buffer.subarray(length);
          messages.push(Buffer.from(message));
          if (type === 3) socket.write(rawUpdate(width, height, pixels));
        } else {
          break;
        }
      }
    });
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  return {
    server,
    messages,
    pixels,
    port: server.address().port,
    waitForInput: () => new Promise((resolve, reject) => {
      const deadline = Date.now() + 1_000;
      const check = () => {
        if (messages.some((message) => message[0] === 5) &&
            messages.some((message) => message[0] === 4)) {
          resolve();
        } else if (Date.now() >= deadline) {
          reject(new Error('fake RFB server did not receive input messages'));
        } else {
          setTimeout(check, 5);
        }
      };
      check();
    }),
    close: () => new Promise((resolve) => server.close(resolve)),
  };
}

test('RFB 3.8 client performs a real handshake, captures Raw pixels, and emits external input', async () => {
  const fake = await fakeRfbServer();
  const client = await connectRfb({
    host: '127.0.0.1',
    port: fake.port,
    allowInput: true,
  });
  try {
    assert.equal(client.protocol, 'RFB 3.8');
    assert.equal(client.width, 2);
    assert.equal(client.height, 2);
    const frame = await client.capture();
    assert.deepEqual(frame.pixels, fake.pixels);
    await client.sendAction({ kind: 'pointerMove', x: 1, y: 1, durationMs: 1 });
    await client.sendAction({ kind: 'pointerClick', button: 'left' });
    await client.sendAction({ kind: 'keyStroke', key: 'Tab', modifiers: [] });
    await client.releaseAll();
    await fake.waitForInput();
    assert.equal(fake.messages.some((message) => message[0] === 5), true);
    assert.equal(fake.messages.some((message) => message[0] === 4), true);
  } finally {
    client.close();
    await fake.close();
  }
});

test('RFB client in USB mode rejects every virtual input attempt', async () => {
  const fake = await fakeRfbServer();
  const client = await connectRfb({
    host: '127.0.0.1',
    port: fake.port,
    allowInput: false,
  });
  try {
    await assert.rejects(
      () => client.sendAction({ kind: 'pointerMove', x: 1, y: 1, durationMs: 1 }),
      /disabled for USB mode/,
    );
  } finally {
    client.close();
    await fake.close();
  }
});

test('RFB pointer movement honors duration with a bounded path instead of teleporting', async () => {
  const fake = await fakeRfbServer(20, 20);
  const client = await connectRfb({
    host: '127.0.0.1',
    port: fake.port,
    allowInput: true,
  });
  try {
    const before = fake.messages.filter((message) => message[0] === 5).length;
    await client.sendAction({ kind: 'pointerMove', x: 19, y: 17, durationMs: 64 });
    await client.sendAction({ kind: 'pointerClick', button: 'left', dwellMs: 20 });
    await new Promise((resolve) => setTimeout(resolve, 20));
    const pointer = fake.messages
      .filter((message) => message[0] === 5)
      .slice(before);
    assert.equal(pointer.length >= 6, true, 'move path plus press/release must be observable');
    const motion = pointer.slice(0, -2);
    assert.equal(motion.length, 4);
    assert.deepEqual(
      [motion.at(-1).readUInt16BE(2), motion.at(-1).readUInt16BE(4)],
      [19, 17],
    );
    assert.equal(pointer.at(-2)[1], 1, 'button press must be distinct');
    assert.equal(pointer.at(-1)[1], 0, 'button release must be distinct');
  } finally {
    client.close();
    await fake.close();
  }
});
