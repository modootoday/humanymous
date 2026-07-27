// Offline SoT-41 artifact assertion. This process has no network and never feeds
// detector data back to the controller.
import { readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { CANONICAL_MODES } from '../test/externalinput/contracts.mjs';
const FORBIDDEN_ORACLE_FIELDS = new Set([
  'riskScore', 'hardRuleFired', 'topContributors', 'signals', 'contributors',
]);

function findForbiddenField(value, path = '$') {
  if (!value || typeof value !== 'object') return null;
  for (const [key, nested] of Object.entries(value)) {
    if (FORBIDDEN_ORACLE_FIELDS.has(key)) return `${path}.${key}`;
    const found = findForbiddenField(nested, `${path}.${key}`);
    if (found) return found;
  }
  return null;
}

function validateResult(result, expected, sequence, engine) {
  const failures = [];
  const {
    profileId,
    observation,
    inputBackend,
    domRequired,
    usbRequired,
  } = expected;
  if (result?.schemaVersion !== '2.0.0') failures.push(`${profileId}: schemaVersion must be 2.0.0`);
  if (result?.profileId !== profileId || result?.sequence !== sequence) {
    failures.push(`${profileId}: sequence/profile mismatch`);
  }
  if (result?.groundTruth !== 'automation') failures.push(`${profileId}: groundTruth must remain automation`);
  if (result?.browser?.engine !== engine) failures.push(`${profileId}: browser engine mismatch`);
  if (!['PASS', 'RESIDUAL'].includes(result?.status)) {
    failures.push(`${profileId}: status ${String(result?.status)} cannot satisfy completion`);
  }
  if (result?.control?.observation !== observation ||
      result?.control?.inputBackend !== inputBackend) {
    failures.push(`${profileId}: observation/input axes mismatch`);
  }
  if (result?.browser?.sandbox !== true ||
      !/^[a-f0-9]{64}$/.test(result?.browser?.binarySha256 || '')) {
    failures.push(`${profileId}: stock-browser sandbox/hash proof is incomplete`);
  }
  if (result?.measurement?.source !== 'framebuffer' ||
      !['ALLOW', 'CHALLENGE', 'DENY'].includes(result?.measurement?.verdict) ||
      result?.measurement?.cueCount < 2) {
    failures.push(`${profileId}: framebuffer verdict proof is incomplete`);
  }
  if (!Array.isArray(result?.tasks) || result.tasks.length !== 5 ||
      result.tasks.some((task) => task.status === 'FAIL')) {
    failures.push(`${profileId}: canonical task suite did not complete`);
  }
  const purity = result?.purity || {};
  for (const key of [
    'forbiddenArgv', 'debugPortListening', 'automationDependency',
    'controllerHasLabNetwork', 'hostDisplayMounted', 'domMutationAttempt',
    'mixedInputBackends', 'uinputPresent',
  ]) {
    if (purity[key] !== false) failures.push(`${profileId}: purity ${key} did not prove false`);
  }
  if (result?.dom?.enabled !== domRequired ||
      purity.domObserverPresent !== domRequired) {
    failures.push(`${profileId}: DOM observer presence mismatch`);
  }
  if (result?.usb?.required !== usbRequired || purity.usbAssigned !== usbRequired) {
    failures.push(`${profileId}: USB assignment mismatch`);
  }
  if (usbRequired && (
    result?.usb?.attested !== true ||
    result?.usb?.dedicatedSeat !== true ||
    result?.usb?.seatEventObserved !== true ||
    purity.xtestEnabled !== false
  )) {
    failures.push(`${profileId}: physical USB HIL proof is incomplete`);
  }
  if (!usbRequired && purity.xtestEnabled !== true) {
    failures.push(`${profileId}: virtual XTEST proof is incomplete`);
  }
  const oracle = findForbiddenField(result);
  if (oracle) failures.push(`${profileId}: detector oracle field retained at ${oracle}`);
  return failures;
}

export function evaluateExternalInputResults(results, {
  engine = 'chromium',
  expectedModes = 4,
} = {}) {
  const failures = [];
  if (!Number.isInteger(expectedModes) || expectedModes < 1 || expectedModes > 4) {
    return { ok: false, failures: ['expectedModes must be 1..4'] };
  }
  if (!Array.isArray(results) || results.length !== expectedModes) {
    return {
      ok: false,
      failures: [`expected exactly ${expectedModes} result(s), got ${Array.isArray(results) ? results.length : 'non-array'}`],
    };
  }
  for (let index = 0; index < expectedModes; index += 1) {
    failures.push(...validateResult(
      results[index],
      CANONICAL_MODES[index],
      index + 1,
      engine,
    ));
  }
  return {
    ok: failures.length === 0,
    failures,
    summary: {
      engine,
      modes: expectedModes,
      passed: results.filter((result) => result.status === 'PASS').length,
      residuals: results.filter((result) => result.status === 'RESIDUAL').length,
    },
  };
}

function readResults(root, expectedModes) {
  const files = readdirSync(root)
    .filter((name) => name.endsWith('.result.json'))
    .sort((left, right) => {
      const leftMode = CANONICAL_MODES.findIndex(({ profileId }) =>
        left.startsWith(profileId));
      const rightMode = CANONICAL_MODES.findIndex(({ profileId }) =>
        right.startsWith(profileId));
      return leftMode - rightMode;
    })
    .slice(0, expectedModes);
  return files.map((name) => JSON.parse(readFileSync(resolve(root, name), 'utf8')));
}

export function main(root = process.argv[2] || '/artifacts/external-input') {
  const expectedModes = Number(process.env.HM_EXTERNAL_EXPECT_MODES || 4);
  const engine = process.env.HM_EXTERNAL_BROWSER || 'chromium';
  const result = evaluateExternalInputResults(readResults(root, expectedModes), {
    engine,
    expectedModes,
  });
  for (const failure of result.failures) console.error(`FAIL: ${failure}`);
  if (!result.ok) return 1;
  console.log(`PASS: external-input ${engine} ${expectedModes}-mode ladder (${result.summary.passed} defended, ${result.summary.residuals} honest residual)`);
  return 0;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) process.exitCode = main();
