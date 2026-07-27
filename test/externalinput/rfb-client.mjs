import { createCipheriv } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import net from 'node:net';
import { CapabilityUnavailableError, ContractViolationError } from './errors.mjs';

const RFB_VERSION = Buffer.from('RFB 003.008\n', 'ascii');

class ByteReader {
  #socket;
  #buffer = Buffer.alloc(0);
  #waiters = [];
  #error = null;

  constructor(socket) {
    this.#socket = socket;
    socket.on('data', (chunk) => {
      this.#buffer = Buffer.concat([this.#buffer, chunk]);
      this.#flush();
    });
    socket.on('error', (error) => {
      this.#error = error;
      this.#flush();
    });
    socket.on('close', () => {
      this.#error ||= new Error('RFB connection closed');
      this.#flush();
    });
  }

  #flush() {
    while (this.#waiters.length) {
      const waiter = this.#waiters[0];
      if (this.#buffer.length >= waiter.length) {
        this.#waiters.shift();
        const value = this.#buffer.subarray(0, waiter.length);
        this.#buffer = this.#buffer.subarray(waiter.length);
        waiter.resolve(value);
      } else if (this.#error) {
        this.#waiters.shift();
        waiter.reject(this.#error);
      } else {
        break;
      }
    }
  }

  read(length, timeoutMs = 10_000) {
    if (!Number.isInteger(length) || length < 0) return Promise.reject(new TypeError('invalid read'));
    if (this.#buffer.length >= length) {
      const value = this.#buffer.subarray(0, length);
      this.#buffer = this.#buffer.subarray(length);
      return Promise.resolve(value);
    }
    if (this.#error) return Promise.reject(this.#error);
    return new Promise((resolve, reject) => {
      const waiter = { length, resolve, reject };
      this.#waiters.push(waiter);
      const timer = setTimeout(() => {
        const index = this.#waiters.indexOf(waiter);
        if (index >= 0) this.#waiters.splice(index, 1);
        reject(new Error(`RFB read timed out after ${timeoutMs}ms`));
      }, timeoutMs);
      timer.unref?.();
      waiter.resolve = (value) => {
        clearTimeout(timer);
        resolve(value);
      };
      waiter.reject = (error) => {
        clearTimeout(timer);
        reject(error);
      };
    });
  }
}

function reverseBits(byte) {
  let value = byte;
  value = ((value & 0xf0) >>> 4) | ((value & 0x0f) << 4);
  value = ((value & 0xcc) >>> 2) | ((value & 0x33) << 2);
  return ((value & 0xaa) >>> 1) | ((value & 0x55) << 1);
}

function vncResponse(challenge, password) {
  const key8 = Buffer.alloc(8);
  Buffer.from(password, 'utf8').subarray(0, 8).copy(key8);
  for (let index = 0; index < key8.length; index += 1) key8[index] = reverseBits(key8[index]);
  // OpenSSL 3 commonly disables single DES. EDE3 with K1=K2=K3 is exactly
  // single-DES encryption and remains interoperable with the RFB VNC challenge.
  const key24 = Buffer.concat([key8, key8, key8]);
  const cipher = createCipheriv('des-ede3', key24, null);
  cipher.setAutoPadding(false);
  return Buffer.concat([cipher.update(challenge), cipher.final()]);
}

async function connectSocket({ host, port, timeoutMs }) {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection({ host, port });
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new Error(`RFB connect timed out after ${timeoutMs}ms`));
    }, timeoutMs);
    socket.once('connect', () => {
      clearTimeout(timer);
      socket.setNoDelay(true);
      resolve(socket);
    });
    socket.once('error', (error) => {
      clearTimeout(timer);
      reject(error);
    });
  });
}

function write(socket, buffer) {
  return new Promise((resolve, reject) => {
    socket.write(buffer, (error) => error ? reject(error) : resolve());
  });
}

function parseVersion(value) {
  const text = value.toString('ascii');
  if (!/^RFB 003\.00[378]\n$/.test(text)) {
    throw new CapabilityUnavailableError('rfb-3.8', `unsupported RFB version: ${JSON.stringify(text)}`);
  }
}

function pixelFormatMessage() {
  const message = Buffer.alloc(20);
  message[0] = 0;
  message[4] = 32;
  message[5] = 24;
  message[6] = 0; // little endian
  message[7] = 1; // true colour
  message.writeUInt16BE(255, 8);
  message.writeUInt16BE(255, 10);
  message.writeUInt16BE(255, 12);
  message[14] = 16;
  message[15] = 8;
  message[16] = 0;
  return message;
}

