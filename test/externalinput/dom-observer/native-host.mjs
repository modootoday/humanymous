#!/usr/bin/env node

import { chmod, unlink } from 'node:fs/promises';
import net from 'node:net';
import {
  MAX_REQUEST_BYTES,
  MAX_RESPONSE_BYTES,
} from './extension/protocol.mjs';
import { NativeQueryBroker } from './broker.mjs';

const FIXED_SOCKET_PATH = '/run/dom-observer/observer.sock';
const socketPath = process.env.HM_EXTERNAL_DOM_SOCKET || FIXED_SOCKET_PATH;
let nativeInput = Buffer.alloc(0);

if (socketPath !== FIXED_SOCKET_PATH) {
  throw new Error(`HM_EXTERNAL_DOM_SOCKET must be exactly ${FIXED_SOCKET_PATH}`);
}

function nativeWrite(value) {
  const payload = Buffer.from(JSON.stringify(value), 'utf8');
  if (payload.length > MAX_RESPONSE_BYTES) throw new Error('native message exceeds response bound');
  const header = Buffer.alloc(4);
  header.writeUInt32LE(payload.length);
  process.stdout.write(Buffer.concat([header, payload]));
}

function socketReply(socket, value) {
  const payload = `${JSON.stringify(value)}\n`;
  if (Buffer.byteLength(payload) > MAX_RESPONSE_BYTES) {
    socket.end(`${JSON.stringify({ sequenceId: value.sequenceId || '', error: 'response exceeds bound' })}\n`);
    return;
  }
  socket.end(payload);
}

const broker = new NativeQueryBroker({
  writeNative: nativeWrite,
  reply: socketReply,
});

function consumeNativeInput() {
  for (;;) {
    if (nativeInput.length < 4) return;
    const length = nativeInput.readUInt32LE(0);
    if (length === 0 || length > MAX_RESPONSE_BYTES) {
      throw new Error('invalid native message length');
    }
    if (nativeInput.length < 4 + length) return;
    const payload = nativeInput.subarray(4, 4 + length);
    nativeInput = nativeInput.subarray(4 + length);
    broker.handleNativeResponse(JSON.parse(payload.toString('utf8')));
  }
}

process.stdin.on('data', (chunk) => {
  nativeInput = Buffer.concat([nativeInput, chunk]);
  if (nativeInput.length > MAX_RESPONSE_BYTES + 4) {
    throw new Error('native input buffer exceeds bound');
  }
  consumeNativeInput();
});
process.stdin.on('end', () => process.exit(0));

await unlink(socketPath).catch((error) => {
  if (error.code !== 'ENOENT') throw error;
});

const server = net.createServer((socket) => {
  let bytes = 0;
  let input = '';
  let handled = false;
  socket.on('data', (chunk) => {
    if (handled) return;
    bytes += chunk.length;
    if (bytes > MAX_REQUEST_BYTES) {
      handled = true;
      socketReply(socket, { sequenceId: '', error: 'request exceeds bound' });
      return;
    }
    input += chunk.toString('utf8');
    const newline = input.indexOf('\n');
    if (newline < 0) return;
    handled = true;
    socket.pause();
    let sequenceId = '';
    try {
      if (input.slice(newline + 1).trim()) throw new TypeError('one request per connection');
      const parsed = JSON.parse(input.slice(0, newline));
      if (typeof parsed?.sequenceId === 'string' && parsed.sequenceId.length <= 96) {
        sequenceId = parsed.sequenceId;
      }
      broker.accept(parsed, socket);
    } catch (error) {
      socketReply(socket, {
        sequenceId,
        error: String(error?.message || error).slice(0, 256),
      });
    }
  });
  socket.on('error', () => {
    broker.cancelClient(socket);
  });
  socket.on('close', () => broker.cancelClient(socket));
});

await new Promise((resolve, reject) => {
  server.once('error', reject);
  server.listen(socketPath, resolve);
});
await chmod(socketPath, 0o660);
