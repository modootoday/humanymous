import { randomUUID } from 'node:crypto';
import {
  encodeFrame,
  MAX_FRAME_BYTES,
  parseFrame,
  PROTOCOL_VERSION,
  validateCommandAck,
  validateHelloAck,
} from './protocol.mjs';

const HANDSHAKE_TIMEOUT_MS = 2_000;
const COMMAND_TIMEOUT_MS = 2_000;

export class FirmwareChannel {
  #transport;
  #attestation;
  #sessionId;
  #buffer = '';
  #waiter = null;
  #sequence = 0;
  #closed = false;
  #onData;
  #onError;

  constructor(transport, attestation, { sessionId = randomUUID() } = {}) {
    if (!transport || typeof transport.write !== 'function' || typeof transport.on !== 'function') {
      throw new TypeError('firmware transport must be a writable event emitter');
    }
    this.#transport = transport;
    this.#attestation = attestation;
    this.#sessionId = sessionId;
    this.#onData = (chunk) => this.#receive(chunk);
    this.#onError = (error) => this.#fail(error);
    transport.on('data', this.#onData);
    transport.on('error', this.#onError);
    transport.on('close', this.#onError);
  }

  get sessionId() {
    return this.#sessionId;
  }

  async handshake({ timeoutMs = HANDSHAKE_TIMEOUT_MS } = {}) {
    const nonce = randomUUID();
    const deadlineUnixMs = Date.now() + timeoutMs;
    const pending = this.#nextFrame(timeoutMs, 'firmware handshake timed out');
    try {
      this.#write({
        protocolVersion: PROTOCOL_VERSION,
        kind: 'hello',
        sessionId: this.#sessionId,
        nonce,
        deadlineUnixMs,
      });
    } catch (error) {
      this.#fail(error);
    }
    const frame = await pending;
    validateHelloAck(frame, {
      sessionId: this.#sessionId,
      nonce,
      attestation: this.#attestation,
    });
    await this.command({ kind: 'releaseAll' }, {
      commandId: randomUUID(),
      timeoutMs,
    });
  }

  async command(action, {
    commandId,
    deadlineUnixMs = Date.now() + COMMAND_TIMEOUT_MS,
    timeoutMs = Math.max(1, Math.min(COMMAND_TIMEOUT_MS, deadlineUnixMs - Date.now())),
  } = {}) {
    if (this.#closed) throw new Error('firmware channel is closed');
    if (this.#waiter) throw new Error('firmware command is already in flight');
    if (!Number.isInteger(deadlineUnixMs) || deadlineUnixMs <= Date.now()) {
      throw new Error('firmware command deadline expired');
    }
    const sequence = ++this.#sequence;
    const id = commandId || randomUUID();
    const remainingMs = deadlineUnixMs - Date.now();
    const pending = this.#nextFrame(
      Math.max(1, Math.min(timeoutMs, remainingMs)),
      'firmware acknowledgement timed out',
    );
    try {
      this.#write({
        protocolVersion: PROTOCOL_VERSION,
        kind: 'command',
        sessionId: this.#sessionId,
        sequence,
        commandId: id,
        deadlineUnixMs,
        action,
      });
    } catch (error) {
      this.#fail(error);
    }
    const frame = await pending;
    validateCommandAck(frame, {
      sessionId: this.#sessionId,
      sequence,
      commandId: id,
      action,
      attestation: this.#attestation,
    });
  }

  async close({ release = true, timeoutMs = COMMAND_TIMEOUT_MS } = {}) {
    if (this.#closed) return;
    let releaseError;
    if (release) {
      try {
        await this.command(
          { kind: 'releaseAll' },
          { commandId: randomUUID(), deadlineUnixMs: Date.now() + timeoutMs, timeoutMs },
        );
      } catch (error) {
        releaseError = error;
      }
    }
    this.#closed = true;
    this.#transport.off?.('data', this.#onData);
    this.#transport.off?.('error', this.#onError);
    this.#transport.off?.('close', this.#onError);
    this.#transport.destroy?.();
    if (releaseError) throw releaseError;
  }

  #write(value) {
    const encoded = encodeFrame(value);
    const accepted = this.#transport.write(encoded);
    if (accepted === false) this.#fail(new Error('firmware transport backpressure rejected command'));
  }

  #nextFrame(timeoutMs, timeoutMessage) {
    if (this.#waiter) throw new Error('firmware response is already pending');
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#waiter = null;
        reject(new Error(timeoutMessage));
      }, timeoutMs);
      this.#waiter = {
        resolve: (value) => {
          clearTimeout(timer);
          this.#waiter = null;
          resolve(value);
        },
        reject: (error) => {
          clearTimeout(timer);
          this.#waiter = null;
          reject(error);
        },
      };
    });
  }

  #receive(chunk) {
    if (this.#closed) return;
    this.#buffer += Buffer.isBuffer(chunk) ? chunk.toString('utf8') : String(chunk);
    if (Buffer.byteLength(this.#buffer) > MAX_FRAME_BYTES) {
      this.#fail(new Error('firmware response exceeds frame bound'));
      return;
    }
    const newline = this.#buffer.indexOf('\n');
    if (newline < 0) return;
    const line = this.#buffer.slice(0, newline);
    const trailing = this.#buffer.slice(newline + 1);
    this.#buffer = '';
    if (trailing.trim() !== '') {
      this.#fail(new Error('firmware sent multiple or trailing frames'));
      return;
    }
    if (!this.#waiter) {
      this.#fail(new Error('unsolicited or replayed firmware response'));
      return;
    }
    try {
      this.#waiter.resolve(parseFrame(line, 'firmware response'));
    } catch (error) {
      this.#fail(error);
    }
  }

  #fail(error) {
    const failure = error instanceof Error ? error : new Error('firmware transport closed');
    if (this.#waiter) this.#waiter.reject(failure);
    this.#closed = true;
  }
}
