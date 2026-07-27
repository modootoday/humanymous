import { TOKENS } from './dom-observer/extension/protocol.mjs';
import { ContractViolationError } from './errors.mjs';

const VERDICTS = Object.freeze(['ALLOW', 'CHALLENGE', 'DENY']);
const TOKEN_SET = new Set(TOKENS);
const MANIFEST_FIELDS = new Set(['schemaVersion', 'pixelFormat', 'maxFrame', 'tokens', 'verdicts']);
const TOKEN_RULE_FIELDS = new Set(['rgb', 'minPixels']);
const VERDICT_RULE_FIELDS = new Set([
  'rgb',
  'minComponentPixels',
  'frameMinWidth',
  'frameMinHeight',
  'iconMinSize',
  'iconMaxSize',
]);

function plainObject(value, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new ContractViolationError(`${label} must be an object`);
  }
  return value;
}

function exactFields(value, allowed, label) {
  for (const field of Object.keys(value)) {
    if (!allowed.has(field)) throw new ContractViolationError(`${label} contains unknown field: ${field}`);
  }
}

function rgb(value, label) {
  if (!Array.isArray(value) || value.length !== 3 ||
      value.some((channel) => !Number.isInteger(channel) || channel < 0 || channel > 255)) {
    throw new ContractViolationError(`${label} RGB must contain three bytes`);
  }
  return Object.freeze([...value]);
}

function positiveInteger(value, label, max = 10_000_000) {
  if (!Number.isInteger(value) || value <= 0 || value > max) {
    throw new ContractViolationError(`${label} must be a bounded positive integer`);
  }
  return value;
}

export function validatePixelManifest(value) {
  const manifest = plainObject(value, 'pixel manifest');
  exactFields(manifest, MANIFEST_FIELDS, 'pixel manifest');
  if (manifest.schemaVersion !== '1.0.0' || manifest.pixelFormat !== 'BGRX8888') {
    throw new ContractViolationError('pixel manifest version or format is unsupported');
  }
  const maxFrame = plainObject(manifest.maxFrame, 'maxFrame');
  exactFields(maxFrame, new Set(['width', 'height']), 'maxFrame');
  const normalized = {
    schemaVersion: manifest.schemaVersion,
    pixelFormat: manifest.pixelFormat,
    maxFrame: {
      width: positiveInteger(maxFrame.width, 'maxFrame.width', 4096),
      height: positiveInteger(maxFrame.height, 'maxFrame.height', 2160),
    },
    tokens: {},
    verdicts: {},
  };

  const tokenRules = plainObject(manifest.tokens, 'tokens');
  if (Object.keys(tokenRules).length !== TOKENS.length ||
      TOKENS.some((token) => !(token in tokenRules))) {
    throw new ContractViolationError('pixel manifest must define every canonical token exactly once');
  }
  const colors = new Set();
  for (const token of TOKENS) {
    const rule = plainObject(tokenRules[token], `token ${token}`);
    exactFields(rule, TOKEN_RULE_FIELDS, `token ${token}`);
    const color = rgb(rule.rgb, `token ${token}`);
    const colorKey = color.join(',');
    if (colors.has(colorKey)) throw new ContractViolationError('pixel marker colors must be unique');
    colors.add(colorKey);
    normalized.tokens[token] = Object.freeze({
      rgb: color,
      minPixels: positiveInteger(rule.minPixels, `token ${token} minPixels`, 1_000_000),
    });
  }

  const verdictRules = plainObject(manifest.verdicts, 'verdicts');
  if (Object.keys(verdictRules).length !== VERDICTS.length ||
      VERDICTS.some((verdict) => !(verdict in verdictRules))) {
    throw new ContractViolationError('pixel manifest must define all coarse verdicts');
  }
  for (const verdict of VERDICTS) {
    const rule = plainObject(verdictRules[verdict], `verdict ${verdict}`);
    exactFields(rule, VERDICT_RULE_FIELDS, `verdict ${verdict}`);
    const color = rgb(rule.rgb, `verdict ${verdict}`);
    const colorKey = color.join(',');
    if (colors.has(colorKey)) throw new ContractViolationError('verdict and marker colors must be unique');
    colors.add(colorKey);
    const iconMinSize = positiveInteger(rule.iconMinSize, `${verdict} iconMinSize`, 512);
    const iconMaxSize = positiveInteger(rule.iconMaxSize, `${verdict} iconMaxSize`, 512);
    if (iconMaxSize < iconMinSize) throw new ContractViolationError(`${verdict} icon bounds are reversed`);
    normalized.verdicts[verdict] = Object.freeze({
      rgb: color,
      minComponentPixels: positiveInteger(
        rule.minComponentPixels,
        `${verdict} minComponentPixels`,
        1_000_000,
      ),
      frameMinWidth: positiveInteger(rule.frameMinWidth, `${verdict} frameMinWidth`, 4096),
      frameMinHeight: positiveInteger(rule.frameMinHeight, `${verdict} frameMinHeight`, 2160),
      iconMinSize,
      iconMaxSize,
    });
  }
  return Object.freeze({
    ...normalized,
    maxFrame: Object.freeze(normalized.maxFrame),
    tokens: Object.freeze(normalized.tokens),
    verdicts: Object.freeze(normalized.verdicts),
  });
}

