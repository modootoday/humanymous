import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { assertVirtualUsbAttestation } from '../input.mjs';
import { atomicJson, receiptBase, sha256 } from './common.mjs';
import { parseStrictJson } from './strict-json.mjs';
import { readSeatEvidence } from './seat-evidence.mjs';

const FORBIDDEN = new Set([
  'riskScore', 'hardRuleFired', 'topContributors', 'signals', 'contributors',
]);

function forbiddenField(value, path = '$') {
  if (!value || typeof value !== 'object') return null;
  for (const [key, nested] of Object.entries(value)) {
    if (FORBIDDEN.has(key)) return `${path}.${key}`;
    const found = forbiddenField(nested, `${path}.${key}`);
    if (found) return found;
  }
  return null;
}

export async function createRunReceipt({
  runId,
  resultPath,
  seatEvidencePath,
  destination,
  now = new Date(),
}) {
  const raw = await readFile(resultPath, 'utf8');
  const result = parseStrictJson(raw, 'external-input result');
  if (result.runId !== runId || !['external_input_vusb', 'external_input_dom_vusb'].includes(result.profileId)) {
    throw new TypeError('virtual USB result is not bound to this run/mode');
  }
  if (!['PASS', 'RESIDUAL'].includes(result.status) ||
      !['ALLOW', 'CHALLENGE', 'DENY'].includes(result.measurement?.verdict)) {
    throw new TypeError('virtual USB run did not produce a successful bounded measurement');
  }
  if (result.control?.inputBackend !== 'usb-hid-emulated' ||
      result.purity?.xtestEnabled !== false || result.purity?.usbAssigned !== true ||
      result.purity?.uinputPresent !== false) {
    throw new TypeError('virtual USB run purity evidence is incomplete');
  }
  const { required, ...virtualAttestation } = result.usb || {};
  if (required !== true || Object.hasOwn(virtualAttestation, 'attested')) {
    throw new TypeError('virtual USB must use the emulation-specific attestation namespace');
  }
  assertVirtualUsbAttestation(virtualAttestation);
  if (virtualAttestation.runId !== runId) {
    throw new TypeError('virtual USB attestation is not bound to this run');
  }
  const { raw: seatRaw } = await readSeatEvidence(seatEvidencePath, runId);
  const oracle = forbiddenField(result);
  if (oracle) throw new TypeError(`detector oracle field is forbidden: ${oracle}`);
  const receipt = {
    ...receiptBase('run', runId, now),
    profileId: result.profileId,
    browserEngine: result.browser?.engine,
    status: result.status,
    measurementVerdict: result.measurement.verdict,
    resultSha256: `sha256:${sha256(raw)}`,
    seatEvidenceSha256: `sha256:${sha256(seatRaw)}`,
    usbOrigin: 'kernel-emulated',
    physicalUsb: false,
  };
  await atomicJson(destination, receipt);
  return receipt;
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  createRunReceipt({
    runId: required('HM_VUSB_RUN_ID'),
    resultPath: required('HM_VUSB_RESULT_PATH'),
    seatEvidencePath: required('HM_VUSB_SEAT_EVIDENCE'),
    destination: required('HM_VUSB_RUN_RECEIPT'),
  }).catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-run-receipt',
      code: 'PURITY_FAIL',
      message: error.message,
    })}\n`);
    process.exitCode = 1;
  });
}
