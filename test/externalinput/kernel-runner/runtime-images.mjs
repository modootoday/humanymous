import { SHA256 } from '../vusb/common.mjs';
import { RUNTIME_IMAGE_FIELDS } from '../vusb/catalog.mjs';

const COMMON_CELL_IMAGES = Object.freeze([
  'labCore',
  'pki',
  'display',
  'controller',
  'lifecycle',
  'gateway',
  'profile',
]);

export function cellRuntimeImageKeys(browser, sequence) {
  if (!['chromium', 'firefox'].includes(browser) ||
      ![3, 4].includes(sequence)) {
    throw new TypeError('kernel cell browser or sequence is invalid');
  }
  const browserKey = `browser${
    browser[0].toUpperCase()
  }${browser.slice(1)}${sequence === 4 ? 'Dom' : ''}`;
  return Object.freeze([
    ...COMMON_CELL_IMAGES.slice(0, 3),
    browserKey,
    ...COMMON_CELL_IMAGES.slice(3),
  ]);
}

export function exactRuntimeImageArguments(runtimeImages, imageKeys) {
  const selected = imageKeys || RUNTIME_IMAGE_FIELDS;
  if (!Array.isArray(selected) || selected.length < 1 ||
      selected.length > RUNTIME_IMAGE_FIELDS.length ||
      new Set(selected).size !== selected.length ||
      selected.some((field) => !RUNTIME_IMAGE_FIELDS.includes(field))) {
    throw new TypeError('kernel image archive keys are invalid');
  }
  if (!runtimeImages || typeof runtimeImages !== 'object' ||
      Array.isArray(runtimeImages) ||
      Object.keys(runtimeImages).some(
        (field) => !RUNTIME_IMAGE_FIELDS.includes(field),
      ) ||
      selected.some((field) => !Object.hasOwn(runtimeImages, field))) {
    throw new TypeError('kernel runtime images are invalid');
  }
  const seen = new Set();
  const values = [];
  for (const field of RUNTIME_IMAGE_FIELDS) {
    if (!selected.includes(field)) continue;
    const digest = runtimeImages[field];
    if (!SHA256.test(digest || '')) {
      throw new TypeError(`kernel runtime image ${field} is invalid`);
    }
    if (!seen.has(digest)) {
      seen.add(digest);
      values.push(digest);
    }
  }
  return Object.freeze(values);
}
