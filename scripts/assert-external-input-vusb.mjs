// Offline SoT-42 ladder assertion. It verifies execution/purity evidence but
// never computes detector scores or changes the measured verdict.
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { assertVirtualUsbAttestation } from '../test/externalinput/input.mjs';
import {
  BROWSERS,
  IME_CELLS,
  VIRTUAL_USB_CONTROL_CELLS,
} from '../test/externalinput/matrix.mjs';
import {
  IME_LOCALES,
  validateImeResult,
} from '../test/externalinput/vusb/ime-contract.mjs';
import { parseStrictJson } from '../test/externalinput/vusb/strict-json.mjs';
import { sha256 } from '../test/externalinput/vusb/common.mjs';
import {
  loadLadderManifest,
  validateAttemptManifest,
} from '../test/externalinput/vusb/manifest.mjs';

async function document(path, label) {
  const raw = await readFile(path, 'utf8');
  return {
    value: parseStrictJson(raw, label),
    hash: `sha256:${sha256(raw)}`,
  };
}

async function json(path, label) {
  return (await document(path, label)).value;
}

export async function assertVirtualUsbLadder(root) {
  const failures = [];
  const results = [];
  const imeFailures = [];
  const imeResults = [];
  let ladder;
  try {
    ladder = await loadLadderManifest(resolve(root, 'manifest', 'ladder.json'));
  } catch (error) {
    failures.push(`ladder manifest: ${error.message}`);
  }
  async function attemptDocument(axis, cell, result) {
    if (!ladder) throw new TypeError('verified ladder manifest is unavailable');
    const episode = axis === 'control'
      ? `m${cell.sequence}`
      : `ime-${cell.locale.split('-')[0]}`;
    const attempt = await document(
      resolve(root, `${cell.browser}-${episode}`, 'manifest', 'attempt.json'),
      `${cell.browser} ${episode} attempt manifest`,
    );
    const value = validateAttemptManifest(attempt.value, ladder.value);
    const childProject = `hmn-ext-${result.runId}-m${cell.sequence}`.slice(0, 63);
    if (value.ladderManifestSha256 !== ladder.sha256 ||
        value.runId !== result.runId ||
        value.childProject !== childProject ||
        value.axis !== axis ||
        value.browser !== cell.browser ||
        value.sequence !== cell.sequence ||
        value.profileId !== cell.profileId ||
        value.locale !== (cell.locale || '')) {
      throw new TypeError('attempt manifest/result identity mismatch');
    }
    return attempt;
  }
  for (const cell of VIRTUAL_USB_CONTROL_CELLS) {
      const {
        browser: engine, sequence, profileId, observation, inputBackend,
      } = cell;
      let result;
      try {
        result = await json(
          resolve(root, `${engine}-m${sequence}.result.json`),
          `${engine} mode ${sequence} result`,
        );
      } catch (error) {
        failures.push(error.message);
        continue;
      }
      results.push(result);
      let attempt;
      try {
        attempt = await attemptDocument('control', cell, result);
      } catch (error) {
        failures.push(`${engine} mode ${sequence}: ${error.message}`);
      }
      if (result.profileId !== profileId || result.sequence !== sequence ||
          result.browser?.engine !== engine) {
        failures.push(`${engine} mode ${sequence}: ladder identity mismatch`);
      }
      if (!['PASS', 'RESIDUAL'].includes(result.status)) {
        failures.push(`${engine} mode ${sequence}: status ${result.status} is not complete`);
      }
      if (result.control?.observation !== observation ||
          result.control?.inputBackend !== inputBackend) {
        failures.push(`${engine} mode ${sequence}: control axes mismatch`);
      }
      if (result.purity?.uinputPresent !== false ||
          result.purity?.mixedInputBackends !== false) {
        failures.push(`${engine} mode ${sequence}: input purity is incomplete`);
      }
      if (sequence <= 2) {
        if (result.usb?.required !== false || result.purity?.xtestEnabled !== true) {
          failures.push(`${engine} mode ${sequence}: virtual-control evidence mismatch`);
        }
      } else {
        try {
          const { required, ...attestation } = result.usb || {};
          if (required !== true || Object.hasOwn(attestation, 'attested')) {
            throw new TypeError('virtual USB must use the emulation-specific attestation namespace');
          }
          assertVirtualUsbAttestation(attestation);
          if (attestation.runId !== result.runId) {
            throw new TypeError('virtual USB attestation is not bound to the result run');
          }
          if (result.purity?.xtestEnabled !== false || result.purity?.usbAssigned !== true) {
            throw new TypeError('virtual USB purity is incomplete');
          }
          const terminal = await json(
            resolve(root, `${engine}-m${sequence}.terminal.json`),
            `${engine} mode ${sequence} terminal receipt`,
          );
          const resultRaw = await readFile(
            resolve(root, `${engine}-m${sequence}.result.json`),
            'utf8',
          );
          if (terminal.kind !== 'terminal' || terminal.canonical !== true ||
              terminal.status !== result.status || terminal.runId !== result.runId ||
              terminal.profileId !== result.profileId ||
              terminal.browserEngine !== engine ||
              terminal.resultSha256 !== `sha256:${sha256(resultRaw)}` ||
              terminal.selectedBrowserImageDigest !==
                attempt?.value.selectedBrowserImageDigest ||
              terminal.evidence?.ladderManifestSha256 !== ladder?.sha256 ||
              terminal.evidence?.attemptManifestSha256 !== attempt?.hash) {
            throw new TypeError('terminal cleanup/result status mismatch');
          }
        } catch (error) {
          failures.push(`${engine} mode ${sequence}: ${error.message}`);
        }
      }
  }
  if (failures.length === 0) {
    for (const { browser: engine, locale } of IME_CELLS) {
        try {
          const resultDocument = await document(
            resolve(root, `${engine}-${locale}.ime.result.json`),
            `${engine} ${locale} IME result`,
          );
          const result = validateImeResult(resultDocument.value);
          imeResults.push(result);
          if (result.browser !== engine || result.locale !== locale || result.status !== 'ACTED') {
            throw new TypeError('IME cell identity or completion mismatch');
          }
          const attempt = await attemptDocument('ime', {
            browser: engine,
            locale,
            sequence: 3,
            profileId: 'external_input_vusb',
          }, result);
          const terminal = await json(
            resolve(root, `${engine}-${locale}.ime.terminal.json`),
            `${engine} ${locale} IME terminal receipt`,
          );
          if (terminal.kind !== 'terminal' || terminal.canonical !== true ||
              terminal.status !== 'PASS' ||
              terminal.measurementVerdict !== 'NOT_APPLICABLE' ||
              terminal.runId !== result.runId ||
              terminal.profileId !== 'ime-composition-vusb' ||
              terminal.axis !== 'ime-composition-vusb' ||
              terminal.browserEngine !== engine ||
              terminal.locale !== locale ||
              terminal.vectorId !== result.vectorId ||
              terminal.resultSha256 !== resultDocument.hash ||
              terminal.selectedBrowserImageDigest !==
                attempt.value.selectedBrowserImageDigest ||
              terminal.evidence?.ladderManifestSha256 !== ladder?.sha256 ||
              terminal.evidence?.attemptManifestSha256 !== attempt.hash) {
            throw new TypeError('IME terminal cleanup receipt mismatch');
          }
        } catch (error) {
          imeFailures.push(`${engine} ${locale}: ${error.message}`);
        }
    }
  } else {
    imeFailures.push('BLOCKED_BY_BASE_LADDER');
  }
  const summary = {
    schemaVersion: 'humanymous.virtual-usb-ladder/v1',
    canonical: failures.length === 0 && imeFailures.length === 0,
    suiteComplete: failures.length === 0 && imeFailures.length === 0,
    engines: BROWSERS,
    ladderManifestSha256: ladder?.sha256 || '',
    order: VIRTUAL_USB_CONTROL_CELLS.slice(0, 4).map(({ profileId }) => profileId),
    measured: results.length,
    pass: results.filter(({ status }) => status === 'PASS').length,
    residual: results.filter(({ status }) => status === 'RESIDUAL').length,
    failures,
    control: {
      expected: VIRTUAL_USB_CONTROL_CELLS.length,
      measured: results.length,
      pass: results.filter(({ status }) => status === 'PASS').length,
      residual: results.filter(({ status }) => status === 'RESIDUAL').length,
      complete: failures.length === 0 &&
        results.length === VIRTUAL_USB_CONTROL_CELLS.length,
      failures,
    },
    ime: {
      schemaVersion: 'humanymous.ime-composition-vusb/v1',
      order: IME_LOCALES,
      expected: IME_CELLS.length,
      measured: imeResults.length,
      pass: imeResults.filter(({ status }) => status === 'ACTED').length,
      complete: imeFailures.length === 0 && imeResults.length === IME_CELLS.length,
      failures: imeFailures,
    },
  };
  return summary;
}

export async function main(
  root = process.argv[2],
  destination = process.env.HM_VUSB_LADDER_RESULT || '',
) {
  if (!root) throw new TypeError('ladder artifact root is required');
  const result = await assertVirtualUsbLadder(resolve(root));
  const output = destination ? resolve(destination) : resolve(root, 'ladder-result.json');
  await writeFile(
    output,
    `${JSON.stringify(result, null, 2)}\n`,
    { encoding: 'utf8', flag: 'wx', mode: 0o600 },
  );
  for (const failure of result.failures) process.stderr.write(`FAIL: ${failure}\n`);
  for (const failure of result.ime.failures) process.stderr.write(`IME FAIL: ${failure}\n`);
  if (!result.canonical) return 1;
  process.stdout.write(
    `PASS: virtual USB ladder (English 8/8; IME ${result.ime.pass}/${result.ime.expected}; ` +
      `${result.pass} defended, ${result.residual} honest residual)\n`,
  );
  return 0;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().then((code) => {
    process.exitCode = code;
  }).catch((error) => {
    process.stderr.write(`virtual USB assertion failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
