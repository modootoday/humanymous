import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { atomicJson, receiptBase } from './common.mjs';
import { connectRfb } from '../rfb-client.mjs';
import { IME_ENGINES, vectorFor } from './ime-contract.mjs';

const CUES = Object.freeze({
  preedit: Object.freeze([246, 194, 62]),
  commit: Object.freeze([52, 211, 153]),
});

const sleep = (ms) => new Promise((resolveDelay) => setTimeout(resolveDelay, ms));

function colorCount(frame, rgb) {
  let count = 0;
  for (let offset = 0; offset < frame.pixels.length; offset += 4) {
    if (frame.pixels[offset] === rgb[2] &&
        frame.pixels[offset + 1] === rgb[1] &&
        frame.pixels[offset + 2] === rgb[0]) count += 1;
  }
  return count;
}

function hashFrame(frame) {
  return createHash('sha256').update(frame.pixels).digest('hex');
}

export async function observeImeFrames({
  capture,
  delay = sleep,
  now = () => Date.now(),
  timeoutMs = 30_000,
  minimumPixels = 96,
}) {
  const deadline = now() + timeoutMs;
  let preedit;
  while (now() <= deadline) {
    const frame = await capture();
    const preeditPixels = colorCount(frame, CUES.preedit);
    const commitPixels = colorCount(frame, CUES.commit);
    if (!preedit && preeditPixels >= minimumPixels) {
      preedit = {
        sha256: hashFrame(frame),
        pixels: preeditPixels,
      };
    } else if (preedit && commitPixels >= minimumPixels) {
      return Object.freeze({
        preeditFramebufferSha256: preedit.sha256,
        commitFramebufferSha256: hashFrame(frame),
        preeditCuePixels: preedit.pixels,
        commitCuePixels: commitPixels,
      });
    }
    await delay(120);
  }
  throw new Error(preedit
    ? 'IME commit framebuffer cue was not observed'
    : 'IME preedit framebuffer cue was not observed');
}

export async function createImeFramebufferObservation({
  runId,
  browser,
  locale,
  destination,
  rfbHost = 'external-display',
  rfbPort = 5900,
  passwordFile,
  timeoutMs = 30_000,
  now = new Date(),
}) {
  if (!IME_ENGINES.includes(browser)) throw new TypeError('IME observer browser is invalid');
  const vector = vectorFor(locale);
  const rfb = await connectRfb({
    host: rfbHost,
    port: Number(rfbPort),
    passwordFile,
    allowInput: false,
  });
  try {
    const observation = await observeImeFrames({
      capture: () => rfb.capture(),
      timeoutMs,
    });
    const receipt = {
      ...receiptBase('ime-framebuffer-observation', runId, now),
      browser,
      locale,
      vectorId: vector.vectorId,
      observation: 'framebuffer',
      ...observation,
      status: 'OBSERVED',
    };
    await atomicJson(destination, receipt);
    return receipt;
  } finally {
    rfb.close();
  }
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  createImeFramebufferObservation({
    runId: required('HM_VUSB_RUN_ID'),
    browser: required('HM_EXTERNAL_BROWSER'),
    locale: required('HM_EXTERNAL_IME_LOCALE'),
    destination: required('HM_VUSB_IME_FRAMEBUFFER_OBSERVATION'),
    rfbHost: process.env.HM_EXTERNAL_RFB_HOST || 'external-display',
    rfbPort: Number(process.env.HM_EXTERNAL_RFB_PORT || 5900),
    passwordFile: required('HM_EXTERNAL_RFB_PASSWORD_FILE'),
    timeoutMs: Number(process.env.HM_VUSB_IME_OBSERVER_TIMEOUT_MS || 30_000),
  }).then((receipt) => {
    process.stdout.write(`${JSON.stringify(receipt)}\n`);
  }).catch((error) => {
    process.stderr.write(`${JSON.stringify({
      level: 'error',
      component: 'external-vusb-ime-framebuffer-observer',
      code: 'OBSERVATION_FAIL',
      message: error.message,
    })}\n`);
    process.exitCode = 1;
  });
}
