import test from 'node:test';
import assert from 'node:assert/strict';
import { IME_AXIS_VERSION, IME_LOCALES, validateImeResult, vectorFor } from './ime-contract.mjs';

test('IME vectors contain only allowlisted physical-key usages', () => {
  for (const locale of IME_LOCALES) {
    const vector = vectorFor(locale);
    assert.ok(vector.usages.every((key) => /^[A-Z]$/.test(key)));
    assert.ok(vector.commit.every((key) => key === 'Space' || key === 'Enter'));
    assert.equal(JSON.stringify(vector).match(/[\u3000-\u9fff\uac00-\ud7af]/u), null);
  }
});

test('input-only actor cannot self-assert framebuffer, direct-Unicode, or bus purity', () => {
  assert.throws(() => validateImeResult({
    axisVersion: IME_AXIS_VERSION,
    runId: 'run',
    browser: 'chromium',
    locale: 'ko-KR',
    vectorId: vectorFor('ko-KR').vectorId,
    inputBackend: 'usb-hid-emulated',
    actorRole: 'input-only',
    hidUsageCount: 7,
    preeditFramebufferSha256: 'a'.repeat(64),
    commitFramebufferSha256: 'b'.repeat(64),
    directUnicodeCapabilityPresent: true,
    status: 'ACTED',
  }), /unknown IME result field/);
});
