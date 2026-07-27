import {
  NATIVE_HOST_NAME,
  sanitizeResult,
  validateEnvelope,
} from './protocol.mjs';

let contentPort = null;
let nativePort = null;
const pending = new Map();

function ensureNativePort() {
  if (nativePort) return nativePort;
  nativePort = chrome.runtime.connectNative(NATIVE_HOST_NAME);
  nativePort.onMessage.addListener((message) => {
    let envelope;
    try {
      envelope = validateEnvelope(message);
    } catch (error) {
      nativePort?.postMessage({
        type: 'response',
        sequenceId: typeof message?.sequenceId === 'string' ? message.sequenceId : '',
        error: String(error?.message || error).slice(0, 256),
      });
      return;
    }
    if (!contentPort) {
      nativePort?.postMessage({
        type: 'response',
        sequenceId: envelope.sequenceId,
        error: 'fixed fixture content script is not connected',
      });
      return;
    }
    if (pending.has(envelope.sequenceId)) {
      nativePort?.postMessage({
        type: 'response',
        sequenceId: envelope.sequenceId,
        error: 'duplicate in-flight sequence',
      });
      return;
    }
    pending.set(envelope.sequenceId, envelope);
    contentPort.postMessage({
      type: 'query',
      sequenceId: envelope.sequenceId,
      deadlineUnixMs: envelope.deadlineUnixMs,
      request: envelope.request,
    });
  });
  nativePort.onDisconnect.addListener(() => {
    nativePort = null;
    pending.clear();
  });
  return nativePort;
}

chrome.runtime.onConnect.addListener((port) => {
  if (port.name !== 'hmn-dom-observer-content') {
    port.disconnect();
    return;
  }
  if (contentPort) contentPort.disconnect();
  contentPort = port;
  ensureNativePort();
  port.onMessage.addListener((message) => {
    if (!message || message.type !== 'response') return;
    const envelope = pending.get(message.sequenceId);
    if (!envelope) return;
    pending.delete(message.sequenceId);
    if (Date.now() > envelope.deadlineUnixMs) {
      nativePort?.postMessage({
        type: 'response',
        sequenceId: envelope.sequenceId,
        error: 'deadline expired',
      });
      return;
    }
    if (message.error) {
      nativePort?.postMessage({
        type: 'response',
        sequenceId: envelope.sequenceId,
        error: String(message.error).slice(0, 256),
      });
      return;
    }
    try {
      const result = sanitizeResult(envelope.request, message.result);
      nativePort?.postMessage({
        type: 'response',
        sequenceId: envelope.sequenceId,
        result,
      });
    } catch (error) {
      nativePort?.postMessage({
        type: 'response',
        sequenceId: envelope.sequenceId,
        error: String(error?.message || error).slice(0, 256),
      });
    }
  });
  port.onDisconnect.addListener(() => {
    if (contentPort === port) contentPort = null;
  });
});
