import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createActionFirewall } from '../firewall.mjs';
import { createEmulatedUsbInputAdapter } from '../input.mjs';
import { locateTokenMarkers, validatePixelManifest } from '../pixel-locator.mjs';
import { connectRfb } from '../rfb-client.mjs';
import { sendUsbBrokerCommand } from '../usb-client.mjs';
import { IME_AXIS_VERSION, validateImeResult, vectorFor } from './ime-contract.mjs';

const sleep = (ms) => new Promise((resolveDelay) => setTimeout(resolveDelay, ms));

async function main(env = process.env) {
  const locale = env.HM_EXTERNAL_IME_LOCALE;
  const browser = env.HM_EXTERNAL_BROWSER;
  const runId = env.HM_EXTERNAL_RUN_ID;
  const vector = vectorFor(locale);
  const manifest = validatePixelManifest(JSON.parse(
    await readFile('/app/test/externalinput/visual-manifest.json', 'utf8'),
  ));
  const attestation = JSON.parse(
    await readFile(env.HM_EXTERNAL_USB_ATTESTATION, 'utf8'),
  );
  const rfb = await connectRfb({
    host: env.HM_EXTERNAL_RFB_HOST || 'external-display',
    port: Number(env.HM_EXTERNAL_RFB_PORT || 5900),
    passwordFile: env.HM_EXTERNAL_RFB_PASSWORD_FILE,
    allowInput: false,
  });
  const send = (action) => sendUsbBrokerCommand(env.HM_EXTERNAL_USB_COMMAND_PATH, action);
  const input = createEmulatedUsbInputAdapter({
    firewall: createActionFirewall({ syntheticTexts: [] }),
    usbAttestation: attestation,
    rfbInputEnabled: false,
    xtestEnabled: false,
    send,
    release: send,
  });
  let result;
  try {
    let target;
    const targetDeadline = Date.now() + 30_000;
    do {
      const frame = await rfb.capture();
      [target] = locateTokenMarkers(frame, manifest, ['ime-input']);
      if (target || Date.now() >= targetDeadline) break;
      await sleep(150);
    } while (true);
    if (!target) throw new Error('IME input marker did not become visible');
    const x = Math.round(target.rect.x + target.rect.width / 2);
    const y = Math.round(target.rect.y + target.rect.height / 2);
    await input.perform({ kind: 'pointerMove', x, y, durationMs: 180 });
    await input.perform({ kind: 'pointerClick', button: 'left', dwellMs: 70 });

    for (const key of vector.usages) {
      await input.perform({ kind: 'keyStroke', key, modifiers: [], dwellMs: 62, flightMs: 88 });
    }
    for (const key of vector.commit) {
      await input.perform({ kind: 'keyStroke', key, modifiers: [], dwellMs: 68, flightMs: 96 });
    }
    result = validateImeResult({
      axisVersion: IME_AXIS_VERSION,
      runId,
      browser,
      locale,
      vectorId: vector.vectorId,
      inputBackend: 'usb-hid-emulated',
      actorRole: 'input-only',
      hidUsageCount: vector.usages.length + vector.commit.length,
      status: 'ACTED',
    });
  } finally {
    await input.releaseAll();
    rfb.close();
  }
  const root = resolve(env.HM_EXTERNAL_ARTIFACT_ROOT || '/artifacts/external-input');
  await mkdir(root, { recursive: true });
  await writeFile(resolve(root, 'ime.result.json'), `${JSON.stringify(result, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(result)}\n`);
  if (result.status !== 'ACTED') process.exitCode = 1;
}

main().catch((error) => {
  process.stderr.write(`IME runner failed: ${error.message || error}\n`);
  process.exitCode = 1;
});
