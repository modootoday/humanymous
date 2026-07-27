import { randomUUID, timingSafeEqual } from 'node:crypto';
import {
  encodeFrame,
  MAX_FRAME_BYTES,
  parseFrame,
  PROTOCOL_VERSION,
} from '../usb-broker/protocol.mjs';

function equal(left, right) {
  if (typeof left !== 'string' || typeof right !== 'string') return false;
  const a = Buffer.from(left);
  const b = Buffer.from(right);
  return a.length === b.length && timingSafeEqual(a, b);
}

export class GatewayChannel {
  #transport;
  #attestation;
  #sessionId;
  #sequence = 0;
  #buffer = '';
  #waiter = null;
  #failed = false;

  constructor(transport, attestation, { sessionId = randomUUID() } = {}) {
    this.#transport = transport;
    this.#attestation = attestation;
    this.#sessionId = sessionId;
    transport.on('data', (chunk) => this.#receive(chunk));
    transport.on('error', (error) => this.#fail(error));
    transport.on('close', () => this.#fail(new Error('gateway transport closed')));
  }

  async handshake({ timeoutMs = 2_000 } = {}) {
    const nonce = randomUUID();
    const pending = this.#next(timeoutMs);
    this.#write({
      protocolVersion: PROTOCOL_VERSION,
      kind: 'hello',
      sessionId: this.#sessionId,
      nonce,
      deadlineUnixMs: Date.now() + timeoutMs,
    });
    const frame = await pending;
    if (frame.kind !== 'helloAck' || frame.protocolVersion !== PROTOCOL_VERSION ||
        !equal(frame.sessionId, this.#sessionId) || !equal(frame.nonce, nonce)) {
      throw new TypeError('gateway hello acknowledgement is not fresh');
    }
    const expected = this.#attestation;
    if (!equal(frame.identity?.modelId, expected.modelId) ||
        !equal(frame.identity?.profileManifestSha256, expected.profileManifestSha256) ||
        frame.identity?.descriptorSetId !== expected.descriptorSetId ||
        frame.identity?.usbOrigin !== 'kernel-emulated' ||
        frame.identity?.physicalCapable !== false) {
      throw new TypeError('gateway identity does not match admitted virtual USB profile');
    }
    this.#validateSafety(frame.safety);
    await this.command({ kind: 'releaseAll' });
  }

  async command(action, {
    commandId = randomUUID(),
    deadlineUnixMs = Date.now() + 2_000,
    timeoutMs = Math.max(1, deadlineUnixMs - Date.now()),
  } = {}) {
    if (this.#failed) throw new Error('gateway channel is failed');
    const sequence = ++this.#sequence;
    const pending = this.#next(Math.min(2_000, timeoutMs));
    this.#write({
      protocolVersion: PROTOCOL_VERSION,
      kind: 'command',
      sessionId: this.#sessionId,
      sequence,
      commandId,
      deadlineUnixMs,
      action,
    });
    const frame = await pending;
    if (frame.kind !== 'ack' || frame.protocolVersion !== PROTOCOL_VERSION ||
        !equal(frame.sessionId, this.#sessionId) || frame.sequence !== sequence ||
        !equal(frame.commandId, commandId) || frame.accepted !== true ||
        frame.releasedAll !== (action.kind === 'releaseAll')) {
      throw new TypeError('gateway acknowledgement is mismatched or replayed');
    }
    this.#validateSafety(frame.safety);
  }

  async close({ release = true } = {}) {
    let failure;
    if (release && !this.#failed) {
      try {
        await this.command({ kind: 'releaseAll' });
      } catch (error) {
        failure = error;
      }
    }
    this.#failed = true;
    this.#transport.destroy?.();
    if (failure) throw failure;
  }

  #validateSafety(safety) {
    if (safety?.deadManArmed !== true ||
        safety.deadManReleaseMs !== this.#attestation.deadManReleaseMs) {
      throw new TypeError('gateway dead-man state is not ready');
    }
  }

  #write(frame) {
    if (this.#transport.write(encodeFrame(frame)) === false) {
      this.#fail(new Error('gateway transport rejected command'));
    }
  }

  #next(timeoutMs) {
    if (this.#waiter) throw new Error('one gateway command may be in flight');
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#waiter = null;
        reject(new Error('gateway acknowledgement timed out'));
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
    this.#buffer += chunk.toString('utf8');
    if (Buffer.byteLength(this.#buffer) > MAX_FRAME_BYTES) {
      this.#fail(new Error('gateway response exceeds frame bound'));
      return;
    }
    const newline = this.#buffer.indexOf('\n');
    if (newline < 0) return;
    const line = this.#buffer.slice(0, newline);
    const trailing = this.#buffer.slice(newline + 1);
    this.#buffer = '';
    if (trailing.trim() || !this.#waiter) {
      this.#fail(new Error('gateway sent unsolicited or multiple responses'));
      return;
    }
    try {
      this.#waiter.resolve(parseFrame(line, 'gateway response'));
    } catch (error) {
      this.#fail(error);
    }
  }

  #fail(error) {
    this.#failed = true;
    this.#waiter?.reject(error);
  }
}

