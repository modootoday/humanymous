import net from 'node:net';
import { lstat, unlink } from 'node:fs/promises';
import {
  encodeFrame,
  MAX_FRAME_BYTES,
  parseFrame,
  PROTOCOL_VERSION,
  validateClientEnvelope,
} from './protocol.mjs';

const MAX_REPLAY_IDS = 4_096;

async function prepareSocketPath(socketPath) {
  const existing = await lstat(socketPath).catch((error) => {
    if (error.code === 'ENOENT') return null;
    throw error;
  });
  if (!existing) return;
  if (!existing.isSocket()) throw new Error('USB broker socket path exists and is not a socket');
  await unlink(socketPath);
}

export class UsbBroker {
  #socketPath;
  #firmware;
  #server = null;
  #seen = new Map();
  #queue = Promise.resolve();
  #stopping = false;
  #clients = new Set();
  #actionPolicy;
  #onFirmwareAccepted;

  constructor({ socketPath, firmware, actionPolicy = null, onFirmwareAccepted = null }) {
    if (!socketPath) throw new TypeError('USB broker socket path is required');
    if (!firmware || typeof firmware.command !== 'function') {
      throw new TypeError('firmware channel is required');
    }
    this.#socketPath = socketPath;
    this.#firmware = firmware;
    if (actionPolicy && typeof actionPolicy.validate !== 'function') {
      throw new TypeError('broker action policy must expose validate');
    }
    this.#actionPolicy = actionPolicy;
    if (onFirmwareAccepted && typeof onFirmwareAccepted !== 'function') {
      throw new TypeError('broker acceptance observer must be a function');
    }
    this.#onFirmwareAccepted = onFirmwareAccepted;
  }

  async start() {
    await prepareSocketPath(this.#socketPath);
    this.#server = net.createServer((client) => this.#accept(client));
    await new Promise((resolve, reject) => {
      this.#server.once('error', reject);
      this.#server.listen(this.#socketPath, () => {
        this.#server.off('error', reject);
        resolve();
      });
    });
  }

  async stop() {
    if (this.#stopping) return;
    this.#stopping = true;
    if (this.#server) {
      const closed = new Promise((resolve) => this.#server.close(resolve));
      for (const client of this.#clients) client.destroy();
      await closed;
    }
    await this.#queue.catch(() => {});
    await this.#firmware.close({ release: true });
    await unlink(this.#socketPath).catch((error) => {
      if (error.code !== 'ENOENT') throw error;
    });
  }

  #accept(client) {
    this.#clients.add(client);
    client.setTimeout(5_000, () => client.destroy());
    client.once('close', () => this.#clients.delete(client));
    let input = '';
    let handled = false;
    const reject = (sequenceId, error) => {
      if (client.destroyed) return;
      client.end(encodeFrame({
        protocolVersion: PROTOCOL_VERSION,
        sequenceId: typeof sequenceId === 'string' ? sequenceId : '',
        accepted: false,
        error: error instanceof Error ? error.message : String(error),
      }));
    };

    client.on('data', (chunk) => {
      if (handled) {
        client.destroy();
        return;
      }
      input += chunk.toString('utf8');
      if (Buffer.byteLength(input) > MAX_FRAME_BYTES) {
        handled = true;
        reject('', new Error('USB broker request exceeds frame bound'));
        return;
      }
      const newline = input.indexOf('\n');
      if (newline < 0) return;
      handled = true;
      const trailing = input.slice(newline + 1);
      let raw;
      try {
        raw = parseFrame(input.slice(0, newline), 'USB broker request');
        if (trailing.trim() !== '') throw new TypeError('multiple requests per connection are forbidden');
        const envelope = validateClientEnvelope(raw);
        this.#actionPolicy?.validate(envelope.action);
        this.#remember(envelope.sequenceId, envelope.deadlineUnixMs);
        this.#queue = this.#queue.then(async () => {
          if (this.#stopping) throw new Error('USB broker is stopping');
          if (Date.now() >= envelope.deadlineUnixMs) throw new Error('deadline expired before dispatch');
          await this.#firmware.command(envelope.action, {
            commandId: envelope.sequenceId,
            deadlineUnixMs: envelope.deadlineUnixMs,
          });
          this.#onFirmwareAccepted?.(envelope);
          client.end(encodeFrame({
            protocolVersion: PROTOCOL_VERSION,
            sequenceId: envelope.sequenceId,
            accepted: true,
          }));
        });
        this.#queue.catch((error) => reject(envelope.sequenceId, error));
      } catch (error) {
        reject(raw?.sequenceId, error);
      }
    });
    client.on('error', () => {});
  }

  #remember(sequenceId, deadlineUnixMs) {
    const now = Date.now();
    for (const [id, deadline] of this.#seen) {
      if (deadline <= now) this.#seen.delete(id);
    }
    if (this.#seen.has(sequenceId)) throw new TypeError('replayed sequence ID');
    if (this.#seen.size >= MAX_REPLAY_IDS) throw new TypeError('replay guard capacity exceeded');
    this.#seen.set(sequenceId, deadlineUnixMs);
  }
}
