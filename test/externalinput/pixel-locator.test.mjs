import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import {
  classifyFramebufferVerdict,
  locateTokenMarkers,
  validatePixelManifest,
} from './pixel-locator.mjs';
import { runStrategicPolicy } from './strategy.mjs';

const manifestUrl = new URL('./visual-manifest.json', import.meta.url);

function frame(width = 640, height = 300) {
  return { width, height, pixels: new Uint8Array(width * height * 4) };
}

function setPixel(target, x, y, rgb) {
  const offset = (y * target.width + x) * 4;
  target.pixels[offset] = rgb[2];
  target.pixels[offset + 1] = rgb[1];
  target.pixels[offset + 2] = rgb[0];
  target.pixels[offset + 3] = 0;
}

function fillRect(target, rect, rgb) {
  for (let y = rect.y; y < rect.y + rect.height; y += 1) {
    for (let x = rect.x; x < rect.x + rect.width; x += 1) setPixel(target, x, y, rgb);
  }
}

function strokeRect(target, rect, thickness, rgb) {
  fillRect(target, { x: rect.x, y: rect.y, width: rect.width, height: thickness }, rgb);
  fillRect(target, {
    x: rect.x,
    y: rect.y + rect.height - thickness,
    width: rect.width,
    height: thickness,
  }, rgb);
  fillRect(target, { x: rect.x, y: rect.y, width: thickness, height: rect.height }, rgb);
  fillRect(target, {
    x: rect.x + rect.width - thickness,
    y: rect.y,
    width: thickness,
    height: rect.height,
  }, rgb);
}

async function manifest() {
  return validatePixelManifest(JSON.parse(await readFile(manifestUrl, 'utf8')));
}

test('BGRX marker scan returns a bounded component rectangle without a frame hash', async () => {
  const rules = await manifest();
  const pixels = frame();
  const markerRect = { x: 111, y: 72, width: 91, height: 44 };
  fillRect(pixels, markerRect, rules.tokens['choice-correct'].rgb);
  const targets = locateTokenMarkers(pixels, rules, ['choice-correct']);
  assert.equal(targets.length, 1);
  assert.deepEqual(targets[0], {
    token: 'choice-correct',
    rect: markerRect,
    enabled: true,
    visible: true,
    source: 'framebuffer',
  });

  // An unrelated pixel changes the framebuffer bytes but not visual localization.
  setPixel(pixels, 1, 1, [12, 34, 56]);
  assert.deepEqual(
    locateTokenMarkers(pixels, rules, ['choice-correct'])[0].rect,
    markerRect,
  );
});

test('token lookup never guesses when the marker component is absent or too small', async () => {
  const rules = await manifest();
  const pixels = frame();
  fillRect(pixels, { x: 1, y: 1, width: 4, height: 4 }, rules.tokens['nav-branch'].rgb);
  assert.deepEqual(locateTokenMarkers(pixels, rules, ['nav-branch']), []);
});

test('verdict classifier requires a wide frame cue and a distinct icon-shape cue', async () => {
  const rules = await manifest();
  const pixels = frame();
  const color = rules.verdicts.CHALLENGE.rgb;
  strokeRect(pixels, { x: 20, y: 20, width: 600, height: 180 }, 4, color);
  assert.deepEqual(classifyFramebufferVerdict(pixels, rules), {
    verdict: 'NO_RESPONSE',
    confidence: 0,
    cueCount: 0,
  });
  strokeRect(pixels, { x: 64, y: 78, width: 72, height: 72 }, 5, color);
  assert.deepEqual(classifyFramebufferVerdict(pixels, rules), {
    verdict: 'CHALLENGE',
    confidence: 1,
    cueCount: 2,
  });
});

test('ambiguous framebuffer with two complete coarse verdicts fails closed', async () => {
  const rules = await manifest();
  const pixels = frame(1280, 600);
  const allow = rules.verdicts.ALLOW.rgb;
  const deny = rules.verdicts.DENY.rgb;
  strokeRect(pixels, { x: 20, y: 20, width: 600, height: 180 }, 4, allow);
  strokeRect(pixels, { x: 50, y: 70, width: 72, height: 72 }, 5, allow);
  strokeRect(pixels, { x: 660, y: 300, width: 600, height: 180 }, 4, deny);
  strokeRect(pixels, { x: 700, y: 350, width: 72, height: 72 }, 5, deny);
  assert.equal(classifyFramebufferVerdict(pixels, rules).verdict, 'NO_RESPONSE');
});

test('manifest rejects duplicate marker colors and unknown schema fields', async () => {
  const raw = JSON.parse(await readFile(manifestUrl, 'utf8'));
  raw.tokens['nav-return'].rgb = raw.tokens['nav-branch'].rgb;
  assert.throws(() => validatePixelManifest(raw), /colors must be unique/);
  const clean = JSON.parse(await readFile(manifestUrl, 'utf8'));
  clean.frames = {};
  assert.throws(() => validatePixelManifest(clean), /unknown field: frames/);
});

