import assert from 'node:assert/strict';
import test from 'node:test';
import { evaluateExternalInputResults } from './assert-external-input.mjs';
import { CANONICAL_MODES } from '../test/externalinput/contracts.mjs';

function result(sequence, verdict = 'CHALLENGE') {
  const mode = CANONICAL_MODES[sequence - 1];
  const {
    profileId,
    observation,
    inputBackend,
    domRequired: dom,
    usbRequired: usb,
  } = mode;
  return {
    schemaVersion: '2.0.0',
    sequence,
    profileId,
    groundTruth: 'automation',
    browser: { engine: 'chromium', binarySha256: 'a'.repeat(64), sandbox: true },
    control: { observation, inputBackend },
    strategy: {},
    dom: { enabled: dom },
    usb: {
      required: usb,
      attested: usb,
      dedicatedSeat: usb,
      seatEventObserved: usb,
    },
    purity: {
      forbiddenArgv: false,
      debugPortListening: false,
      automationDependency: false,
      controllerHasLabNetwork: false,
      hostDisplayMounted: false,
      domMutationAttempt: false,
      mixedInputBackends: false,
      uinputPresent: false,
      domObserverPresent: dom,
      usbAssigned: usb,
      xtestEnabled: !usb,
    },
    tasks: Array.from({ length: 5 }, (_, index) => ({ id: String(index), status: 'PASS' })),
    measurement: { verdict, source: 'framebuffer', cueCount: 2 },
    status: verdict === 'ALLOW' ? 'RESIDUAL' : 'PASS',
  };
}

test('accepts the exact defended four-mode ladder', () => {
  const evaluated = evaluateExternalInputResults([1, 2, 3, 4].map((sequence) => result(sequence)));
  assert.equal(evaluated.ok, true, evaluated.failures.join('\n'));
});

test('accepts ALLOW only as an explicit residual', () => {
  const evaluated = evaluateExternalInputResults(
    [result(1, 'ALLOW'), result(2)],
    { expectedModes: 2 },
  );
  assert.equal(evaluated.ok, true, evaluated.failures.join('\n'));
  assert.equal(evaluated.summary.residuals, 1);
});

test('rejects unavailable USB and detector oracle fields', () => {
  const records = [1, 2, 3, 4].map((sequence) => result(sequence));
  records[2].status = 'UNAVAILABLE';
  records[3].riskScore = 91;
  const evaluated = evaluateExternalInputResults(records);
  assert.equal(evaluated.ok, false);
  assert.match(evaluated.failures.join('\n'), /UNAVAILABLE/);
  assert.match(evaluated.failures.join('\n'), /oracle/);
});