function validateFrame(frame, manifest) {
  if (!frame || !Number.isInteger(frame.width) || !Number.isInteger(frame.height) ||
      frame.width <= 0 || frame.height <= 0 ||
      frame.width > manifest.maxFrame.width || frame.height > manifest.maxFrame.height) {
    throw new ContractViolationError('frame geometry exceeds manifest bounds');
  }
  if (!(frame.pixels instanceof Uint8Array) ||
      frame.pixels.byteLength !== frame.width * frame.height * 4) {
    throw new ContractViolationError('Raw BGRX framebuffer byte length is invalid');
  }
}

function isColor(pixels, index, color) {
  const offset = index * 4;
  return pixels[offset] === color[2] &&
    pixels[offset + 1] === color[1] &&
    pixels[offset + 2] === color[0];
}

function findColorComponents(frame, color, { minPixels = 1, maxComponents = 64 } = {}) {
  const { width, height, pixels } = frame;
  const count = width * height;
  const visited = new Uint8Array(count);
  const queue = new Int32Array(count);
  const components = [];

  for (let start = 0; start < count; start += 1) {
    if (visited[start] || !isColor(pixels, start, color)) continue;
    let head = 0;
    let tail = 0;
    let componentPixels = 0;
    let minX = width;
    let minY = height;
    let maxX = -1;
    let maxY = -1;
    visited[start] = 1;
    queue[tail++] = start;

    while (head < tail) {
      const index = queue[head++];
      const x = index % width;
      const y = Math.floor(index / width);
      componentPixels += 1;
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
      const neighbors = [
        x > 0 ? index - 1 : -1,
        x + 1 < width ? index + 1 : -1,
        y > 0 ? index - width : -1,
        y + 1 < height ? index + width : -1,
      ];
      for (const neighbor of neighbors) {
        if (neighbor < 0 || visited[neighbor] || !isColor(pixels, neighbor, color)) continue;
        visited[neighbor] = 1;
        queue[tail++] = neighbor;
      }
    }

    if (componentPixels < minPixels) continue;
    const componentWidth = maxX - minX + 1;
    const componentHeight = maxY - minY + 1;
    components.push(Object.freeze({
      rect: Object.freeze({
        x: minX,
        y: minY,
        width: componentWidth,
        height: componentHeight,
      }),
      pixels: componentPixels,
      fillRatio: componentPixels / (componentWidth * componentHeight),
    }));
    if (components.length > maxComponents) {
      throw new ContractViolationError('color component count exceeds bound');
    }
  }
  return Object.freeze(components);
}

export function locateTokenMarkers(frame, manifest, targetTokens) {
  validateFrame(frame, manifest);
  if (!Array.isArray(targetTokens) || targetTokens.length > TOKENS.length ||
      new Set(targetTokens).size !== targetTokens.length ||
      targetTokens.some((token) => !TOKEN_SET.has(token))) {
    throw new ContractViolationError('visual target token is not canonical');
  }
  const targets = [];
  for (const token of targetTokens) {
    const rule = manifest.tokens[token];
    const components = findColorComponents(frame, rule.rgb, { minPixels: rule.minPixels });
    const component = [...components].sort((left, right) => right.pixels - left.pixels)[0];
    if (!component) continue;
    targets.push(Object.freeze({
      token,
      rect: component.rect,
      enabled: true,
      visible: true,
      source: 'framebuffer',
    }));
  }
  return Object.freeze(targets);
}

function verdictCues(components, rule) {
  const frame = components.find((component) =>
    component.rect.width >= rule.frameMinWidth &&
    component.rect.height >= rule.frameMinHeight &&
    component.fillRatio < 0.35);
  const icon = components.find((component) => {
    if (component === frame) return false;
    const { width, height } = component.rect;
    const aspect = width / height;
    return width >= rule.iconMinSize && width <= rule.iconMaxSize &&
      height >= rule.iconMinSize && height <= rule.iconMaxSize &&
      aspect >= 0.7 && aspect <= 1.3 &&
      component.fillRatio >= 0.05 && component.fillRatio <= 0.65;
  });
  return frame && icon ? Object.freeze({ frame, icon }) : null;
}

export function classifyFramebufferVerdict(frame, manifest) {
  validateFrame(frame, manifest);
  const matches = [];
  for (const verdict of VERDICTS) {
    const rule = manifest.verdicts[verdict];
    const components = findColorComponents(frame, rule.rgb, {
      minPixels: rule.minComponentPixels,
    });
    const cues = verdictCues(components, rule);
    if (cues) matches.push({ verdict, cues });
  }
  if (matches.length !== 1) {
    return Object.freeze({ verdict: 'NO_RESPONSE', confidence: 0, cueCount: 0 });
  }
  return Object.freeze({
    verdict: matches[0].verdict,
    confidence: 1,
    cueCount: 2,
  });
}