test('fixture marker CSS exactly matches the supervisor RGB manifest', async () => {
  const rules = await manifest();
  const css = await readFile(new URL('./fixture.css', import.meta.url), 'utf8');
  for (const [token, rule] of Object.entries(rules.tokens)) {
    const selector = `[data-hmn-token="${token}"]`;
    const start = css.indexOf(selector);
    assert.notEqual(start, -1, `missing CSS marker for ${token}`);
    const block = css.slice(start, css.indexOf('}', start));
    assert.match(block, new RegExp(`background:\\s*rgb\\(${rule.rgb.join(' ')}\\)`));
  }
  for (const [verdict, rule] of Object.entries(rules.verdicts)) {
    const hex = `#${rule.rgb.map((channel) => channel.toString(16).padStart(2, '0')).join('')}`;
    assert.match(css, new RegExp(`--${verdict.toLowerCase()}:\\s*${hex}`, 'i'));
  }
});

test('strategy prefers a DOM rectangle only when the DOM adapter actually returned one', async () => {
  async function finalPointer(targets) {
    const actions = [];
    await runStrategicPolicy({
      seed: 'dom-preference',
      tasks: [{ id: 'read-select', targetTokens: ['choice-correct'] }],
      observation: {
        observe: async () => ({
          targets,
          domQueries: targets.some((target) => target.source === 'dom') ? 1 : 0,
        }),
      },
      input: {
        perform: async (action) => actions.push(action),
      },
    });
    const moves = actions.filter((action) => action.kind === 'pointerMove');
    for (const move of moves) {
      assert.ok(move.x >= 0 && move.x < 1280);
      assert.ok(move.y >= 0 && move.y < 720);
    }
    return moves.at(-1);
  }
  const visual = {
    token: 'choice-correct',
    rect: { x: 10, y: 10, width: 20, height: 20 },
    enabled: true,
    visible: true,
    source: 'framebuffer',
  };
  const dom = {
    ...visual,
    rect: { x: 100, y: 10, width: 20, height: 20 },
    source: 'dom',
  };
  const visualPoint = await finalPointer([visual]);
  assert.ok(visualPoint.x >= 10 && visualPoint.x < 30);
  assert.ok(visualPoint.y >= 10 && visualPoint.y < 30);
  const domPoint = await finalPointer([visual, dom]);
  assert.ok(domPoint.x >= 100 && domPoint.x < 120);
  assert.ok(domPoint.y >= 10 && domPoint.y < 30);
});

test('strategy re-observes a visual transition without guessing a target', async () => {
  let observations = 0;
  const actions = [];
  const strategy = await runStrategicPolicy({
    seed: 'visual-transition',
    tasks: [{ id: 'read-select', targetTokens: ['choice-correct'] }],
    observationTimeoutMs: 1_000,
    sleep: async () => {},
    observation: {
      observe: async () => {
        observations += 1;
        return {
          targets: observations === 1 ? [] : [{
            token: 'choice-correct',
            rect: { x: 30, y: 20, width: 40, height: 20 },
            enabled: true,
            visible: true,
            source: 'framebuffer',
          }],
          domQueries: 0,
        };
      },
    },
    input: { perform: async (action) => actions.push(action) },
  });

  assert.equal(strategy.completed, true);
  assert.equal(strategy.decisions, 1);
  assert.equal(strategy.frames, 2);
  const moves = actions.filter((action) => action.kind === 'pointerMove');
  assert.ok(moves.length >= 4);
  for (const move of moves) {
    assert.ok(move.x >= 0 && move.x < 1280);
    assert.ok(move.y >= 0 && move.y < 720);
  }
  assert.ok(moves.at(-1).x >= 30 && moves.at(-1).x < 70);
  assert.ok(moves.at(-1).y >= 20 && moves.at(-1).y < 40);
});

test('strategy scrolls in bounded steps and re-orients before activating an offscreen target', async () => {
  const actions = [];
  const target = (token, y = 20) => ({
    token,
    rect: { x: 30, y, width: 40, height: 20 },
    enabled: true,
    visible: true,
    source: 'framebuffer',
  });
  const strategy = await runStrategicPolicy({
    seed: 'bounded-scroll',
    tasks: [{
      id: 'multi-step-navigation',
      targetTokens: ['nav-branch', 'nav-return'],
    }],
    transitionProbeMs: 0,
    sleep: async () => {},
    observation: {
      observe: async ({ targetTokens }) => ({
        targets: targetTokens.includes('nav-branch')
          ? [target('nav-branch')]
          : actions.some((action) => action.kind === 'scroll')
            ? [target('nav-return', 620)]
            : [],
        domQueries: 0,
      }),
    },
    input: { perform: async (action) => actions.push(action) },
  });

  assert.equal(strategy.completed, true);
  assert.equal(actions.filter((action) => action.kind === 'scroll').length, 1);
  assert.equal(actions.at(-1).kind, 'pointerClick');
});