function encodingsMessage() {
  const message = Buffer.alloc(8);
  message[0] = 2;
  message.writeUInt16BE(1, 2);
  message.writeInt32BE(0, 4); // Raw only: deterministic, bounded, no decoder ambiguity.
  return message;
}

function framebufferRequest(width, height) {
  const message = Buffer.alloc(10);
  message[0] = 3;
  message[1] = 0;
  message.writeUInt16BE(width, 6);
  message.writeUInt16BE(height, 8);
  return message;
}

const KEY_SYMS = Object.freeze({
  Backspace: 0xff08,
  Tab: 0xff09,
  Enter: 0xff0d,
  Escape: 0xff1b,
  Space: 0x20,
  Delete: 0xffff,
  ArrowLeft: 0xff51,
  ArrowUp: 0xff52,
  ArrowRight: 0xff53,
  ArrowDown: 0xff54,
  Shift: 0xffe1,
});

function keysym(key) {
  if (KEY_SYMS[key]) return KEY_SYMS[key];
  if (/^[\x20-\x7e]$/.test(key)) return key.charCodeAt(0);
  throw new ContractViolationError(`RFB key is not mapped: ${key}`);
}

function keyEvent(key, down) {
  const message = Buffer.alloc(8);
  message[0] = 4;
  message[1] = down ? 1 : 0;
  message.writeUInt32BE(keysym(key), 4);
  return message;
}

function pointerEvent(mask, x, y) {
  const message = Buffer.alloc(6);
  message[0] = 5;
  message[1] = mask;
  message.writeUInt16BE(x, 2);
  message.writeUInt16BE(y, 4);
  return message;
}

function delay(durationMs) {
  return durationMs > 0
    ? new Promise((resolve) => setTimeout(resolve, durationMs))
    : Promise.resolve();
}

