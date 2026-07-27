import net from 'node:net';
import { randomUUID } from 'node:crypto';
import { ContractViolationError } from './errors.mjs';
import { PROTOCOL_VERSION } from './dom-observer/extension/protocol.mjs';

const MAX_RESPONSE_BYTES = 64 * 1024;
const MAX_REQUEST_BYTES = 4 * 1024;

export function queryDomSocket(socketPath, request, { timeoutMs = 2_000 } = {}) {
  if (!socketPath) throw new ContractViolationError('DOM socket path is missing');
  const sequenceId = randomUUID();
  const envelope = {
    protocolVersion: PROTOCOL_VERSION,
    sequenceId,
    deadlineUnixMs: Date.now() + timeoutMs,
    request,
  };
  const encoded = `${JSON.stringify(envelope)}\n`;
  if (Buffer.byteLength(encoded) > MAX_REQUEST_BYTES) {
    throw new ContractViolationError('DOM observer request exceeds bound');
  }
  return new Promise((resolve, reject) => {
    const socket = net.createConnection({ path: socketPath });
    let bytes = 0;
    let response = '';
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new Error('DOM observer query timed out'));
    }, timeoutMs);
    socket.on('connect', () => socket.write(encoded));
    socket.on('data', (chunk) => {
      bytes += chunk.length;
      if (bytes > MAX_RESPONSE_BYTES) {
        socket.destroy();
        reject(new ContractViolationError('DOM observer response exceeds bound'));
        return;
      }
      response += chunk.toString('utf8');
      const newline = response.indexOf('\n');
      if (newline < 0) return;
      clearTimeout(timer);
      socket.end();
      try {
        const parsed = JSON.parse(response.slice(0, newline));
        if (parsed?.sequenceId !== sequenceId) {
          reject(new ContractViolationError('DOM observer response sequence mismatch'));
          return;
        }
        if (parsed?.error) reject(new ContractViolationError(`DOM observer rejected query: ${parsed.error}`));
        else resolve(parsed?.result ?? null);
      } catch (error) {
        reject(new ContractViolationError(`invalid DOM observer response: ${error.message}`));
      }
    });
    socket.on('error', (error) => {
      clearTimeout(timer);
      reject(error);
    });
    socket.on('close', () => clearTimeout(timer));
  });
}
