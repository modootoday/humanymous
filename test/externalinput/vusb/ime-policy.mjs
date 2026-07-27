import { readFile, writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { RUN_ID, canonicalJson, sha256 } from './common.mjs';
import { IME_AXIS_VERSION, IME_LOCALES, vectorFor } from './ime-contract.mjs';

export const IME_POLICY_VERSION = 'humanymous.ime-hid-policy/v1';

function exactFields(value, fields, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  const expected = new Set(fields);
  for (const field of Object.keys(value)) {
    if (!expected.has(field)) throw new TypeError(`${label} has unknown field: ${field}`);
  }
  for (const field of expected) {
    if (!Object.hasOwn(value, field)) throw new TypeError(`${label} is missing field: ${field}`);
  }
}

function actionTemplate(kind, key) {
  return key ? Object.freeze({ kind, key }) : Object.freeze({ kind });
}

export function createImePolicy(runId, locale) {
  if (!RUN_ID.test(runId || '')) throw new TypeError('IME policy run ID is invalid');
  if (!IME_LOCALES.includes(locale)) throw new TypeError('IME policy locale is invalid');
  const vector = vectorFor(locale);
  const actions = [
    actionTemplate('pointerMove'),
    actionTemplate('pointerClick'),
    ...vector.usages.map((key) => actionTemplate('keyStroke', key)),
    ...vector.commit.map((key) => actionTemplate('keyStroke', key)),
  ];
  const body = {
    schemaVersion: IME_POLICY_VERSION,
    axisVersion: IME_AXIS_VERSION,
    runId,
    locale,
    vectorId: vector.vectorId,
    actions,
  };
  return Object.freeze({
    ...body,
    policySha256: `sha256:${sha256(canonicalJson(body))}`,
  });
}

export function validateImePolicy(value, expectedRunId = '') {
  exactFields(value, [
    'schemaVersion', 'axisVersion', 'runId', 'locale', 'vectorId',
    'actions', 'policySha256',
  ], 'IME HID policy');
  const expected = createImePolicy(value.runId, value.locale);
  if (expectedRunId && value.runId !== expectedRunId) {
    throw new TypeError('IME HID policy is not bound to this run');
  }
  if (canonicalJson(value) !== canonicalJson(expected)) {
    throw new TypeError('IME HID policy differs from the fixed locale vector');
  }
  return expected;
}

export async function loadImePolicy(path, expectedRunId = '') {
  return validateImePolicy(JSON.parse(await readFile(path, 'utf8')), expectedRunId);
}

export function createImeActionTracker(policy) {
  const validated = validateImePolicy(policy);
  let index = 0;
  let releaseAllCount = 0;

  return Object.freeze({
    validate(action) {
      if (action?.kind === 'releaseAll') {
        releaseAllCount += 1;
        return action;
      }
      const expected = validated.actions[index];
      if (!expected) throw new TypeError('IME HID episode already consumed its fixed action vector');
      if (action?.kind !== expected.kind ||
          (expected.kind === 'keyStroke' &&
            (action.key !== expected.key || (action.modifiers || []).length !== 0))) {
        throw new TypeError(`IME HID action ${index + 1} differs from the fixed vector`);
      }
      index += 1;
      return action;
    },
    snapshot() {
      return Object.freeze({
        policySha256: validated.policySha256,
        expectedActionCount: validated.actions.length,
        acceptedActionCount: index,
        releaseAllCount,
        complete: index === validated.actions.length && releaseAllCount > 0,
      });
    },
  });
}

async function main() {
  const [action, runId, locale, destination] = process.argv.slice(2);
  if (action !== 'write' || !destination) {
    throw new TypeError('usage: ime-policy.mjs write <run-id> <locale> <destination>');
  }
  await writeFile(destination, `${JSON.stringify(createImePolicy(runId, locale), null, 2)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o600,
  });
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 2;
  });
}
