import {
  ReplayGuard,
  sanitizeResult,
  validateEnvelope,
} from './extension/protocol.mjs';

export class NativeQueryBroker {
  #pending = new Map();
  #replayGuard;
  #maxPending;
  #now;
  #setTimer;
  #clearTimer;
  #writeNative;
  #reply;

  constructor({
    writeNative,
    reply,
    now = () => Date.now(),
    setTimer = (callback, delay) => setTimeout(callback, delay),
    clearTimer = (timer) => clearTimeout(timer),
    replayGuard = new ReplayGuard(),
    maxPending = 16,
  }) {
    if (typeof writeNative !== 'function' || typeof reply !== 'function') {
      throw new TypeError('broker requires writeNative and reply functions');
    }
    this.#writeNative = writeNative;
    this.#reply = reply;
    this.#now = now;
    this.#setTimer = setTimer;
    this.#clearTimer = clearTimer;
    this.#replayGuard = replayGuard;
    this.#maxPending = maxPending;
  }

  get pendingCount() {
    return this.#pending.size;
  }

  accept(rawEnvelope, client) {
    const now = this.#now();
    const envelope = validateEnvelope(rawEnvelope, { now });
    this.#replayGuard.accept(envelope.sequenceId, envelope.deadlineUnixMs, now);
    if (this.#pending.size >= this.#maxPending) throw new TypeError('too many in-flight queries');
    const timer = this.#setTimer(
      () => this.#expire(envelope.sequenceId),
      Math.max(1, envelope.deadlineUnixMs - now),
    );
    this.#pending.set(envelope.sequenceId, { client, envelope, timer });
    try {
      this.#writeNative(envelope);
    } catch (error) {
      this.#pending.delete(envelope.sequenceId);
      this.#clearTimer(timer);
      throw error;
    }
    return envelope.sequenceId;
  }

  handleNativeResponse(message) {
    if (!message || message.type !== 'response' || typeof message.sequenceId !== 'string') {
      return false;
    }
    const entry = this.#pending.get(message.sequenceId);
    if (!entry) return false;
    this.#pending.delete(message.sequenceId);
    this.#clearTimer(entry.timer);
    if (this.#now() > entry.envelope.deadlineUnixMs) {
      this.#reply(entry.client, { sequenceId: message.sequenceId, error: 'deadline expired' });
      return true;
    }
    if (message.error) {
      this.#reply(entry.client, {
        sequenceId: message.sequenceId,
        error: String(message.error).slice(0, 256),
      });
      return true;
    }
    try {
      const result = sanitizeResult(entry.envelope.request, message.result);
      this.#reply(entry.client, { sequenceId: message.sequenceId, result });
    } catch (error) {
      this.#reply(entry.client, {
        sequenceId: message.sequenceId,
        error: String(error?.message || error).slice(0, 256),
      });
    }
    return true;
  }

  cancelClient(client) {
    for (const [sequenceId, entry] of this.#pending) {
      if (entry.client !== client) continue;
      this.#clearTimer(entry.timer);
      this.#pending.delete(sequenceId);
    }
  }

  #expire(sequenceId) {
    const entry = this.#pending.get(sequenceId);
    if (!entry) return;
    this.#pending.delete(sequenceId);
    this.#reply(entry.client, { sequenceId, error: 'deadline expired' });
  }
}
