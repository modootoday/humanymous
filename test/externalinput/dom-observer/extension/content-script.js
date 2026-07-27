(() => {
  'use strict';

  const FIXTURE_ORIGIN = 'https://core:8443';
  const FIXTURE_PATH = '/static/external-input.html';
  const TOKENS = new Set([
    'choice-correct',
    'synthetic-form',
    'nav-branch',
    'nav-return',
    'resume-action',
    'challenge-action',
    'ime-input',
  ]);
  const ROLES = new Set(['button', 'checkbox', 'link', 'radio', 'textbox']);
  const METHODS = new Set([
    'snapshot',
    'findByRole',
    'findByTextToken',
    'rectForNode',
    'visibleState',
  ]);
  const nodesById = new Map();
  const idsByNode = new WeakMap();

  if (location.origin !== FIXTURE_ORIGIN || location.pathname !== FIXTURE_PATH) return;

  function opaqueId(node) {
    let id = idsByNode.get(node);
    if (!id) {
      id = crypto.randomUUID();
      idsByNode.set(node, id);
      nodesById.set(id, node);
    }
    return id;
  }

  function visible(node) {
    if (!(node instanceof HTMLElement) || node.hidden) return false;
    const rect = node.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return false;
    if (rect.right <= 0 || rect.bottom <= 0 ||
        rect.left >= window.innerWidth || rect.top >= window.innerHeight) return false;
    const style = getComputedStyle(node);
    return style.display !== 'none' && style.visibility !== 'hidden' && Number(style.opacity) > 0;
  }

  function enabled(node) {
    return !(node.matches(':disabled') || node.getAttribute('aria-disabled') === 'true');
  }

  function roleOf(node) {
    const explicit = node.getAttribute('role');
    if (explicit) return explicit;
    if (node instanceof HTMLButtonElement) return 'button';
    if (node instanceof HTMLAnchorElement) return 'link';
    if (node instanceof HTMLInputElement) {
      if (node.type === 'checkbox') return 'checkbox';
      if (node.type === 'radio') return 'radio';
      return 'textbox';
    }
    return '';
  }

  function rectOf(node) {
    const rect = node.getBoundingClientRect();
    const viewportX = window.screenX + Math.max(0, (window.outerWidth - window.innerWidth) / 2);
    const viewportY = window.screenY + Math.max(0, window.outerHeight - window.innerHeight);
    return {
      x: Math.round(viewportX + rect.x),
      y: Math.round(viewportY + rect.y),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    };
  }

  function targetOf(node) {
    const token = node.dataset.hmnToken;
    if (!TOKENS.has(token)) throw new TypeError('fixture target token is not allowlisted');
    return {
      token,
      rect: rectOf(node),
      enabled: enabled(node),
      visible: visible(node),
      nodeId: opaqueId(node),
    };
  }

  function fixtureNodes() {
    return [...document.querySelectorAll('[data-hmn-token]')]
      .filter((node) => TOKENS.has(node.dataset.hmnToken));
  }

  function exactRequestFields(request, fields) {
    for (const field of Object.keys(request)) {
      if (!fields.has(field)) throw new TypeError(`unknown request field: ${field}`);
    }
  }

  function execute(request) {
    if (!request || typeof request !== 'object' || !METHODS.has(request.method)) {
      throw new TypeError('method is not allowlisted');
    }
    if (request.method === 'snapshot') {
      exactRequestFields(request, new Set(['method']));
      return fixtureNodes().filter(visible).map(targetOf);
    }
    if (request.method === 'findByTextToken') {
      exactRequestFields(request, new Set(['method', 'token']));
      if (!TOKENS.has(request.token)) throw new TypeError('text token is not allowlisted');
      const node = fixtureNodes().find((candidate) => candidate.dataset.hmnToken === request.token);
      return node && visible(node) ? targetOf(node) : null;
    }
    if (request.method === 'findByRole') {
      exactRequestFields(request, new Set(['method', 'role', 'nameToken']));
      if (!ROLES.has(request.role) || !TOKENS.has(request.nameToken)) {
        throw new TypeError('role or synthetic name token is not allowlisted');
      }
      const node = fixtureNodes().find((candidate) =>
        roleOf(candidate) === request.role &&
        candidate.dataset.hmnNameToken === request.nameToken);
      return node && visible(node) ? targetOf(node) : null;
    }
    exactRequestFields(request, new Set(['method', 'nodeId']));
    if (!/^[A-Za-z0-9_-]{1,96}$/.test(request.nodeId || '')) {
      throw new TypeError('opaque node ID is invalid');
    }
    const node = nodesById.get(request.nodeId);
    if (!node || !node.isConnected || !TOKENS.has(node.dataset.hmnToken)) return null;
    if (request.method === 'rectForNode') return visible(node) ? rectOf(node) : null;
    if (request.method === 'visibleState') {
      return {
        visible: visible(node),
        enabled: enabled(node),
        checked: node.matches(':checked'),
        selected: node.matches(':checked,[aria-selected="true"]'),
        focused: document.activeElement === node,
      };
    }
    throw new TypeError('unreachable method');
  }

  const port = chrome.runtime.connect({ name: 'hmn-dom-observer-content' });
  port.onMessage.addListener((message) => {
    if (!message || message.type !== 'query' || typeof message.sequenceId !== 'string') return;
    if (!Number.isInteger(message.deadlineUnixMs) || Date.now() > message.deadlineUnixMs) {
      port.postMessage({ type: 'response', sequenceId: message.sequenceId, error: 'deadline expired' });
      return;
    }
    try {
      const result = execute(message.request);
      port.postMessage({ type: 'response', sequenceId: message.sequenceId, result });
    } catch (error) {
      port.postMessage({
        type: 'response',
        sequenceId: message.sequenceId,
        error: String(error?.message || error).slice(0, 256),
      });
    }
  });
})();
