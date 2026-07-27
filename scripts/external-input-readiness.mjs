#!/usr/bin/env node

import { readFile } from 'node:fs/promises';
import { connectRfb } from '../test/externalinput/rfb-client.mjs';
import {
  locateTokenMarkers,
  validatePixelManifest,
} from '../test/externalinput/pixel-locator.mjs';

const timeoutMs = Number(process.env.HM_EXTERNAL_READY_TIMEOUT_MS || 30_000);
const deadline = Date.now() + timeoutMs;
const manifest = validatePixelManifest(JSON.parse(
  await readFile(process.env.HM_EXTERNAL_VISUAL_MANIFEST, 'utf8'),
));
const rfb = await connectRfb({
  host: process.env.HM_EXTERNAL_RFB_HOST,
  port: Number(process.env.HM_EXTERNAL_RFB_PORT || 5900),
  passwordFile: process.env.HM_EXTERNAL_RFB_PASSWORD_FILE,
  allowInput: false,
});

function colorDiagnostics(frame, expectedRgb) {
  const colors = new Map();
  let expectedPixels = 0;
  for (let offset = 0; offset < frame.pixels.length; offset += 4) {
    const b = frame.pixels[offset];
    const g = frame.pixels[offset + 1];
    const r = frame.pixels[offset + 2];
    if (r === expectedRgb[0] && g === expectedRgb[1] && b === expectedRgb[2]) {
      expectedPixels += 1;
    }
    const key = `${r},${g},${b}`;
    colors.set(key, (colors.get(key) || 0) + 1);
  }
  return {
    expectedPixels,
    topColors: [...colors.entries()]
      .sort((left, right) => right[1] - left[1])
      .slice(0, 8),
  };
}

try {
  let lastFrame;
  while (Date.now() < deadline) {
    lastFrame = await rfb.capture();
    const targets = locateTokenMarkers(lastFrame, manifest, ['choice-correct']);
    if (targets.length === 1) {
      process.stdout.write(`${JSON.stringify({
        ready: true,
        protocol: rfb.protocol,
        width: lastFrame.width,
        height: lastFrame.height,
        target: targets[0].rect,
      })}\n`);
      process.exitCode = 0;
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  if (process.exitCode === undefined) {
    const diagnostics = lastFrame
      ? colorDiagnostics(lastFrame, manifest.tokens['choice-correct'].rgb)
      : {};
    throw new Error(
      `initial visual target did not appear within ${timeoutMs}ms` +
      (lastFrame ? ` (${lastFrame.width}x${lastFrame.height}; ${JSON.stringify(diagnostics)})` : ''),
    );
  }
} finally {
  rfb.close();
}
