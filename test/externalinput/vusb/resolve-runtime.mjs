import { execFileSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { loadVerifiedCatalog, resolveModel } from './catalog.mjs';

const ENV_MAPPING = Object.freeze({
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
});

export class RuntimeUnavailableError extends Error {}

function inspectLocalDigest(digest, docker = process.env.DOCKER || 'docker') {
  let raw;
  try {
    raw = execFileSync(docker, ['image', 'inspect', digest, '--format', '{{json .}}'], {
      encoding: 'utf8',
      windowsHide: true,
    });
  } catch (error) {
    throw new RuntimeUnavailableError(`preloaded image is unavailable: ${digest}`, {
      cause: error,
    });
  }
  const inspected = JSON.parse(raw);
  if (inspected?.Descriptor?.digest !== digest && inspected?.Id !== digest) {
    throw new TypeError(`preloaded image identity mismatch: ${digest}`);
  }
}

export async function resolveRuntime({
  catalogDirectory,
  trustRootPath,
  modelId,
  outputPath,
  inspect = inspectLocalDigest,
}) {
  const verified = await loadVerifiedCatalog({
    catalogPath: resolve(catalogDirectory, 'profiles.lock.json'),
    signaturePath: resolve(catalogDirectory, 'profiles.lock.sig'),
    publicKeyPath: resolve(trustRootPath),
  });
  const model = resolveModel(verified.catalog, modelId);
  if (verified.catalog.runtimeImages.profile !== model.platformManifestDigest) {
    throw new TypeError('runtime profile image and selected model digest differ');
  }
  for (const digest of Object.values(verified.catalog.runtimeImages)) inspect(digest);
  const lines = [
    `HM_VUSB_MODEL_ID=${model.modelId}`,
    `HM_VUSB_CATALOG_SHA256=${verified.catalog.catalogSha256}`,
    ...Object.entries(ENV_MAPPING).map(
      ([key, environment]) => `${environment}=${verified.catalog.runtimeImages[key]}`,
    ),
  ];
  await writeFile(outputPath, `${lines.join('\n')}\n`, { encoding: 'utf8', mode: 0o600 });
  return Object.freeze({ model, runtimeImages: verified.catalog.runtimeImages });
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  resolveRuntime({
    catalogDirectory: required('HM_VUSB_CATALOG_DIR'),
    trustRootPath: required('HM_VUSB_CATALOG_TRUST_ROOT'),
    modelId: required('HM_VUSB_MODEL_ID'),
    outputPath: required('HM_VUSB_RUNTIME_ENV'),
  }).catch((error) => {
    process.stderr.write(`virtual USB runtime resolution failed: ${error.message}\n`);
    process.exitCode = error instanceof RuntimeUnavailableError ? 3 : 2;
  });
}
