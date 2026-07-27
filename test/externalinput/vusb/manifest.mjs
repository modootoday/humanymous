import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { IME_CELLS, VIRTUAL_USB_CONTROL_CELLS } from '../matrix.mjs';
import {
  atomicJson,
  canonicalJson,
  exactObject,
  MODEL_ID,
  RUN_ID,
  SHA256,
  sha256,
} from './common.mjs';
import { RUNTIME_IMAGE_FIELDS } from './catalog.mjs';
import { parseStrictJson } from './strict-json.mjs';

export const LADDER_MANIFEST_VERSION =
  'humanymous.virtual-usb-ladder-manifest/v1';
export const ATTEMPT_MANIFEST_VERSION =
  'humanymous.virtual-usb-attempt-manifest/v1';

const LADDER_FIELDS = Object.freeze([
  'schemaVersion', 'ladderId', 'modelId', 'catalogSha256', 'runtimeImages',
  'matrixSha256', 'controlCells', 'imeCells',
]);
const ATTEMPT_FIELDS = Object.freeze([
  'schemaVersion', 'ladderId', 'ladderManifestSha256', 'runId', 'modelId',
  'axis', 'browser', 'sequence', 'profileId', 'observation', 'inputBackend',
  'domRequired', 'usbOrigin', 'locale', 'selectedBrowserImageDigest',
  'childProject', 'parentProject',
]);
const PROJECT_ID = /^[a-z0-9][a-z0-9-]{0,62}$/;

function canonicalCells() {
  return {
    controlCells: structuredClone(VIRTUAL_USB_CONTROL_CELLS),
    imeCells: structuredClone(IME_CELLS),
  };
}

export function canonicalMatrixSha256() {
  return `sha256:${sha256(canonicalJson(canonicalCells()))}`;
}

function exactRuntimeImages(runtimeImages) {
  exactObject(runtimeImages, RUNTIME_IMAGE_FIELDS, 'ladder runtime images');
  for (const [name, digest] of Object.entries(runtimeImages)) {
    if (!SHA256.test(digest || '')) {
      throw new TypeError(`ladder runtime image ${name} is invalid`);
    }
  }
  return runtimeImages;
}

export function validateLadderManifest(value) {
  exactObject(value, LADDER_FIELDS, 'ladder manifest');
  if (value.schemaVersion !== LADDER_MANIFEST_VERSION ||
      !RUN_ID.test(value.ladderId || '') ||
      !MODEL_ID.test(value.modelId || '') ||
      !SHA256.test(value.catalogSha256 || '')) {
    throw new TypeError('ladder manifest identity is invalid');
  }
  exactRuntimeImages(value.runtimeImages);
  const cells = canonicalCells();
  if (canonicalJson(value.controlCells) !== canonicalJson(cells.controlCells) ||
      canonicalJson(value.imeCells) !== canonicalJson(cells.imeCells) ||
      value.matrixSha256 !== canonicalMatrixSha256()) {
    throw new TypeError('ladder manifest matrix is not canonical');
  }
  return Object.freeze(structuredClone(value));
}

export function createLadderManifest({
  ladderId,
  modelId,
  catalogSha256,
  runtimeImages,
}) {
  const cells = canonicalCells();
  return validateLadderManifest({
    schemaVersion: LADDER_MANIFEST_VERSION,
    ladderId,
    modelId,
    catalogSha256,
    runtimeImages: structuredClone(runtimeImages),
    matrixSha256: canonicalMatrixSha256(),
    ...cells,
  });
}

function canonicalCell(axis, { browser, sequence, profileId, locale }) {
  const cells = axis === 'control'
    ? VIRTUAL_USB_CONTROL_CELLS
    : axis === 'ime'
      ? IME_CELLS
      : null;
  if (!cells) throw new TypeError('attempt axis must be control or ime');
  const matches = cells.filter((cell) =>
    cell.browser === browser &&
    cell.sequence === sequence &&
    cell.profileId === profileId &&
    (axis === 'control' ? locale === '' : cell.locale === locale));
  if (matches.length !== 1) throw new TypeError('attempt is not a canonical matrix cell');
  return matches[0];
}

export function validateAttemptManifest(value, ladder) {
  exactObject(value, ATTEMPT_FIELDS, 'attempt manifest');
  const validatedLadder = validateLadderManifest(ladder);
  if (value.schemaVersion !== ATTEMPT_MANIFEST_VERSION ||
      value.ladderId !== validatedLadder.ladderId ||
      value.modelId !== validatedLadder.modelId ||
      !SHA256.test(value.ladderManifestSha256 || '') ||
      !RUN_ID.test(value.runId || '') ||
      !PROJECT_ID.test(value.childProject || '') ||
      (value.parentProject !== '' && !PROJECT_ID.test(value.parentProject || ''))) {
    throw new TypeError('attempt manifest identity is invalid');
  }
  const cell = canonicalCell(value.axis, value);
  for (const field of [
    'browser', 'sequence', 'profileId', 'observation', 'inputBackend', 'usbOrigin',
  ]) {
    if (value[field] !== cell[field]) {
      throw new TypeError(`attempt manifest ${field} is not canonical`);
    }
  }
  if (value.domRequired !== Boolean(cell.domRequired) ||
      value.locale !== (cell.locale || '')) {
    throw new TypeError('attempt manifest DOM or locale identity is not canonical');
  }
  const browserImageKey = value.axis === 'ime'
    ? value.browser === 'chromium' ? 'browserChromiumIme' : 'browserFirefoxIme'
    : value.browser === 'chromium'
      ? value.domRequired ? 'browserChromiumDom' : 'browserChromium'
      : value.domRequired ? 'browserFirefoxDom' : 'browserFirefox';
  if (value.selectedBrowserImageDigest !==
      validatedLadder.runtimeImages[browserImageKey]) {
    throw new TypeError('attempt manifest selected browser image is not canonical');
  }
  const expectsParent = value.axis === 'ime' || value.sequence >= 3;
  if (expectsParent !== (value.parentProject !== '')) {
    throw new TypeError('attempt manifest parent project does not match its backend');
  }
  return Object.freeze(structuredClone(value));
}

