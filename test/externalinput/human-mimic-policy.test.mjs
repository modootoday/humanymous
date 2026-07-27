import assert from 'node:assert/strict';
import test from 'node:test';
import { createHumanMimicPolicy } from './human-mimic-policy.mjs';

const TARGET = Object.freeze({ x: 760, y: 430, width: 140, height: 54 });

function correlation(left, right) {
  const leftMean = left.reduce((sum, value) => sum + value, 0) / left.length;
  const rightMean = right.reduce((sum, value) => sum + value, 0) / right.length;
  let numerator = 0;
  let leftSquare = 0;
  let rightSquare = 0;
  for (let index = 0; index < left.length; index += 1) {
    const leftDelta = left[index] - leftMean;
    const rightDelta = right[index] - rightMean;
    numerator += leftDelta * rightDelta;
    leftSquare += leftDelta ** 2;
    rightSquare += rightDelta ** 2;
  }
  return numerator / Math.sqrt(leftSquare * rightSquare);
}

function withinTarget(point, rect) {
  return point.x >= rect.x && point.x < rect.x + rect.width &&
    point.y >= rect.y && point.y < rect.y + rect.height;
}

test('same seed reproduces a complete correlated policy trace', () => {
  function trace() {
    const policy = createHumanMimicPolicy('repeatable-red-session');
    return {
      traits: policy.traits,
      decisions: [
        policy.decisionPause('read-select', 0),
        policy.decisionPause('form-correction', 1),
      ],
      pointer: policy.pointerPlan(TARGET),
      keys: [...'HUMANYMOUS'].map((key, index) => policy.keyTiming(key, index)),
      idle: policy.idleBreak(),
      hesitation: policy.hesitation(4, { challenge: true }),
    };
  }
  assert.deepEqual(trace(), trace());
  const other = createHumanMimicPolicy('different-red-session').pointerPlan(TARGET);
  assert.notDeepEqual(trace().pointer, other);
});

test('curved acquisition stays bounded and finishes inside the selected target', () => {
  for (let index = 0; index < 96; index += 1) {
    const plan = createHumanMimicPolicy(`geometry-${index}`).pointerPlan(TARGET, {
      purpose: index % 3 === 0 ? 'challenge' : 'activate',
    });
    assert.ok(plan.moves.length >= 4 && plan.moves.length <= 5);
    for (const move of plan.moves) {
      assert.equal(move.kind, 'pointerMove');
      assert.ok(move.x >= 0 && move.x < 1280);
      assert.ok(move.y >= 0 && move.y < 720);
      assert.ok(move.durationMs >= 20 && move.durationMs <= 2_000);
    }
    assert.equal(withinTarget(plan.moves.at(-1), TARGET), true);
    assert.ok(plan.clickDwellMs >= 30 && plan.clickDwellMs <= 145);

    const first = plan.moves[0];
    const middle = plan.moves[Math.floor(plan.moves.length / 2)];
    const last = plan.moves.at(-1);
    const crossProduct =
      (middle.x - first.x) * (last.y - first.y) -
      (middle.y - first.y) * (last.x - first.x);
    assert.notEqual(crossProduct, 0, 'path collapsed to an exactly straight line');
  }
});

test('challenge acquisition always includes a bounded overshoot correction', () => {
  for (let index = 0; index < 48; index += 1) {
    const plan = createHumanMimicPolicy(`challenge-${index}`)
      .pointerPlan(TARGET, { purpose: 'challenge' });
    assert.equal(plan.corrected, true);
    assert.equal(plan.moves.length, 5);
    assert.notDeepEqual(plan.moves.at(-2), plan.moves.at(-1));
    assert.equal(withinTarget(plan.moves.at(-1), TARGET), true);
  }
});

