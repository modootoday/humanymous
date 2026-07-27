import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const root = new URL('./', import.meta.url);

test('Firefox ESR package pins one read-only laboratory extension and native host', async () => {
  const [manifest, nativeHost, policies, dockerfile] = await Promise.all([
    readFile(new URL('firefox-manifest.json', root), 'utf8').then(JSON.parse),
    readFile(new URL('firefox-native-host-manifest.json', root), 'utf8').then(JSON.parse),
    readFile(
      new URL('../../../deployments/external-input/firefox-policies.json', root),
      'utf8',
    ).then(JSON.parse),
    readFile(
      new URL('../../../build/external-input-browser.Dockerfile', root),
      'utf8',
    ),
  ]);
  const id = 'external-input-dom@humanymous.invalid';
  assert.equal(manifest.browser_specific_settings.gecko.id, id);
  assert.deepEqual(manifest.permissions, ['nativeMessaging']);
  assert.deepEqual(manifest.host_permissions, ['https://core/*']);
  assert.deepEqual(manifest.background, {
    scripts: ['service-worker.mjs'],
    type: 'module',
  });
  assert.deepEqual(nativeHost.allowed_extensions, [id]);
  assert.equal(
    policies.policies.Preferences['xpinstall.signatures.required'].Value,
    false,
  );
  assert.equal(policies.policies.ExtensionSettings['*'].installation_mode, 'blocked');
  assert.equal(policies.policies.ExtensionSettings[id].installation_mode, 'allowed');
  assert.match(
    dockerfile,
    /distribution\/extensions\/external-input-dom@humanymous\.invalid\.xpi/,
  );
  assert.match(dockerfile, /zip -X -q/);
});
