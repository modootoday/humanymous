import { createHash } from 'node:crypto';
import { setTimeout as sleepDefault } from 'node:timers/promises';
import { validateAction } from '../usb-broker/protocol.mjs';

const KEY_CODES = Object.freeze({
  Enter: 0x28, Escape: 0x29, Backspace: 0x2a, Tab: 0x2b, Space: 0x2c,
  Delete: 0x4c, ArrowRight: 0x4f, ArrowLeft: 0x50,
  ArrowDown: 0x51, ArrowUp: 0x52,
  ...Object.fromEntries('ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('').map((key, index) => [key, 0x04 + index])),
  ...Object.fromEntries('1234567890'.split('').map((key, index) => [key, 0x1e + index])),
});
const TEXT = 'humanymous synthetic task';

function signedByte(value) {
  const bounded = Math.max(-127, Math.min(127, value));
  return bounded < 0 ? 256 + bounded : bounded;
}

export class HidReportGateway {
  #keyboard;
  #pointer;
  #limits;
  #sleep;
  #now;
  #reports = [];
  #cursor;
  #watchdog = null;
  #closed = false;
  #actionPolicy;
  #actionHash = createHash('sha256');
  #reportHash = createHash('sha256');
  #actionCount = 0;
  #reportCount = 0;

  constructor({
    writeKeyboard,
    writePointer,
    limits,
    sleep = sleepDefault,
    now = () => Date.now(),
    initialPointer = { x: 640, y: 360 },
    actionPolicy = null,
  }) {
    if (typeof writeKeyboard !== 'function' || typeof writePointer !== 'function') {
      throw new TypeError('gateway requires fixed keyboard and pointer writers');
    }
    this.#keyboard = writeKeyboard;
    this.#pointer = writePointer;
    this.#limits = limits;
    this.#sleep = sleep;
    this.#now = now;
    this.#cursor = { ...initialPointer };
    if (actionPolicy && typeof actionPolicy.validate !== 'function') {
      throw new TypeError('gateway action policy must expose validate');
    }
    this.#actionPolicy = actionPolicy;
    this.#armWatchdog();
  }

  async perform(input) {
    if (this.#closed) throw new Error('gateway is closed');
    const action = validateAction(input);
    this.#actionPolicy?.validate(action);
    this.#armWatchdog();
    switch (action.kind) {
      case 'pointerMove':
        await this.#move(action.x, action.y, action.durationMs);
        break;
      case 'pointerClick':
        await this.#pointerReport(1, 0, 0, 0);
        await this.#sleep(action.dwellMs);
        await this.#pointerReport(0, 0, 0, 0);
        break;
      case 'scroll': {
        let remaining = action.dy;
        while (remaining !== 0) {
          const wheel = Math.sign(remaining) * Math.min(127, Math.abs(remaining));
          await this.#pointerReport(0, 0, 0, wheel);
          remaining -= wheel;
        }
        await this.#pointerReport(0, 0, 0, 0);
        break;
      }
      case 'keyStroke':
        await this.#stroke(action.key, action.modifiers, action.dwellMs, action.flightMs);
        break;
      case 'typeText':
        if (action.text !== TEXT) throw new TypeError('text is not the fixed synthetic task');
        for (const char of TEXT) {
          const upper = char.toUpperCase();
          await this.#stroke(upper === ' ' ? 'Space' : upper, [], 40, 40);
        }
        break;
      case 'releaseAll':
        await this.releaseAll();
        break;
      default:
        throw new TypeError('action kind is not implemented');
    }
    this.#actionHash.update(`${JSON.stringify(action)}\n`);
    this.#actionCount += 1;
  }

  evidence() {
    return Object.freeze({
      actionCount: this.#actionCount,
      actionSha256: this.#actionHash.copy().digest('hex'),
      reportCount: this.#reportCount,
      reportSha256: this.#reportHash.copy().digest('hex'),
      ...(this.#actionPolicy ? { policy: this.#actionPolicy.snapshot() } : {}),
    });
  }

  async releaseAll() {
    await this.#keyboardReport(0, 0);
    await this.#pointerReport(0, 0, 0, 0);
  }

  async close() {
    if (this.#closed) return;
    this.#closed = true;
    clearTimeout(this.#watchdog);
    await this.releaseAll();
  }

  async #stroke(key, modifiers = [], dwellMs = 60, flightMs = 60) {
    const code = KEY_CODES[key];
    if (!code) throw new TypeError(`key has no boot-keyboard mapping: ${key}`);
    const modifier = modifiers?.includes('Shift') ? 0x02 : 0;
    await this.#keyboardReport(modifier, code);
    await this.#sleep(dwellMs);
    await this.#keyboardReport(0, 0);
    await this.#sleep(flightMs);
  }

  async #move(targetX, targetY, durationMs) {
    let dx = targetX - this.#cursor.x;
    let dy = targetY - this.#cursor.y;
    const step = this.#limits.maxRelativeStep;
    const count = Math.max(1, Math.ceil(Math.max(Math.abs(dx), Math.abs(dy)) / step));
    const delay = count > 0 ? Math.floor(durationMs / count) : 0;
    for (let remaining = count; remaining > 0; remaining -= 1) {
      const nextX = Math.max(-step, Math.min(step, Math.round(dx / remaining)));
      const nextY = Math.max(-step, Math.min(step, Math.round(dy / remaining)));
      await this.#pointerReport(0, nextX, nextY, 0);
      dx -= nextX;
      dy -= nextY;
      this.#cursor.x += nextX;
      this.#cursor.y += nextY;
      if (delay > 0) await this.#sleep(delay);
    }
    await this.#pointerReport(0, 0, 0, 0);
  }

  async #throttle() {
    const now = this.#now();
    this.#reports = this.#reports.filter((time) => now - time < 1000);
    if (this.#reports.length >= this.#limits.maxReportsPerSecond) {
      await this.#sleep(Math.max(1, 1000 - (now - this.#reports[0])));
      this.#reports = this.#reports.slice(1);
    }
    this.#reports.push(this.#now());
  }

  async #keyboardReport(modifier, keyCode) {
    await this.#throttle();
    const report = Buffer.from([modifier, 0, keyCode, 0, 0, 0, 0, 0]);
    await this.#keyboard(report);
    this.#reportHash.update(report);
    this.#reportCount += 1;
  }

  async #pointerReport(buttons, dx, dy, wheel) {
    await this.#throttle();
    const report = Buffer.from([
      buttons, signedByte(dx), signedByte(dy), signedByte(wheel),
    ]);
    await this.#pointer(report);
    this.#reportHash.update(report);
    this.#reportCount += 1;
  }

  #armWatchdog() {
    clearTimeout(this.#watchdog);
    this.#watchdog = setTimeout(() => {
      void this.releaseAll().catch(() => {
        process.exitCode = 1;
      });
    }, this.#limits.deadManReleaseMs);
    this.#watchdog.unref?.();
  }
}

export function decodeSignedByte(value) {
  return value > 127 ? value - 256 : value;
}