test('pointer duration follows target distance and size rather than a fixed delay', () => {
  const nearDurations = [];
  const farDurations = [];
  const wideDurations = [];
  for (let index = 0; index < 128; index += 1) {
    nearDurations.push(createHumanMimicPolicy(`fitts-${index}`)
      .pointerPlan({ x: 90, y: 70, width: 38, height: 30 }).totalDurationMs);
    farDurations.push(createHumanMimicPolicy(`fitts-${index}`)
      .pointerPlan({ x: 1040, y: 610, width: 38, height: 30 }).totalDurationMs);
    wideDurations.push(createHumanMimicPolicy(`fitts-${index}`)
      .pointerPlan({ x: 960, y: 540, width: 260, height: 180 }).totalDurationMs);
  }
  const average = (values) => values.reduce((sum, value) => sum + value, 0) / values.length;
  assert.ok(average(farDurations) > average(nearDurations) + 180);
  assert.ok(average(farDurations) > average(wideDurations) + 80);
  assert.ok(new Set(farDurations).size > 60, 'duration distribution is too repetitive');
});

test('key dwell and flight retain positive within-session autocorrelation', () => {
  const policy = createHumanMimicPolicy('correlated-key-session');
  const timings = Array.from({ length: 240 }, (_, index) =>
    policy.keyTiming(index % 7 === 6 ? 'Space' : 'H', index));
  const dwell = timings.map((timing) => timing.dwellMs);
  const flight = timings.map((timing) => timing.flightMs);
  assert.ok(correlation(dwell.slice(0, -1), dwell.slice(1)) > 0.12);
  assert.ok(correlation(flight.slice(0, -1), flight.slice(1)) > 0.12);
  for (const timing of timings) {
    assert.ok(timing.dwellMs >= 28 && timing.dwellMs <= 145);
    assert.ok(timing.flightMs >= 32 && timing.flightMs <= 220);
  }
});

test('stable session pace correlates motor timing across pointer and keyboard modalities', () => {
  const pointer = [];
  const keyboard = [];
  for (let index = 0; index < 160; index += 1) {
    const policy = createHumanMimicPolicy(`cross-modal-${index}`);
    pointer.push(policy.pointerPlan(TARGET).totalDurationMs);
    const timings = Array.from({ length: 24 }, (_, keyIndex) =>
      policy.keyTiming('H', keyIndex));
    keyboard.push(
      timings.reduce((sum, timing) => sum + timing.dwellMs + timing.flightMs, 0) /
      timings.length,
    );
  }
  assert.ok(correlation(pointer, keyboard) > 0.22);
});

test('decision, hesitation, idle, scroll, and correction envelopes remain bounded', () => {
  const decisionValues = [];
  let zeroHesitations = 0;
  let nonzeroHesitations = 0;
  for (let index = 0; index < 160; index += 1) {
    const policy = createHumanMimicPolicy(`envelope-${index}`);
    const decision = policy.decisionPause('read-select', index % 5);
    const recovery = policy.decisionPause('visible-challenge', 4, { recovery: true });
    const hesitation = policy.hesitation(index % 5);
    const challengeHesitation = policy.hesitation(4, { challenge: true });
    const idle = policy.idleBreak();
    const scroll = policy.scrollAmount(index % 4);
    decisionValues.push(decision, recovery);
    if (hesitation === 0) zeroHesitations += 1;
    else nonzeroHesitations += 1;
    assert.ok(decision >= 180 && decision <= 2_600);
    assert.ok(recovery >= 180 && recovery <= 2_600);
    assert.ok(challengeHesitation >= 120 && challengeHesitation <= 1_900);
    assert.ok(idle >= 900 && idle <= 3_800);
    assert.ok(scroll >= 220 && scroll <= 460);
  }
  assert.ok(zeroHesitations > 40);
  assert.ok(nonzeroHesitations > 20);
  assert.ok(new Set(decisionValues).size > 200);
});

test('policy surface exposes no detector oracle or adaptive verdict feedback', () => {
  const policy = createHumanMimicPolicy('no-oracle');
  const surface = JSON.stringify({
    keys: Object.keys(policy),
    traits: policy.traits,
    decision: policy.decisionPause('visible-challenge', 4, { recovery: true }),
    pointer: policy.pointerPlan(TARGET, { purpose: 'challenge' }),
  }).toLowerCase();
  for (const forbidden of ['riskscore', 'threshold', 'verdict', 'detector', 'oracle']) {
    assert.equal(surface.includes(forbidden), false);
  }
});
