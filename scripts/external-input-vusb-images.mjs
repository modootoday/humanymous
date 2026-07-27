import { execFileSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

export const TAGS = Object.freeze({
  labCore: 'humanymous/external-input-core:vusb-local',
  pki: 'humanymous/external-input-pki:vusb-local',
  display: 'humanymous/external-input-display:vusb-local',
  browserChromium: 'humanymous/external-input-browser-chromium:vusb-local',
  browserChromiumDom: 'humanymous/external-input-browser-chromium-dom:vusb-local',
  browserChromiumIme: 'humanymous/external-input-browser-chromium-ime:vusb-local',
  browserFirefox: 'humanymous/external-input-browser-firefox:vusb-local',
  browserFirefoxDom: 'humanymous/external-input-browser-firefox-dom:vusb-local',
  browserFirefoxIme: 'humanymous/external-input-browser-firefox-ime:vusb-local',
  controller: 'humanymous/external-input-controller:vusb-local',
  lifecycle: 'humanymous/external-input-vusb-lifecycle:vusb-local',
  gateway: 'humanymous/external-input-vusb-gateway:vusb-local',
  profile: 'humanymous/external-input-vusb-profile:reference-relative-v1',
});

export const RUNTIME_BUILD_PLAN = Object.freeze([
  ['build/external-input-core.Dockerfile', TAGS.labCore, ''],
  ['build/external-input-pki.Dockerfile', TAGS.pki, ''],
  ['build/external-input-browser.Dockerfile', TAGS.display, 'display'],
  ['build/external-input-browser.Dockerfile', TAGS.browserChromium, 'browser-chromium'],
  ['build/external-input-browser.Dockerfile', TAGS.browserChromiumDom, 'browser-chromium-dom'],
  ['build/external-input-browser.Dockerfile', TAGS.browserChromiumIme, 'browser-chromium-ime'],
  ['build/external-input-browser.Dockerfile', TAGS.browserFirefox, 'browser-firefox'],
  ['build/external-input-browser.Dockerfile', TAGS.browserFirefoxDom, 'browser-firefox-dom'],
  ['build/external-input-browser.Dockerfile', TAGS.browserFirefoxIme, 'browser-firefox-ime'],
  ['build/external-input-controller.Dockerfile', TAGS.controller, ''],
  ['build/external-input-vusb-lifecycle.Dockerfile', TAGS.lifecycle, ''],
  ['build/external-input-vusb-gateway.Dockerfile', TAGS.gateway, ''],
  ['build/external-input-vusb-profile.Dockerfile', TAGS.profile, ''],
].map((entry) => Object.freeze(entry)));

function selectedImageKeys(keys) {
  if (!Array.isArray(keys) || keys.length < 1 ||
      keys.length > Object.keys(TAGS).length ||
      new Set(keys).size !== keys.length ||
      keys.some((key) => !Object.hasOwn(TAGS, key))) {
    throw new TypeError('runtime image selection is invalid');
  }
  return keys;
}

export function selectedRuntimeBuildPlan(keys = Object.keys(TAGS)) {
  const selectedTags = new Set(
    selectedImageKeys(keys).map((key) => TAGS[key]),
  );
  return Object.freeze(
    RUNTIME_BUILD_PLAN.filter(([, tag]) => selectedTags.has(tag)),
  );
}

export function inspectRuntimeImages({
  docker = process.env.DOCKER || 'docker',
  keys = Object.keys(TAGS),
} = {}) {
  return Object.fromEntries(selectedImageKeys(keys).map((name) => {
    const tag = TAGS[name];
    const raw = execFileSync(
      docker,
      ['image', 'inspect', tag, '--format', '{{json .}}'],
      { encoding: 'utf8', windowsHide: true },
    );
    const image = JSON.parse(raw);
    const digest = image?.Descriptor?.digest;
    if (!/^sha256:[a-f0-9]{64}$/.test(digest || '')) {
      throw new TypeError(`image ${tag} has no immutable local descriptor digest`);
    }
    return [name, digest];
  }));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const output = process.env.HM_VUSB_RUNTIME_IMAGES_JSON;
  if (!output) throw new Error('HM_VUSB_RUNTIME_IMAGES_JSON is required');
  writeFile(output, `${JSON.stringify(inspectRuntimeImages(), null, 2)}\n`, {
    encoding: 'utf8',
    mode: 0o600,
  }).catch((error) => {
    process.stderr.write(`virtual USB image inspection failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
