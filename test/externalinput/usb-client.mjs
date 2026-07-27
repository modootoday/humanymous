import { randomUUID } from 'node:crypto';
import net from 'node:net';
import { CapabilityUnavailableError, ContractViolationError } from './errors.mjs';

const MAX_MESSAGE_BYTES = 4 * 1024;

export function sendUsbBrokerCommand(socketPath, action, { timeoutMs = 2_000 } = {}) {
  if (!socketPath) {
    throw new CapabilityUnavailableError('usb-command', 'USB broker command socket is missing');
  }
  const sequenceId = randomUUID();
  const envelope = {
    protocolVersion: '1.0.0',
    sequenceId,
    deadlineUnixMs: Date.now() + timeoutMs,
    action,
  };
  const encoded = `${JSON.stringify(envelope)}\n`;
  if (Buffer.byteLength(encoded) > MAX_MESSAGE_BYTES) {
    throw new ContractViolationError('USB broker command exceeds bound');
  }

  return new Promise((resolve, reject) => {
    const socket = net.createConnection({ path: socketPath });
    let bytes = 0;
    let response = '';
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new CapabilityUnavailableError('usb-broker', 'USB broker acknowledgement timed out'));
    }, timeoutMs);
    socket.on('connect', () => socket.write(encoded));
    socket.on('data', (chunk) => {
      bytes += chunk.length;
      if (bytes > MAX_MESSAGE_BYTES) {
        socket.destroy();
        reject(new ContractViolationError('USB broker response exceeds bound'));
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
          reject(new ContractViolationError('USB broker response sequence mismatch'));
        } else if (parsed?.accepted !== true) {
          reject(new CapabilityUnavailableError('usb-broker', parsed?.error || 'USB command rejected'));
        } else {
          resolve();
        }
      } catch (error) {
        reject(new ContractViolationError(`invalid USB broker response: ${error.message}`));
      }
    });
    socket.on('error', (error) => {
      clearTimeout(timer);
      reject(new CapabilityUnavailableError('usb-broker', `USB broker connection failed: ${error.message}`));
    });
    socket.on('close', () => clearTimeout(timer));
  });
}
