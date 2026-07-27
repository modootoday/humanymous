import { createHash } from 'node:crypto';
import { ContractViolationError } from './errors.mjs';

export const DOM_QUERY_METHODS = Object.freeze([
  'snapshot', 'findByRole', 'findByTextToken', 'rectForNode', 'visibleState',
]);

const DOM_METHODS = new Set(DOM_QUERY_METHODS);
const TARGET_FIELDS = new Set(['token', 'rect', 'enabled', 'visible', 'source', 'nodeId']);
const DOM_ROLES = new Set(['button', 'checkbox', 'link', 'radio', 'textbox']);
const STATE_FIELDS = new Set(['visible', 'enabled', 'checked', 'selected', 'focused']);

function assertToken(token, field = 'token') {
  if (!/^[a-z0-9][a-z0-9-]{0,63}$/.test(token || '')) {
    throw new ContractViolationError(`${field} is not an allowlisted synthetic token`);
  }
}

function hashFrame(frame) {
  if (typeof frame?.sha256 === 'string' && /^[a-f0-9]{64}$/.test(frame.sha256)) {
    return frame.sha256;
  }
  if (frame?.pixels instanceof Uint8Array) {
    return createHash('sha256').update(frame.pixels).digest('hex');
  }
  throw new ContractViolationError('frame must provide sha256 or bounded pixel bytes');
}

function normalizeRect(rect) {
  if (!rect || !['x', 'y', 'width', 'height'].every((key) => Number.isFinite(rect[key]))) {
    throw new ContractViolationError('target rectangle is invalid');
  }
  if (rect.width <= 0 || rect.height <= 0) {
    throw new ContractViolationError('target rectangle must have positive area');
  }
  return Object.freeze({
    x: rect.x,
    y: rect.y,
    width: rect.width,
    height: rect.height,
  });
}

function normalizeTarget(target, defaultSource) {
  if (!target || typeof target !== 'object') throw new ContractViolationError('target is invalid');
  for (const field of Object.keys(target)) {
    if (!TARGET_FIELDS.has(field)) throw new ContractViolationError(`forbidden target field: ${field}`);
  }
  assertToken(target.token, 'target token');
  return Object.freeze({
    token: target.token,
    rect: normalizeRect(target.rect),
    enabled: target.enabled !== false,
    visible: target.visible !== false,
    source: target.source || defaultSource,
    ...(target.nodeId ? { nodeId: String(target.nodeId) } : {}),
  });
}

function validateDomRequest(request) {
  if (!request || typeof request !== 'object' || !DOM_METHODS.has(request.method)) {
    throw new ContractViolationError('DOM request method is not allowlisted');
  }
  const allowedFields = {
    snapshot: new Set(['method']),
    findByRole: new Set(['method', 'role', 'nameToken']),
    findByTextToken: new Set(['method', 'token']),
    rectForNode: new Set(['method', 'nodeId']),
    visibleState: new Set(['method', 'nodeId']),
  }[request.method];
  for (const field of Object.keys(request)) {
    if (!allowedFields.has(field)) throw new ContractViolationError(`forbidden DOM request field: ${field}`);
  }
  if (request.method === 'findByTextToken') assertToken(request.token);
  if (request.method === 'findByRole') {
    if (!DOM_ROLES.has(request.role)) throw new ContractViolationError('DOM role is not allowlisted');
    assertToken(request.nameToken, 'nameToken');
  }
  if (request.method === 'rectForNode' || request.method === 'visibleState') {
    if (!/^[A-Za-z0-9_-]{1,96}$/.test(request.nodeId || '')) {
      throw new ContractViolationError('opaque DOM node ID is invalid');
    }
  }
  return Object.freeze({ ...request });
}

function validateDomResponse(request, response) {
  if (response === null || response === undefined) return null;
  if (request.method === 'findByTextToken' || request.method === 'findByRole') {
    const token = request.token || request.nameToken;
    return normalizeTarget({ ...response, token, source: 'dom' }, 'dom');
  }
  if (request.method === 'rectForNode') {
    if (Object.keys(response).some((field) => !['x', 'y', 'width', 'height'].includes(field))) {
      throw new ContractViolationError('DOM rectangle response contains a forbidden field');
    }
    return normalizeRect(response);
  }
  if (request.method === 'visibleState') {
    for (const field of Object.keys(response)) {
      if (!STATE_FIELDS.has(field) || typeof response[field] !== 'boolean') {
        throw new ContractViolationError('DOM visible-state response contains a forbidden field');
      }
    }
    return Object.freeze({ ...response });
  }
  if (request.method === 'snapshot') {
    if (!Array.isArray(response)) throw new ContractViolationError('DOM snapshot must be an array');
    return Object.freeze(response.map((target) => normalizeTarget(target, 'dom')));
  }
  throw new ContractViolationError('unreachable DOM response method');
}

function commonAdapter({ kind, domEnabled, readFrame, locateVisualTargets, queryDom, domManifest }) {
  if (typeof readFrame !== 'function' || typeof locateVisualTargets !== 'function') {
    throw new TypeError('observation requires readFrame and locateVisualTargets functions');
  }
  if (domEnabled && typeof queryDom !== 'function') {
    throw new TypeError('DOM observation requires a typed queryDom function');
  }
  if (domEnabled && (
    !/^[a-f0-9]{64}$/.test(domManifest?.extensionSha256 || '') ||
    !/^[a-f0-9]{64}$/.test(domManifest?.manifestSha256 || '')
  )) {
    throw new ContractViolationError('DOM observer extension/manifest hashes are not pinned');
  }

  return Object.freeze({
    kind,
    domEnabled,
    ...(domEnabled ? { domManifest: Object.freeze({ ...domManifest }) } : {}),
    async observe({ taskId, targetTokens = [] }) {
      const frame = await readFrame();
      const frameSha256 = hashFrame(frame);
      const visual = await locateVisualTargets({ taskId, targetTokens, frameSha256, frame });
      if (!Array.isArray(visual)) throw new ContractViolationError('visual locator must return targets');
      const targets = visual.map((target) => normalizeTarget(target, 'framebuffer'));
      let domQueries = 0;

      if (domEnabled) {
        for (const token of targetTokens) {
          const request = validateDomRequest({ method: 'findByTextToken', token });
          const result = validateDomResponse(request, await queryDom(request));
          domQueries += 1;
          if (result) targets.push(result);
        }
      }

      return Object.freeze({
        taskId,
        frameSha256,
        width: frame.width,
        height: frame.height,
        targets: Object.freeze(targets),
        cues: Object.freeze(Array.isArray(frame.cues) ? [...frame.cues] : []),
        domQueries,
      });
    },
    async query(request) {
      if (!domEnabled) throw new ContractViolationError('DOM observer is absent in this mode');
      const safeRequest = validateDomRequest(request);
      return validateDomResponse(safeRequest, await queryDom(safeRequest));
    },
  });
}

export function createFramebufferObservation(options) {
  return commonAdapter({
    ...options,
    kind: 'framebuffer',
    domEnabled: false,
    queryDom: undefined,
  });
}

export function createDomFramebufferObservation(options) {
  return commonAdapter({
    ...options,
    kind: 'dom+framebuffer',
    domEnabled: true,
  });
}
