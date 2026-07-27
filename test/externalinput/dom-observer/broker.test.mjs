import assert from 'node:assert/strict';
import test from 'node:test';
import { NativeQueryBroker } from './broker.mjs';
import { PROTOCOL_VERSION } from './extension/protocol.mjs';

const NOW = 1_900_000_000_000;
const SEQUENCE = '20000000-0000-4000-8000-000000000001';

function envelope(sequenceId = SEQUENCE) {
  return {
    protocolVersion: PROTOCOL_VERSION,
    sequenceId,
    deadlineUnixMs: NOW + 1_000,
    request: { method: 'findByTextToken', token: 'choice-correct' },
  };
}

function harness() {
  const forwarded = [];
  const replies = [];
  const timers = [];
  const cleared = [];
  const broker = new NativeQueryBroker({
    now: () => NOW,
    writeNative: (value) => forwarded.push(value),
    reply: (client, value) => replies.push({ client, value }),
    setTimer: (callback, delay) => {
      const timer = { callback, delay };
      timers.push(timer);
      return timer;
    },
    clearTimer: (timer) => cleared.push(timer),
  });
  return { broker, forwarded, replies, timers, cleared };
}

test('broker binds one socket request to one sanitized native response', () => {
  const h = harness();
  const client = {};
  h.broker.accept(envelope(), client);
  assert.equal(h.broker.pendingCount, 1);
  assert.deepEqual(h.forwarded, [envelope()]);
  const handled = h.broker.handleNativeResponse({
    type: 'response',
    sequenceId: SEQUENCE,
    result: {
      token: 'choice-correct',
      rect: { x: 1, y: 2, width: 30, height: 40 },
      enabled: true,
      visible: true,
      nodeId: 'opaque_1',
    },
  });
  assert.equal(handled, true);
  assert.equal(h.broker.pendingCount, 0);
  assert.equal(h.replies[0].client, client);
  assert.equal(h.replies[0].value.result.token, 'choice-correct');
  assert.equal(h.cleared.length, 1);
});

test('broker converts an unsafe extension response into a bounded error', () => {
  const h = harness();
  h.broker.accept(envelope(), {});
  h.broker.handleNativeResponse({
    type: 'response',
    sequenceId: SEQUENCE,
    result: {
      token: 'choice-correct',
      rect: { x: 1, y: 2, width: 30, height: 40 },
      enabled: true,
      visible: true,
      nodeId: 'opaque_1',
      value: 'raw secret',
    },
  });
  assert.match(h.replies[0].value.error, /unknown field/);
  assert.equal('result' in h.replies[0].value, false);
});

test('broker rejects replay and expires an unanswered query', () => {
  const h = harness();
  const client = {};
  h.broker.accept(envelope(), client);
  h.timers[0].callback();
  assert.equal(h.broker.pendingCount, 0);
  assert.deepEqual(h.replies[0], {
    client,
    value: { sequenceId: SEQUENCE, error: 'deadline expired' },
  });
  assert.throws(() => h.broker.accept(envelope(), {}), /replayed/);
});

test('broker cancels pending work when a controller socket closes', () => {
  const h = harness();
  const first = {};
  const second = {};
  h.broker.accept(envelope(SEQUENCE), first);
  h.broker.accept(envelope('20000000-0000-4000-8000-000000000002'), second);
  h.broker.cancelClient(first);
  assert.equal(h.broker.pendingCount, 1);
  assert.equal(h.cleared.length, 1);
});
