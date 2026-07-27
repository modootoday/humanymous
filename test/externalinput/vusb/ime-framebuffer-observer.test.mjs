import assert from 'node:assert/strict';
import test from 'node:test';
import { observeImeFrames } from './ime-framebuffer-observer.mjs';

function frame(rgb, pixels = 96) {
  const data = Buffer.alloc(pixels * 4);
  for (let offset = 0; offset < data.length; offset += 4) {
    data[offset] = rgb[2];
    data[offset + 1] = rgb[1];
    data[offset + 2] = rgb[0];
    data[offset + 3] = 255;
  }
  return { pixels: data };
}

test('independent observer requires preedit before commit and hashes both frames', async () => {
  const frames = [
    frame([52, 211, 153]),
    frame([246, 194, 62]),
    frame([52, 211, 153]),
  ];
  let tick = 0;
  const observation = await observeImeFrames({
    capture: async () => frames.shift(),
    delay: async () => {},
    now: () => tick++,
    timeoutMs: 10,
  });
  assert.match(observation.preeditFramebufferSha256, /^[a-f0-9]{64}$/);
  assert.match(observation.commitFramebufferSha256, /^[a-f0-9]{64}$/);
  assert.notEqual(
    observation.preeditFramebufferSha256,
    observation.commitFramebufferSha256,
  );
});

test('observer fails closed when commit never follows preedit', async () => {
  let tick = 0;
  await assert.rejects(observeImeFrames({
    capture: async () => frame([246, 194, 62]),
    delay: async () => {},
    now: () => tick++,
    timeoutMs: 4,
  }), /commit framebuffer cue/);
});
