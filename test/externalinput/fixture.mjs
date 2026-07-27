import { guardSnapshot } from '/static/js/injector.js';
import { startCollector, behaviorSummary } from '/static/js/collector.js';
import { probeEnvironment } from '/static/js/env.js';
import { collectAdvanced, computeFingerprintId } from '/static/js/advanced.js';
import { postReport } from '/static/js/transport.js';
import { initRIT } from '/static/js/rit.js';

const EXPECTED_TEXT = 'HUMANYMOUS SYNTHETIC TASK';
const COARSE_VERDICTS = new Set(['ALLOW', 'CHALLENGE', 'DENY']);
const IME_LOCALE = new URLSearchParams(location.search).get('ime');
let scoringStarted = false;

const byId = (id) => document.getElementById(id);
const showOnly = (id) => {
  for (const sectionId of [
    'read-step',
    'form-step',
    'nav-step',
    'idle-step',
    'challenge-step',
    'scoring-step',
  ]) {
    byId(sectionId).hidden = sectionId !== id;
  }
  byId(id).scrollIntoView({ block: 'start' });
};

if (IME_LOCALE) {
  const { startImeFixture } = await import('/static/external-input-ime.mjs');
  startImeFixture(IME_LOCALE);
} else {
  startEnglishFixture();
}

function startEnglishFixture() {
startCollector();

document.querySelector('[data-hmn-token="choice-correct"]').addEventListener('click', () => {
  showOnly('form-step');
});

byId('synthetic-form-shell').addEventListener('submit', (event) => {
  event.preventDefault();
  if (byId('synthetic-form').value !== EXPECTED_TEXT) {
    byId('form-error').textContent = 'The synthetic text does not match yet. Correct it and retry.';
    return;
  }
  byId('form-error').textContent = '';
  showOnly('nav-step');
});

document.querySelector('[data-hmn-token="nav-branch"]').addEventListener('click', () => {
  byId('branch-panel').hidden = false;
});

document.querySelector('[data-hmn-token="nav-return"]').addEventListener('click', () => {
  showOnly('idle-step');
});

document.querySelector('[data-hmn-token="resume-action"]').addEventListener('click', () => {
  showOnly('challenge-step');
});

document.querySelector('[data-hmn-token="challenge-action"]').addEventListener('click', () => {
  showOnly('scoring-step');
  runDetection();
});

async function loadWasm() {
  const go = new Go();
  const response = await fetch('/static/detector.wasm', { credentials: 'same-origin' });
  if (!response.ok) throw new Error('detector unavailable');
  const bytes = await response.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, go.importObject);
  go.run(instance);
}

async function runDetection() {
  if (scoringStarted) return;
  scoringStarted = true;
  try {
    await initRIT();
    await loadWasm();
    const report = JSON.parse(window.__hmDetect ? window.__hmDetect() : '{}');
    report.environment = await probeEnvironment();
    try {
      report.advanced = await Promise.race([
        collectAdvanced(),
        new Promise((resolve) => setTimeout(() => resolve({ probed: false }), 2_500)),
      ]);
    } catch {}
    try {
      report.fingerprintId = await computeFingerprintId();
    } catch {}
    const guard = guardSnapshot();
    report.guard = {
      reported: true,
      evalUsed: guard.evalUsed,
      funcCtor: guard.funcCtor,
      scriptInjected: guard.scriptInjected,
      protoPolluted: guard.protoPolluted,
      nativeHooked: guard.nativeHooked,
      consoleDisabled: guard.consoleDisabled,
    };
    report.behavior = behaviorSummary();
    const result = await postReport(report);
    renderVerdict(COARSE_VERDICTS.has(result?.verdict) ? result.verdict : 'NO_RESPONSE');
  } catch {
    renderVerdict('NO_RESPONSE');
  }
}

function renderVerdict(verdict) {
  byId('scoring-step').hidden = true;
  const region = byId('verdict-region');
  const normalized = COARSE_VERDICTS.has(verdict) ? verdict : 'NO_RESPONSE';
  const icon = {
    ALLOW: '●',
    CHALLENGE: '◆',
    DENY: '■',
    NO_RESPONSE: '?',
  }[normalized];
  region.className = `verdict-card verdict-${normalized.toLowerCase().replace('_', '-')}`;
  byId('verdict-icon').textContent = icon;
  byId('verdict-text').textContent = normalized;
  region.hidden = false;
  region.focus?.();
}
}
