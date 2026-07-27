import { ContractViolationError } from '../errors.mjs';
import { RUN_ID } from './common.mjs';

export const IME_AXIS_VERSION = 'humanymous.ime-composition-vusb/v1';
export const IME_LOCALES = Object.freeze(['ko-KR', 'zh-CN', 'ja-JP']);
export const IME_ENGINES = Object.freeze(['chromium', 'firefox']);
export const IME_VECTORS = Object.freeze({
  'ko-KR': Object.freeze({
    vectorId: 'ko-dubeolsik-hangul-v1',
    engineId: 'hangul',
    usages: Object.freeze([...'GKSRMF']),
    commit: Object.freeze(['Enter']),
  }),
  'zh-CN': Object.freeze({
    vectorId: 'zh-full-pinyin-zhongwen-v1',
    engineId: 'libpinyin',
    usages: Object.freeze([...'ZHONGWEN']),
    commit: Object.freeze(['Space', 'Enter']),
  }),
  'ja-JP': Object.freeze({
    vectorId: 'ja-romaji-nihongo-v1',
    engineId: 'anthy',
    usages: Object.freeze([...'NIHONGO']),
    commit: Object.freeze(['Space', 'Enter']),
  }),
});
const EVDEV_KEY_CODES = Object.freeze({
  A: 30, B: 48, C: 46, D: 32, E: 18, F: 33, G: 34, H: 35,
  I: 23, J: 36, K: 37, L: 38, M: 50, N: 49, O: 24, P: 25,
  Q: 16, R: 19, S: 31, T: 20, U: 22, V: 47, W: 17, X: 45,
  Y: 21, Z: 44, Enter: 28, Space: 57,
});

export function vectorFor(locale) {
  const vector = IME_VECTORS[locale];
  if (!vector) throw new ContractViolationError('IME locale is not allowlisted');
  return vector;
}

export function evdevCodeForKey(key) {
  const code = EVDEV_KEY_CODES[key];
  if (!code) throw new ContractViolationError('IME key has no canonical evdev code');
  return code;
}

export function validateImeResult(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new ContractViolationError('IME result must be an object');
  }
  const allowed = new Set([
    'axisVersion', 'runId', 'browser', 'locale', 'vectorId', 'inputBackend',
    'actorRole', 'hidUsageCount', 'status', 'reason',
  ]);
  for (const field of Object.keys(value)) {
    if (!allowed.has(field)) throw new ContractViolationError(`unknown IME result field: ${field}`);
  }
  for (const field of [
    'axisVersion', 'runId', 'browser', 'locale', 'vectorId', 'inputBackend',
    'actorRole', 'hidUsageCount', 'status',
  ]) {
    if (!Object.hasOwn(value, field)) {
      throw new ContractViolationError(`IME result is missing field: ${field}`);
    }
  }
  if (value.axisVersion !== IME_AXIS_VERSION ||
      !RUN_ID.test(value.runId) ||
      !IME_ENGINES.includes(value.browser) ||
      !IME_LOCALES.includes(value.locale) ||
      value.vectorId !== vectorFor(value.locale).vectorId ||
      value.inputBackend !== 'usb-hid-emulated' ||
      value.actorRole !== 'input-only' ||
      !Number.isInteger(value.hidUsageCount) ||
      value.hidUsageCount < 1 || value.hidUsageCount > 32 ||
      !['ACTED', 'FAIL'].includes(value.status)) {
    throw new ContractViolationError('IME result identity or purity is invalid');
  }
  return Object.freeze({ ...value });
}
