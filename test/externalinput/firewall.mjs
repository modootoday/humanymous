import { ContractViolationError } from './errors.mjs';

const ALLOWED_KEYS = new Set([
  'Tab', 'Enter', 'Space', 'Escape', 'Backspace', 'Delete',
  'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight',
  ...'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.split(''),
]);
const ALLOWED_BUTTONS = new Set(['left']);
const ALLOWED_KINDS = new Set([
  'pointerMove', 'pointerClick', 'scroll', 'keyStroke', 'typeText', 'pause', 'releaseAll',
]);

function finiteNumber(value, field) {
  if (!Number.isFinite(value)) throw new ContractViolationError(`${field} must be finite`);
  return value;
}

function rejectUnknownFields(action, allowed) {
  for (const key of Object.keys(action)) {
    if (!allowed.has(key)) throw new ContractViolationError(`unknown action field: ${key}`);
  }
}

export function createActionFirewall({
  width = 1280,
  height = 720,
  maxPauseMs = 5_000,
  maxScroll = 1_200,
  syntheticTexts = ['humanymous synthetic task'],
} = {}) {
  const textAllowlist = new Set(syntheticTexts);

  return Object.freeze({
    validate(action) {
      if (!action || typeof action !== 'object' || Array.isArray(action)) {
        throw new ContractViolationError('action must be a typed object');
      }
      if (!ALLOWED_KINDS.has(action.kind)) {
        throw new ContractViolationError(`forbidden action kind: ${String(action.kind)}`);
      }

      switch (action.kind) {
        case 'pointerMove': {
          rejectUnknownFields(action, new Set(['kind', 'x', 'y', 'durationMs']));
          const x = finiteNumber(action.x, 'x');
          const y = finiteNumber(action.y, 'y');
          const durationMs = finiteNumber(action.durationMs ?? 0, 'durationMs');
          if (x < 0 || x >= width || y < 0 || y >= height || durationMs < 0 || durationMs > 2_000) {
            throw new ContractViolationError('pointerMove exceeds display or duration bounds');
          }
          break;
        }
        case 'pointerClick': {
          rejectUnknownFields(action, new Set(['kind', 'button', 'dwellMs']));
          if (!ALLOWED_BUTTONS.has(action.button)) {
            throw new ContractViolationError('pointerClick button is not allowlisted');
          }
          const dwellMs = finiteNumber(action.dwellMs ?? 60, 'dwellMs');
          if (dwellMs < 20 || dwellMs > 250) {
            throw new ContractViolationError('pointerClick dwell exceeds bounds');
          }
          break;
        }
        case 'scroll': {
          rejectUnknownFields(action, new Set(['kind', 'dx', 'dy']));
          const dx = finiteNumber(action.dx, 'dx');
          const dy = finiteNumber(action.dy, 'dy');
          if (dx !== 0 || Math.abs(dy) > maxScroll) {
            throw new ContractViolationError('scroll exceeds bounded task range');
          }
          break;
        }
        case 'keyStroke': {
          rejectUnknownFields(action, new Set(['kind', 'key', 'modifiers', 'dwellMs', 'flightMs']));
          if (!ALLOWED_KEYS.has(action.key)) {
            throw new ContractViolationError(`key is not allowlisted: ${String(action.key)}`);
          }
          const modifiers = action.modifiers || [];
          if (!Array.isArray(modifiers) || modifiers.some((value) => value !== 'Shift')) {
            throw new ContractViolationError('system/browser shortcut modifiers are forbidden');
          }
          for (const field of ['dwellMs', 'flightMs']) {
            const value = action[field] ?? 60;
            if (!Number.isFinite(value) || value < 20 || value > 250) {
              throw new ContractViolationError(`${field} is outside the bounded human-paced range`);
            }
          }
          break;
        }
        case 'typeText':
          rejectUnknownFields(action, new Set(['kind', 'text']));
          if (!textAllowlist.has(action.text)) {
            throw new ContractViolationError('typed text is not a fixed synthetic task value');
          }
          break;
        case 'pause': {
          rejectUnknownFields(action, new Set(['kind', 'durationMs']));
          const durationMs = finiteNumber(action.durationMs, 'durationMs');
          if (durationMs < 0 || durationMs > maxPauseMs) {
            throw new ContractViolationError('pause exceeds episode bound');
          }
          break;
        }
        case 'releaseAll':
          rejectUnknownFields(action, new Set(['kind']));
          break;
        default:
          throw new ContractViolationError('unreachable action kind');
      }
      return Object.freeze({ ...action });
    },
  });
}
