// collector.js — passive behavioral event collection (SoT-03 §5). Records a
// bounded rolling buffer of pointer/key/wheel events and summarizes them into a
// BehaviorSummary matching the Go schema. Never logs key values (privacy).

import { mouseFeatures } from './features/mouse.js';
import { keyFeatures } from './features/keystroke.js';

const MAX = 600; // cap buffered samples to bound memory/CPU
const state = {
  start: performance.now(),
  moves: [],
  downs: [],
  ups: [],
  events: 0,
  untrusted: 0,
  noMoveClicks: 0,
  pastes: 0,
  lastMoveT: 0,
  coalescedTotal: 0,
  coalescedSamples: 0,
  clicks: 0,
  eventTimes: [], // timestamps of all input events (for LLM-loop cadence)
};

function push(arr, v) { if (arr.length < MAX) arr.push(v); }

// mark records the timestamp of any input event for LLM-loop cadence analysis
// (SoT-16): AI agents produce bursts of near-instant actions separated by
// multi-second model-inference "thinking" pauses.
function mark() { if (state.eventTimes.length < MAX) state.eventTimes.push(performance.now()); }

export function startCollector() {
  addEventListener('pointermove', (e) => {
    state.events++;
    if (!e.isTrusted) state.untrusted++;
    // Coalesced events (SoT-14): real pointing devices batch many high-frequency
    // sub-samples into one pointermove; synthetic input produces ~1.
    let coalesced = 1;
    try { if (typeof e.getCoalescedEvents === 'function') coalesced = e.getCoalescedEvents().length || 1; } catch (_) {}
    state.coalescedTotal += coalesced;
    state.coalescedSamples++;
    push(state.moves, { x: e.clientX, y: e.clientY, t: performance.now() });
    state.lastMoveT = performance.now();
    mark();
  }, { passive: true });

  addEventListener('pointerdown', (e) => {
    state.events++;
    state.clicks++;
    if (!e.isTrusted) state.untrusted++;
    // click with no recent movement is a synthetic-input tell.
    if (performance.now() - state.lastMoveT > 800 || state.moves.length === 0) state.noMoveClicks++;
    mark();
  }, { passive: true });

  addEventListener('keydown', (e) => {
    state.events++;
    if (!e.isTrusted) state.untrusted++;
    push(state.downs, { code: e.code || 'k', t: performance.now() });
    mark();
  }, { passive: true });

  addEventListener('keyup', (e) => {
    push(state.ups, { code: e.code || 'k', t: performance.now() });
  }, { passive: true });

  addEventListener('wheel', () => { state.events++; mark(); }, { passive: true });
  addEventListener('paste', () => { state.pastes++; }, { passive: true });
}

// cadence analyses the input-event timeline for the LLM inference-loop signature
// (SoT-16): long "thinking" gaps punctuating bursts of low-variance actions.
function cadence(times) {
  const out = { longGapCount: 0, burstVarMs: 0, minReactionMs: 0 };
  if (times.length < 4) return out;
  const gaps = [];
  for (let i = 1; i < times.length; i++) gaps.push(times[i] - times[i - 1]);
  const LONG = 1500; // model-inference pause threshold (ms)
  const intra = [];  // gaps inside bursts (not the long pauses)
  let minReaction = Infinity;
  for (let i = 0; i < gaps.length; i++) {
    if (gaps[i] > LONG) {
      out.longGapCount++;
      // the action right after a long pause: human reaction floor ~200-300ms;
      // agents resume instantly.
      if (i + 1 < gaps.length) minReaction = Math.min(minReaction, gaps[i + 1]);
    } else {
      intra.push(gaps[i]);
    }
  }
  out.burstVarMs = variance(intra);
  out.minReactionMs = minReaction === Infinity ? 0 : minReaction;
  return out;
}

function variance(a) {
  if (a.length < 2) return 0;
  const m = a.reduce((s, x) => s + x, 0) / a.length;
  return a.reduce((s, x) => s + (x - m) * (x - m), 0) / a.length;
}

export function behaviorSummary() {
  const mouse = mouseFeatures(state.moves);
  mouse.coalescedRatio = state.coalescedSamples ? state.coalescedTotal / state.coalescedSamples : 0;
  return {
    mouse,
    key: keyFeatures(state.downs, state.ups, state.pastes),
    events: {
      totalEvents: state.events,
      untrustedFrac: state.events ? state.untrusted / state.events : 0,
      noMoveClicks: state.noMoveClicks,
      syntheticFlags: 0,
      clickCount: state.clicks,
      ...cadence(state.eventTimes),
    },
    durationS: (performance.now() - state.start) / 1000,
  };
}
