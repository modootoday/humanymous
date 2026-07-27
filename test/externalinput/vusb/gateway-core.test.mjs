import assert from 'node:assert/strict';
import test from 'node:test';
import { decodeSignedByte, HidReportGateway } from './gateway-core.mjs';

function harness(overrides = {}) {
  const keyboard = [];
  const pointer = [];
  let now = 0;
  const gateway = new HidReportGateway({
    writeKeyboard: async (report) => keyboard.push(Buffer.from(report)),
    writePointer: async (report) => pointer.push(Buffer.from(report)),
    limits: {
      maxReportsPerSecond: 120,
      maxRelativeStep: 127,
      maxKeyDwellMs: 250,
      maxPointerDwellMs: 250,
      deadManReleaseMs: 500,
    },
    sleep: async (ms) => { now += ms; },
    now: () => now,
    ...overrides,
  });
  return { gateway, keyboard, pointer };
}

test('relative pointer movement is bounded and terminates in a neutral report', async () => {
  const { gateway, pointer } = harness();
  await gateway.perform({ kind: 'pointerMove', x: 1279, y: 719, durationMs: 100 });
  assert.ok(pointer.length > 2);
  for (const report of pointer.slice(0, -1)) {
    assert.ok(Math.abs(decodeSignedByte(report[1])) <= 127);
    assert.ok(Math.abs(decodeSignedByte(report[2])) <= 127);
  }
  assert.deepEqual([...pointer.at(-1)], [0, 0, 0, 0]);
  await gateway.close();
});

test('keyboard reports use the fixed boot-HID shape and release every key', async () => {
  const { gateway, keyboard } = harness();
  await gateway.perform({
    kind: 'keyStroke',
    key: 'A',
    modifiers: ['Shift'],
    dwellMs: 20,
    flightMs: 20,
  });
  assert.deepEqual([...keyboard[0]], [2, 0, 4, 0, 0, 0, 0, 0]);
  assert.deepEqual([...keyboard[1]], [0, 0, 0, 0, 0, 0, 0, 0]);
  await gateway.close();
});

test('forbidden shortcuts and raw reports have no gateway representation', async () => {
  const { gateway } = harness();
  await assert.rejects(
    () => gateway.perform({ kind: 'keyStroke', key: 'L', modifiers: ['Control'], dwellMs: 20, flightMs: 20 }),
    /shortcut modifier/,
  );
  await assert.rejects(
    () => gateway.perform({ kind: 'rawReport', bytes: [1, 2, 3] }),
    /action kind is forbidden/,
  );
  await gateway.close();
});

test('close always writes neutral keyboard and pointer reports', async () => {
  const { gateway, keyboard, pointer } = harness();
  await gateway.perform({ kind: 'pointerClick', button: 'left', dwellMs: 20 });
  await gateway.close();
  assert.deepEqual([...keyboard.at(-1)], [0, 0, 0, 0, 0, 0, 0, 0]);
  assert.deepEqual([...pointer.at(-1)], [0, 0, 0, 0]);
});

test('gateway independently enforces an episode action policy', async () => {
  const { gateway } = harness({
    actionPolicy: {
      validate(action) {
        if (action.kind === 'typeText') throw new TypeError('IME text actions are forbidden');
      },
      snapshot() {
        return { complete: false };
      },
    },
  });
  await assert.rejects(
    () => gateway.perform({ kind: 'typeText', text: 'humanymous synthetic task' }),
    /IME text actions are forbidden/,
  );
  await gateway.close();
});