export function createAttemptManifest({
  ladder,
  ladderManifestSha256,
  runId,
  axis,
  browser,
  sequence,
  profileId,
  locale = '',
  childProject,
  parentProject = '',
}) {
  const validatedLadder = validateLadderManifest(ladder);
  const cell = canonicalCell(axis, { browser, sequence, profileId, locale });
  return validateAttemptManifest({
    schemaVersion: ATTEMPT_MANIFEST_VERSION,
    ladderId: validatedLadder.ladderId,
    ladderManifestSha256,
    runId,
    modelId: validatedLadder.modelId,
    axis,
    browser: cell.browser,
    sequence: cell.sequence,
    profileId: cell.profileId,
    observation: cell.observation,
    inputBackend: cell.inputBackend,
    domRequired: Boolean(cell.domRequired),
    usbOrigin: cell.usbOrigin,
    locale: cell.locale || '',
    selectedBrowserImageDigest: validatedLadder.runtimeImages[
      axis === 'ime'
        ? cell.browser === 'chromium' ? 'browserChromiumIme' : 'browserFirefoxIme'
        : cell.browser === 'chromium'
          ? cell.domRequired ? 'browserChromiumDom' : 'browserChromium'
          : cell.domRequired ? 'browserFirefoxDom' : 'browserFirefox'
    ],
    childProject,
    parentProject,
  }, validatedLadder);
}

export async function loadLadderManifest(path) {
  const raw = await readFile(path, 'utf8');
  return Object.freeze({
    value: validateLadderManifest(parseStrictJson(raw, 'ladder manifest')),
    sha256: `sha256:${sha256(raw)}`,
  });
}

export async function writeLadderManifest(options) {
  const value = createLadderManifest(options);
  await atomicJson(options.destination, value);
  return value;
}

export async function writeAttemptManifest(options) {
  const ladder = await loadLadderManifest(options.ladderPath);
  const value = createAttemptManifest({
    ...options,
    ladder: ladder.value,
    ladderManifestSha256: ladder.sha256,
  });
  await atomicJson(options.destination, value);
  return value;
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

function runtimeImagesFromEnvironment() {
  const names = {
    labCore: 'HM_VUSB_LAB_CORE_IMAGE_ID',
    pki: 'HM_VUSB_PKI_IMAGE_ID',
    display: 'HM_VUSB_DISPLAY_IMAGE_ID',
    browserChromium: 'HM_VUSB_BROWSER_CHROMIUM_IMAGE_ID',
    browserChromiumDom: 'HM_VUSB_BROWSER_CHROMIUM_DOM_IMAGE_ID',
    browserChromiumIme: 'HM_VUSB_BROWSER_CHROMIUM_IME_IMAGE_ID',
    browserFirefox: 'HM_VUSB_BROWSER_FIREFOX_IMAGE_ID',
    browserFirefoxDom: 'HM_VUSB_BROWSER_FIREFOX_DOM_IMAGE_ID',
    browserFirefoxIme: 'HM_VUSB_BROWSER_FIREFOX_IME_IMAGE_ID',
    controller: 'HM_VUSB_CONTROLLER_IMAGE_ID',
    lifecycle: 'HM_VUSB_LIFECYCLE_IMAGE_ID',
    gateway: 'HM_VUSB_GATEWAY_IMAGE_ID',
    profile: 'HM_VUSB_PROFILE_IMAGE_ID',
  };
  return Object.fromEntries(
    Object.entries(names).map(([name, environment]) => [name, required(environment)]),
  );
}

async function main() {
  const action = process.argv[2];
  if (action === 'ladder') {
    await writeLadderManifest({
      ladderId: required('HM_VUSB_LADDER_ID'),
      modelId: required('HM_VUSB_MODEL_ID'),
      catalogSha256: required('HM_VUSB_CATALOG_SHA256'),
      runtimeImages: runtimeImagesFromEnvironment(),
      destination: required('HM_VUSB_LADDER_MANIFEST'),
    });
    return;
  }
  if (action === 'attempt') {
    await writeAttemptManifest({
      ladderPath: required('HM_VUSB_LADDER_MANIFEST'),
      destination: required('HM_VUSB_ATTEMPT_MANIFEST'),
      runId: required('HM_VUSB_RUN_ID'),
      axis: required('HM_VUSB_AXIS'),
      browser: required('HM_EXTERNAL_BROWSER'),
      sequence: Number.parseInt(required('HM_VUSB_SEQUENCE'), 10),
      profileId: required('HM_EXTERNAL_MODE'),
      locale: process.env.HM_EXTERNAL_IME_LOCALE || '',
      childProject: required('HM_VUSB_CHILD_PROJECT'),
      parentProject: process.env.HM_VUSB_PARENT_PROJECT || '',
    });
    return;
  }
  throw new TypeError('manifest action must be ladder or attempt');
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-manifest',
      code: 'CONFIG_ERROR',
      message: error.message,
    })}\n`);
    process.exitCode = 2;
  });
}
