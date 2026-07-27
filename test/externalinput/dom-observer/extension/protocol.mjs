// Shared by the Chromium service worker and the native host. It intentionally
// contains no DOM, network, detector, or mutation API.

export const PROTOCOL_VERSION = '1.2.0';
export const NATIVE_HOST_NAME = 'org.humanymous.external_input_dom_observer';
export const EXTENSION_ID = 'gapgaeefmeaooimpnofjjahjnhfgnjlm';
export const FIXTURE_ORIGIN = 'https://core:8443';
export const FIXTURE_PATH = '/static/external-input.html';
export const MAX_REQUEST_BYTES = 4 * 1024;
export const MAX_RESPONSE_BYTES = 64 * 1024;
export const MAX_DEADLINE_AHEAD_MS = 2_000;
export const MAX_SNAPSHOT_NODES = 64;

export const METHODS = Object.freeze([
  'snapshot',
  'findByRole',
  'findByTextToken',
  'rectForNode',
  'visibleState',
]);

export const TOKENS = Object.freeze([
  'choice-correct',
  'synthetic-form',
  'nav-branch',
  'nav-return',
  'resume-action',
  'challenge-action',
  'ime-input',
]);

export const ROLES = Object.freeze(['button', 'checkbox', 'link', 'radio', 'textbox']);

const METHOD_SET = new Set(METHODS);
const TOKEN_SET = new Set(TOKENS);
const ROLE_SET = new Set(ROLES);
const ENVELOPE_FIELDS = new Set(['protocolVersion', 'sequenceId', 'deadlineUnixMs', 'request']);
const REQUEST_FIELDS = Object.freeze({
  snapshot: new Set(['method']),
  findByRole: new Set(['method', 'role', 'nameToken']),
  findByTextToken: new Set(['method', 'token']),
  rectForNode: new Set(['method', 'nodeId']),
  visibleState: new Set(['method', 'nodeId']),
});
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const NODE_ID = /^[A-Za-z0-9_-]{1,96}$/;

function plainObject(value, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function exactFields(value, allowed, label) {
  for (const field of Object.keys(value)) {
    if (!allowed.has(field)) throw new TypeError(`${label} contains unknown field: ${field}`);
  }
}

function validateRequest(value) {
  const request = plainObject(value, 'request');
  if (!METHOD_SET.has(request.method)) throw new TypeError('request method is not allowlisted');
  exactFields(request, REQUEST_FIELDS[request.method], 'request');
  if (request.method === 'findByTextToken' && !TOKEN_SET.has(request.token)) {
    throw new TypeError('text token is not allowlisted');
  }
  if (request.method === 'findByRole') {
    if (!ROLE_SET.has(request.role)) throw new TypeError('role is not allowlisted');
    if (!TOKEN_SET.has(request.nameToken)) throw new TypeError('name token is not allowlisted');
  }
  if ((request.method === 'rectForNode' || request.method === 'visibleState') &&
      !NODE_ID.test(request.nodeId || '')) {
    throw new TypeError('opaque node ID is invalid');
  }
  return Object.freeze({ ...request });
}

export function validateEnvelope(value, { now = Date.now() } = {}) {
  const envelope = plainObject(value, 'envelope');
  exactFields(envelope, ENVELOPE_FIELDS, 'envelope');
  if (envelope.protocolVersion !== PROTOCOL_VERSION) {
    throw new TypeError('protocol version mismatch');
  }
  if (!UUID.test(envelope.sequenceId || '')) throw new TypeError('sequence ID is invalid');
  if (!Number.isInteger(envelope.deadlineUnixMs) ||
      envelope.deadlineUnixMs < now ||
      envelope.deadlineUnixMs > now + MAX_DEADLINE_AHEAD_MS) {
    throw new TypeError('deadline is expired or outside the bounded window');
  }
  return Object.freeze({
    protocolVersion: PROTOCOL_VERSION,
    sequenceId: envelope.sequenceId,
    deadlineUnixMs: envelope.deadlineUnixMs,
    request: validateRequest(envelope.request),
  });
}

export class ReplayGuard {
  #seen = new Map();
  #maxEntries;

  constructor(maxEntries = 1_024) {
    this.#maxEntries = maxEntries;
  }

  accept(sequenceId, deadlineUnixMs, now = Date.now()) {
    for (const [id, deadline] of this.#seen) {
      if (deadline < now) this.#seen.delete(id);
    }
    if (this.#seen.has(sequenceId)) throw new TypeError('replayed sequence ID');
    if (this.#seen.size >= this.#maxEntries) throw new TypeError('replay window is full');
    this.#seen.set(sequenceId, deadlineUnixMs);
  }
}

function safeRect(value) {
  const rect = plainObject(value, 'rectangle');
  exactFields(rect, new Set(['x', 'y', 'width', 'height']), 'rectangle');
  for (const field of ['x', 'y', 'width', 'height']) {
    if (!Number.isFinite(rect[field])) throw new TypeError(`rectangle ${field} is invalid`);
  }
  if (rect.width <= 0 || rect.height <= 0 ||
      Math.abs(rect.x) > 8_192 || Math.abs(rect.y) > 8_192 ||
      rect.width > 4_096 || rect.height > 2_160) {
    throw new TypeError('rectangle exceeds bounds');
  }
  return Object.freeze({ x: rect.x, y: rect.y, width: rect.width, height: rect.height });
}

function safeTarget(value) {
  const target = plainObject(value, 'target');
  const fields = new Set(['token', 'rect', 'enabled', 'visible', 'nodeId']);
  exactFields(target, fields, 'target');
  if (!TOKEN_SET.has(target.token)) throw new TypeError('response token is not allowlisted');
  if (!NODE_ID.test(target.nodeId || '')) throw new TypeError('response node ID is invalid');
  if (typeof target.enabled !== 'boolean' || typeof target.visible !== 'boolean') {
    throw new TypeError('target state is invalid');
  }
  return Object.freeze({
    token: target.token,
    rect: safeRect(target.rect),
    enabled: target.enabled,
    visible: target.visible,
    nodeId: target.nodeId,
  });
}

export function sanitizeResult(request, value) {
  if (value === null || value === undefined) return null;
  if (request.method === 'snapshot') {
    if (!Array.isArray(value) || value.length > MAX_SNAPSHOT_NODES) {
      throw new TypeError('snapshot is invalid or exceeds the node bound');
    }
    return Object.freeze(value.map(safeTarget));
  }
  if (request.method === 'findByRole' || request.method === 'findByTextToken') {
    const target = safeTarget(value);
    const expected = request.token || request.nameToken;
    if (target.token !== expected) throw new TypeError('response token does not match the request');
    return target;
  }
  if (request.method === 'rectForNode') return safeRect(value);
  if (request.method === 'visibleState') {
    const state = plainObject(value, 'visible state');
    const fields = new Set(['visible', 'enabled', 'checked', 'selected', 'focused']);
    exactFields(state, fields, 'visible state');
    if (Object.keys(state).length !== fields.size) {
      throw new TypeError('visible state is incomplete');
    }
    for (const field of fields) {
      if (typeof state[field] !== 'boolean') throw new TypeError(`visible state ${field} is invalid`);
    }
    return Object.freeze({ ...state });
  }
  throw new TypeError('unreachable result method');
}

export function assertFixtureLocation(locationLike) {
  if (locationLike.origin !== FIXTURE_ORIGIN || locationLike.pathname !== FIXTURE_PATH) {
    throw new TypeError('observer is restricted to the fixed external-input fixture');
  }
}
