// injector.js — runs first, before any other page script. It captures pristine
// native references and installs the script-injection / eval guards (SoT-11).
// Everything it collects is exposed on window.__hmGuard for the loader.

const pristine = {
  fetch: window.fetch.bind(window),
  Function: window.Function,
  eval: window.eval,
  toString: Function.prototype.toString,
  defineProperty: Object.defineProperty,
  MutationObserver: window.MutationObserver,
  now: performance.now.bind(performance),
};

const guard = {
  evalUsed: 0,
  funcCtor: 0,
  scriptInjected: 0,
  protoPolluted: false,
  nativeHooked: false,
  consoleDisabled: false,
};

// Snapshot core prototypes to detect later pollution.
const protoSnapshot = {
  objectKeys: Object.getOwnPropertyNames(Object.prototype).length,
  arrayKeys: Object.getOwnPropertyNames(Array.prototype).length,
};

// Instrument dynamic code execution vectors (observe, do not block — the page
// may legitimately use them; SoT-11 §3). We wrap on a best-effort basis.
try {
  const realEval = window.eval;
  // Note: reassigning window.eval only affects indirect eval; direct eval is
  // lexical. This still catches (0,eval)() style bypass attempts.
  window.eval = function (...a) { guard.evalUsed++; return realEval.apply(this, a); };
} catch (_) {}

try {
  const RealFunction = window.Function;
  const Wrapped = function (...a) { guard.funcCtor++; return RealFunction.apply(this, a); };
  Wrapped.prototype = RealFunction.prototype;
  // Only override the global reference; constructor access via (fn).constructor
  // still reaches RealFunction, so this is a partial (best-effort) probe.
  window.Function = Wrapped;
} catch (_) {}

// Detect native hooks on fetch/XHR/JSON/toString (SoT-11 / L3).
function looksNative(fn) {
  try { return /\{\s*\[native code\]\s*\}/.test(pristine.toString.call(fn)); }
  catch (_) { return false; }
}
guard.nativeHooked = !(looksNative(window.fetch) && looksNative(window.XMLHttpRequest) &&
  looksNative(JSON.stringify) && looksNative(JSON.parse));

// Watch for injected <script> nodes without our marker.
try {
  const mo = new pristine.MutationObserver((muts) => {
    for (const m of muts) {
      for (const node of m.addedNodes) {
        if (node.tagName === 'SCRIPT' && !node.dataset?.hm) guard.scriptInjected++;
      }
    }
  });
  mo.observe(document.documentElement, { childList: true, subtree: true });
} catch (_) {}

// CDP Runtime.enable probe (SoT-04): if a DevTools client with Runtime.enable
// is attached, serializing a logged Error triggers a user-defined stack getter.
// Mostly patched in Chrome since 2025-05, but still catches stale frameworks.
try {
  let cdp = false;
  const e = new Error();
  Object.defineProperty(e, 'stack', { get() { cdp = true; return ''; } });
  // eslint-disable-next-line no-console
  console.debug(e);
  window.__hmCDP = cdp;
} catch (_) { window.__hmCDP = false; }

// Post-May-2025 CDP probe (SoT-14): the V8 patch that killed the Error.stack
// getter probe only guarded Error previews. A Proxy on the PROTOTYPE chain still
// gets its ownKeys trap invoked during CDP object-preview serialization, because
// DebugPropertyIterator walks onto the prototype and KeyAccumulator must call
// the trap. Catches CURRENT Chrome driven over CDP (Playwright/Puppeteer/etc.).
try {
  let leaked = false;
  const trap = new Proxy({}, { ownKeys() { leaked = true; return []; } });
  const probe = Object.create(trap);
  // eslint-disable-next-line no-console
  console.groupEnd(probe);
  window.__hmCDP2 = leaked;
} catch (_) { window.__hmCDP2 = false; }

// Console-API sanity (SoT-04): Patchright disables the Console API entirely to
// kill the console-serialization CDP leak. A missing / non-native console.log is
// itself an anomaly (a real browser's console.log is native code).
try {
  const c = window.console;
  guard.consoleDisabled = !c || typeof c.log !== 'function' || !looksNative(c.log);
} catch (_) { guard.consoleDisabled = true; }

// Re-check prototype pollution at read time.
export function guardSnapshot() {
  guard.protoPolluted =
    Object.getOwnPropertyNames(Object.prototype).length !== protoSnapshot.objectKeys ||
    Object.getOwnPropertyNames(Array.prototype).length !== protoSnapshot.arrayKeys;
  return { ...guard };
}

export const nativeRefs = pristine;
window.__hmGuard = guardSnapshot;
