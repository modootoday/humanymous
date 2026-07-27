import assert from 'node:assert/strict';
import test from 'node:test';
import {
  createImeActionTracker,
  createImePolicy,
  validateImePolicy,
} from './ime-policy.mjs';

test('IME policy is run-bound and rejects any text action at the episode boundary', () => {
  const policy = createImePolicy('vusb-policy-test', 'ko-KR');
  assert.equal(validateImePolicy(policy, 'vusb-policy-test').locale, 'ko-KR');
  const tracker = createImeActionTracker(policy);
  assert.throws(
    () => tracker.validate({ kind: 'typeText', text: '한글' }),
    /differs from the fixed vector/,
  );
});

test('IME tracker accepts only the immutable action order and an all-up release', () => {
  const policy = createImePolicy('vusb-policy-order', 'ko-KR');
  const tracker = createImeActionTracker(policy);
  for (const expected of policy.actions) {
    tracker.validate(expected.kind === 'keyStroke'
      ? { ...expected, modifiers: [], dwellMs: 60, flightMs: 60 }
      : expected.kind === 'pointerMove'
        ? { kind: 'pointerMove', x: 10, y: 20, durationMs: 100 }
        : { kind: 'pointerClick', button: 'left', dwellMs: 60 });
  }
  tracker.validate({ kind: 'releaseAll' });
  assert.equal(tracker.snapshot().complete, true);
  assert.throws(
    () => tracker.validate({ kind: 'keyStroke', key: 'A', modifiers: [] }),
    /already consumed/,
  );
});