export async function connectRfb({
  host,
  port = 5900,
  passwordFile,
  allowInput,
  timeoutMs = 10_000,
}) {
  const socket = await connectSocket({ host, port, timeoutMs });
  const reader = new ByteReader(socket);
  const pressedKeys = new Set();
  let pointerMask = 0;
  let pointerX = 0;
  let pointerY = 0;

  try {
    parseVersion(await reader.read(12, timeoutMs));
    await write(socket, RFB_VERSION);
    const count = (await reader.read(1, timeoutMs))[0];
    if (count === 0) {
      const length = (await reader.read(4, timeoutMs)).readUInt32BE();
      const reason = (await reader.read(length, timeoutMs)).toString('utf8');
      throw new CapabilityUnavailableError('rfb-security', reason);
    }
    const offered = [...await reader.read(count, timeoutMs)];
    let securityType;
    if (passwordFile && offered.includes(2)) securityType = 2;
    else if (offered.includes(1)) securityType = 1;
    else throw new CapabilityUnavailableError('rfb-security', 'RFB offers no supported security type');
    await write(socket, Buffer.from([securityType]));

    if (securityType === 2) {
      const challenge = await reader.read(16, timeoutMs);
      const password = (await readFile(passwordFile, 'utf8')).replace(/\r?\n$/, '');
      await write(socket, vncResponse(challenge, password));
    }
    const securityResult = (await reader.read(4, timeoutMs)).readUInt32BE();
    if (securityResult !== 0) {
      let reason = `RFB authentication failed (${securityResult})`;
      try {
        const length = (await reader.read(4, 500)).readUInt32BE();
        reason = (await reader.read(length, 500)).toString('utf8');
      } catch {}
      throw new CapabilityUnavailableError('rfb-auth', reason);
    }

    await write(socket, Buffer.from([0])); // exclusive client; second client is disconnected.
    const init = await reader.read(24, timeoutMs);
    const width = init.readUInt16BE(0);
    const height = init.readUInt16BE(2);
    const nameLength = init.readUInt32BE(20);
    const name = (await reader.read(nameLength, timeoutMs)).toString('utf8');
    if (!width || !height || width > 4096 || height > 2160) {
      throw new ContractViolationError('RFB framebuffer geometry is outside bounds');
    }
    await write(socket, pixelFormatMessage());
    await write(socket, encodingsMessage());

    async function capture() {
      await write(socket, framebufferRequest(width, height));
      for (;;) {
        const type = (await reader.read(1, timeoutMs))[0];
        if (type === 2) continue; // Bell
        if (type === 3) {
          const header = await reader.read(7, timeoutMs);
          const length = header.readUInt32BE(3);
          await reader.read(length, timeoutMs);
          throw new ContractViolationError('RFB ServerCutText/clipboard is forbidden');
        }
        if (type !== 0) throw new ContractViolationError(`unsupported RFB server message: ${type}`);
        const update = await reader.read(3, timeoutMs);
        const rectangles = update.readUInt16BE(1);
        const fullFrame = Buffer.alloc(width * height * 4);
        for (let index = 0; index < rectangles; index += 1) {
          const rect = await reader.read(12, timeoutMs);
          const x = rect.readUInt16BE(0);
          const y = rect.readUInt16BE(2);
          const rectWidth = rect.readUInt16BE(4);
          const rectHeight = rect.readUInt16BE(6);
          const encoding = rect.readInt32BE(8);
          if (encoding !== 0) throw new ContractViolationError(`non-Raw RFB encoding received: ${encoding}`);
          if (x + rectWidth > width || y + rectHeight > height) {
            throw new ContractViolationError('RFB rectangle exceeds framebuffer');
          }
          const pixels = await reader.read(rectWidth * rectHeight * 4, timeoutMs);
          for (let row = 0; row < rectHeight; row += 1) {
            pixels.copy(
              fullFrame,
              ((y + row) * width + x) * 4,
              row * rectWidth * 4,
              (row + 1) * rectWidth * 4,
            );
          }
        }
        return { width, height, pixels: fullFrame, cues: [] };
      }
    }

    async function sendAction(action) {
      if (!allowInput) throw new ContractViolationError('RFB input is disabled for USB mode');
      if (action.kind === 'pointerMove') {
        const startX = pointerX;
        const startY = pointerY;
        const durationMs = Math.max(0, action.durationMs || 0);
        const steps = Math.max(1, Math.min(120, Math.ceil(durationMs / 16)));
        const stepDelayMs = durationMs / steps;
        for (let step = 1; step <= steps; step += 1) {
          const progress = step / steps;
          const eased = 0.5 - Math.cos(Math.PI * progress) / 2;
          pointerX = Math.round(startX + (action.x - startX) * eased);
          pointerY = Math.round(startY + (action.y - startY) * eased);
          await write(socket, pointerEvent(pointerMask, pointerX, pointerY));
          if (step < steps) await delay(stepDelayMs);
        }
      } else if (action.kind === 'pointerClick') {
        const bit = { left: 1, middle: 2, right: 4 }[action.button];
        pointerMask |= bit;
        await write(socket, pointerEvent(pointerMask, pointerX, pointerY));
        await delay(action.dwellMs || 60);
        pointerMask &= ~bit;
        await write(socket, pointerEvent(pointerMask, pointerX, pointerY));
      } else if (action.kind === 'scroll') {
        const bit = action.dy < 0 ? 8 : 16;
        const ticks = Math.max(1, Math.min(10, Math.ceil(Math.abs(action.dy) / 120)));
        for (let tick = 0; tick < ticks; tick += 1) {
          await write(socket, pointerEvent(pointerMask | bit, pointerX, pointerY));
          await write(socket, pointerEvent(pointerMask, pointerX, pointerY));
          if (tick + 1 < ticks) await delay(18);
        }
      } else if (action.kind === 'keyStroke') {
        if (action.modifiers?.includes('Shift')) await write(socket, keyEvent('Shift', true));
        pressedKeys.add(action.key);
        await write(socket, keyEvent(action.key, true));
        await delay(action.dwellMs || 0);
        await write(socket, keyEvent(action.key, false));
        pressedKeys.delete(action.key);
        if (action.modifiers?.includes('Shift')) await write(socket, keyEvent('Shift', false));
        await delay(action.flightMs || 0);
      } else if (action.kind === 'typeText') {
        for (const char of action.text) {
          const key = char === ' ' ? 'Space' : char;
          pressedKeys.add(key);
          await write(socket, keyEvent(key, true));
          await write(socket, keyEvent(key, false));
          pressedKeys.delete(key);
        }
      } else if (action.kind === 'releaseAll') {
        await releaseAll();
      } else {
        throw new ContractViolationError(`RFB cannot send action: ${action.kind}`);
      }
    }

    async function releaseAll() {
      if (socket.destroyed || !allowInput) return;
      for (const key of pressedKeys) await write(socket, keyEvent(key, false));
      pressedKeys.clear();
      pointerMask = 0;
      await write(socket, pointerEvent(0, pointerX, pointerY));
    }

    return Object.freeze({
      protocol: 'RFB 3.8',
      width,
      height,
      name,
      allowInput,
      capture,
      sendAction,
      releaseAll,
      close: () => socket.destroy(),
    });
  } catch (error) {
    socket.destroy();
    throw error;
  }
}
