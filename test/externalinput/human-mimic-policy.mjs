// Defense-only Red policy for the fixed external-input fixture.
//
// The model intentionally receives task geometry and task names only. It has no
// detector result, risk score, threshold, or feedback channel. Its irregularity
// is correlated through stable per-session traits and short-lived motor state;
// it is not a collection of independent random sleeps.

const DISPLAY_WIDTH = 1280;
const DISPLAY_HEIGHT = 720;

function clamp(value, minimum, maximum) {
  return Math.min(maximum, Math.max(minimum, value));
}

function integer(value, minimum, maximum) {
  return Math.round(clamp(value, minimum, maximum));
}

function seedNumber(seed) {
  const text = String(seed || '0');
  let value = 2166136261;
  for (const char of text) value = Math.imul(value ^ char.charCodeAt(0), 16777619);
  return value >>> 0;
}

function createRandom(seed) {
  let state = seedNumber(seed) || 0x6d2b79f5;
  let spareNormal;

  function uniform() {
    state += 0x6d2b79f5;
    let value = state;
    value = Math.imul(value ^ (value >>> 15), value | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
  }

  function normal() {
    if (spareNormal !== undefined) {
      const value = spareNormal;
      spareNormal = undefined;
      return value;
    }
    const radius = Math.sqrt(-2 * Math.log(Math.max(Number.EPSILON, uniform())));
    const angle = 2 * Math.PI * uniform();
    spareNormal = radius * Math.sin(angle);
    return radius * Math.cos(angle);
  }

  return Object.freeze({ uniform, normal });
}

function pointInDisplay(point) {
  return Object.freeze({
    x: integer(point.x, 0, DISPLAY_WIDTH - 1),
    y: integer(point.y, 0, DISPLAY_HEIGHT - 1),
  });
}

function cubicPoint(start, control1, control2, end, progress) {
  const remainder = 1 - progress;
  return pointInDisplay({
    x: remainder ** 3 * start.x +
      3 * remainder ** 2 * progress * control1.x +
      3 * remainder * progress ** 2 * control2.x +
      progress ** 3 * end.x,
    y: remainder ** 3 * start.y +
      3 * remainder ** 2 * progress * control1.y +
      3 * remainder * progress ** 2 * control2.y +
      progress ** 3 * end.y,
  });
}

function targetPoint(rect, random) {
  const insetX = Math.max(2, Math.min(rect.width * 0.28, 18));
  const insetY = Math.max(2, Math.min(rect.height * 0.28, 14));
  return pointInDisplay({
    x: clamp(
      rect.x + rect.width / 2 + random.normal() * Math.max(1, rect.width * 0.08),
      rect.x + insetX,
      rect.x + rect.width - insetX,
    ),
    y: clamp(
      rect.y + rect.height / 2 + random.normal() * Math.max(1, rect.height * 0.08),
      rect.y + insetY,
      rect.y + rect.height - insetY,
    ),
  });
}

const TASK_COMPLEXITY = Object.freeze({
  'read-select': 1.12,
  'form-correction': 0.82,
  'multi-step-navigation': 1.02,
  'idle-resume': 0.72,
  'visible-challenge': 1.38,
});

export function createHumanMimicPolicy(seed, {
  width = DISPLAY_WIDTH,
  height = DISPLAY_HEIGHT,
} = {}) {
  if (width !== DISPLAY_WIDTH || height !== DISPLAY_HEIGHT) {
    throw new TypeError('human mimic policy requires the canonical 1280x720 fixture');
  }

  const random = createRandom(seed);
  const traits = Object.freeze({
    pace: 0.86 + random.uniform() * 0.34,
    precision: 0.72 + random.uniform() * 0.24,
    deliberation: 0.82 + random.uniform() * 0.42,
    correctionBias: 0.18 + random.uniform() * 0.28,
    modalityPreference: random.uniform(),
  });
  let cursor = Object.freeze({ x: 0, y: 0 });
  let dwellState = 58 + random.normal() * 7;
  let flightState = 86 + random.normal() * 13;
  let attentionState = random.normal() * 0.15;
  let fatigue = random.uniform() * 0.08;
  let actionCount = 0;

  function decisionPause(taskId, taskIndex, { recovery = false } = {}) {
    const complexity = TASK_COMPLEXITY[taskId] || 1;
    attentionState = 0.64 * attentionState + 0.36 * random.normal();
    fatigue = clamp(fatigue + 0.025 + random.uniform() * 0.018, 0, 0.42);
    const base = 390 * complexity * traits.deliberation;
    const reading = taskId === 'read-select' ? 260 + random.uniform() * 420 : 0;
    const recoveryCost = recovery ? 380 + random.uniform() * 620 : 0;
    const laterTaskCost = taskIndex * (34 + 38 * fatigue);
    return integer(
      base + reading + recoveryCost + laterTaskCost +
        attentionState * 125 + random.normal() * 92,
      180,
      2_600,
    );
  }

  function hesitation(taskIndex, { challenge = false } = {}) {
    const probability = challenge ? 1 : 0.24 + fatigue * 0.65;
    if (random.uniform() >= probability) return 0;
    const taskCost = Math.min(260, taskIndex * 31);
    return integer(
      170 + taskCost + (challenge ? 420 : 0) + random.uniform() * (challenge ? 980 : 650),
      120,
      1_900,
    );
  }

  function idleBreak() {
    attentionState = 0.42 * attentionState + random.normal() * 0.2;
    fatigue = clamp(fatigue - 0.09, 0, 0.42);
    return integer(
      (1_260 + random.uniform() * 1_780) * traits.deliberation,
      900,
      3_800,
    );
  }

  function pointerPlan(rect, { purpose = 'activate' } = {}) {
    const target = targetPoint(rect, random);
    const dx = target.x - cursor.x;
    const dy = target.y - cursor.y;
    const distance = Math.max(1, Math.hypot(dx, dy));
    const targetWidth = Math.max(8, Math.min(rect.width, rect.height));
    const indexOfDifficulty = Math.log2(distance / targetWidth + 1);
    const totalDuration = integer(
      (105 + 116 * indexOfDifficulty) * traits.pace *
        (1 + fatigue * 0.28) + random.normal() * 34,
      120,
      1_450,
    );
    const normalX = -dy / distance;
    const normalY = dx / distance;
    const curveDirection = random.uniform() < 0.5 ? -1 : 1;
    const curve = clamp(
      distance * (0.035 + random.uniform() * 0.075) *
        curveDirection * (1.18 - traits.precision * 0.28),
      -86,
      86,
    );
    const control1 = pointInDisplay({
      x: cursor.x + dx * (0.23 + random.uniform() * 0.1) + normalX * curve,
      y: cursor.y + dy * (0.23 + random.uniform() * 0.1) + normalY * curve,
    });
    const control2 = pointInDisplay({
      x: cursor.x + dx * (0.68 + random.uniform() * 0.12) - normalX * curve * 0.64,
      y: cursor.y + dy * (0.68 + random.uniform() * 0.12) - normalY * curve * 0.64,
    });
    const progress = [0.24, 0.49, 0.74];
    const points = progress.map((value) =>
      cubicPoint(cursor, control1, control2, target, value));

    const overshootProbability = clamp(
      0.2 + indexOfDifficulty * 0.07 + traits.correctionBias - traits.precision * 0.18,
      0.18,
      0.68,
    );
    const corrected = random.uniform() < overshootProbability || purpose === 'challenge';
    if (corrected) {
      const overshoot = 3 + random.uniform() * Math.min(20, 5 + indexOfDifficulty * 3);
      points.push(pointInDisplay({
        x: target.x + (dx / distance) * overshoot + normalX * random.normal() * 2.4,
        y: target.y + (dy / distance) * overshoot + normalY * random.normal() * 2.4,
      }));
    }
    points.push(target);

    const weights = points.map((_, index) =>
      index === points.length - 1 && corrected ? 0.13 : 1);
    const weightTotal = weights.reduce((sum, value) => sum + value, 0);
    let remaining = totalDuration;
    const moves = points.map((point, index) => {
      const last = index === points.length - 1;
      const durationMs = last
        ? remaining
        : integer(totalDuration * weights[index] / weightTotal, 20, totalDuration);
      remaining = Math.max(20, remaining - durationMs);
      return Object.freeze({
        kind: 'pointerMove',
        x: point.x,
        y: point.y,
        durationMs: integer(durationMs, 20, 2_000),
      });
    });
    cursor = target;
    actionCount += moves.length;
    return Object.freeze({
      moves: Object.freeze(moves),
      corrected,
      distance: Math.round(distance),
      indexOfDifficulty,
      totalDurationMs: moves.reduce((sum, move) => sum + move.durationMs, 0),
      clickDwellMs: integer(
        61 * traits.pace + random.normal() * 13 + (purpose === 'challenge' ? 12 : 0),
        30,
        145,
      ),
    });
  }

  function keyTiming(key, index, { correction = false } = {}) {
    const characterClass = key === 'Space' ? 'space' :
      key === 'Backspace' ? 'correction' : 'letter';
    const burstPosition = index % (4 + Math.floor(traits.pace * 3));
    const burstAdjustment = burstPosition === 0 ? 14 + random.uniform() * 24 : -4;
    dwellState = 0.58 * dwellState +
      0.42 * (54 * traits.pace + random.normal() * 13);
    flightState = 0.71 * flightState +
      0.29 * (76 * traits.pace + random.normal() * 24);
    const classDwell = characterClass === 'correction' ? 17 :
      characterClass === 'space' ? -5 : 0;
    const classFlight = characterClass === 'space' ? 26 :
      characterClass === 'correction' ? 42 : 0;
    const correctionCost = correction ? 12 : 0;
    actionCount += 1;
    fatigue = clamp(fatigue + 0.0025, 0, 0.42);
    return Object.freeze({
      dwellMs: integer(
        dwellState + classDwell + correctionCost + random.normal() * 4,
        28,
        145,
      ),
      flightMs: integer(
        flightState + classFlight + burstAdjustment +
          fatigue * 38 + random.normal() * 7,
        32,
        220,
      ),
    });
  }

  function scrollAmount(scanIndex) {
    const directionConsistency = scanIndex === 0 ? 1 : 0.84 + random.uniform() * 0.16;
    return integer(
      (255 + random.uniform() * 155) * directionConsistency,
      220,
      460,
    );
  }

  function keyboardNavigationTiming(key, index = 0) {
    return keyTiming(key, actionCount + index);
  }

  return Object.freeze({
    traits,
    decisionPause,
    hesitation,
    idleBreak,
    pointerPlan,
    keyTiming,
    keyboardNavigationTiming,
    scrollAmount,
  });
}

export const humanMimicPolicyInternals = Object.freeze({
  seedNumber,
  display: Object.freeze({ width: DISPLAY_WIDTH, height: DISPLAY_HEIGHT }),
});
