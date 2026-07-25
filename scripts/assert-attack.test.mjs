import assert from 'node:assert/strict';
import test from 'node:test';
import { evaluateAttackResults } from './assert-attack.mjs';

const valid = [
  { profile: 'human.mjs', label: 'human', verdict: 'ALLOW' },
  { profile: 'bot.mjs', label: 'bot:test', verdict: 'DENY' },
  { profile: 'ceiling.mjs', label: 'ceiling:test', verdict: 'ALLOW' },
];

test('accepts a complete fail-closed catalog', () => {
  assert.equal(evaluateAttackResults(valid, 3).ok, true);
});

for (const [name, record, message] of [
  ['error', { profile: 'bot.mjs', label: 'bot:test', error: 'boom' }, 'errored'],
  ['skip', { profile: 'bot.mjs', skipped: true, reason: 'missing browser' }, 'skipped'],
  ['escape', { profile: 'bot.mjs', label: 'bot:test', verdict: 'ALLOW' }, 'reached ALLOW'],
]) {
  test(`rejects a profile ${name}`, () => {
    const result = evaluateAttackResults([valid[0], record, valid[2]], 3);
    assert.equal(result.ok, false);
    assert.match(result.failures.join('\n'), new RegExp(message));
  });
}

test('rejects silently missing catalog profiles', () => {
  const result = evaluateAttackResults(valid.slice(0, 2), 3);
  assert.equal(result.ok, false);
  assert.match(result.failures.join('\n'), /catalog coverage/);
});
