// Freeze-safe Blue disposition for the defense-only human-mimic Red profile.
//
// This module audits measurement evidence. It does not score a session, add a
// signal, or recommend a verdict change. Candidate observations remain shadow
// only until a real-human cohort (including assistive technology) establishes
// an accessibility-safe baseline.

const REQUIRED_EPISODE_KEYS = Object.freeze([
  'profileId',
  'riskScore',
  'verdict',
  'hardRuleFired',
  'policyVersion',
  'scoreReceiptSha256',
  'coreLogSha256',
]);

function exactKeys(value, expected, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length ||
      actual.some((key, index) => key !== wanted[index])) {
    throw new TypeError(`${label} has an unexpected schema`);
  }
}

function sha256(value, label) {
  if (!/^[a-f0-9]{64}$/.test(value || '')) {
    throw new TypeError(`${label} must be a lowercase SHA-256 digest`);
  }
}

function freeze(value) {
  if (!value || typeof value !== 'object' || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

export const freezeSafeBlueCandidates = freeze([
  {
    id: 'higher-order-temporal-consistency',
    status: 'shadow-only-pending-human-baseline',
    observes: [
      'serial dependence within pointer and keyboard phases',
      'target-geometry-conditioned acquisition residuals',
      'pause-to-action and correction-phase transitions',
      'session-trait consistency across pointer, keyboard, scroll, and recovery',
    ],
    limits: [
      'reduced and privacy-bounded sequence sketches only',
      'never challenge on motor richness, speed, tremor, or precision alone',
      'requires cohorts covering assistive technology, keyboard-only use, and low-rate devices',
    ],
  },
  {
    id: 'run-bound-input-provenance',
    status: 'lab-corroboration-only',
    observes: [
      'independent operating-system seat events bound to the run receipt',
      'browser sandbox and input-backend identity',
      'kernel, image, profile, and evidence hashes',
    ],
    limits: [
      'a web page cannot establish kernel input lineage by itself',
      'trusted, operating-system, virtual-USB, or physical-USB input is not proof of a human',
      'provenance may classify the mechanism but cannot be a stand-alone humanity gate',
    ],
  },
  {
    id: 'defense-only-profile-invariants',
    status: 'catalog-regression-only',
    observes: [
      'deterministic seeded replay',
      'fixed four-or-five-segment pointer acquisition',
      'mandatory challenge overshoot and fixed task/correction structure',
    ],
    limits: [
      'valid only against this repository own Red profile',
      'must not become a production tell or a third-party probing technique',
    ],
  },
]);

export function blueDisposition(evidence) {
  exactKeys(evidence, [
    'schemaVersion',
    'runId',
    'browser',
    'challengeAt',
    'episodes',
    'scoreTraceLimit',
  ], 'Blue evidence');
  if (evidence.schemaVersion !== 'humanymous.blue-human-mimic-disposition/v1' ||
      evidence.browser !== 'chromium' ||
      !/^[A-Za-z0-9._-]{1,128}$/.test(evidence.runId || '') ||
      !Number.isFinite(evidence.challengeAt) ||
      evidence.challengeAt <= 0 ||
      !Array.isArray(evidence.episodes) ||
      evidence.episodes.length < 1) {
    throw new TypeError('Blue evidence identity or policy facts are invalid');
  }

  const episodes = evidence.episodes.map((episode, index) => {
    exactKeys(episode, REQUIRED_EPISODE_KEYS, `episode ${index}`);
    sha256(episode.scoreReceiptSha256, `episode ${index} score receipt`);
    sha256(episode.coreLogSha256, `episode ${index} Core log`);
    if (!Number.isFinite(episode.riskScore) ||
        !['ALLOW', 'CHALLENGE', 'DENY'].includes(episode.verdict) ||
        typeof episode.profileId !== 'string' ||
        typeof episode.policyVersion !== 'string') {
      throw new TypeError(`episode ${index} measurement is invalid`);
    }
    return Object.freeze({
      profileId: episode.profileId,
      scoreBandVerdict: episode.riskScore < evidence.challengeAt ? 'ALLOW' : 'NOT_ALLOW',
      finalVerdict: episode.verdict,
      decisiveRule: episode.hardRuleFired,
      behaviorCausedChallenge: episode.verdict === 'CHALLENGE' &&
        episode.riskScore >= evidence.challengeAt,
    });
  });

  const hr23Only = episodes.every((episode) =>
    episode.finalVerdict === 'CHALLENGE' &&
    episode.scoreBandVerdict === 'ALLOW' &&
    episode.decisiveRule === 'HR-23' &&
    episode.behaviorCausedChallenge === false);

  return freeze({
    verdict: hr23Only ? 'needs-work' : 'block',
    attribution: {
      decisiveMechanism: hr23Only ? 'HR-23' : 'unproven',
      behaviorTriggeredChallenge: !hr23Only,
      behaviorSignalAbsenceProven: false,
      explanation: hr23Only
        ? 'Each measured risk score is below the challenge band; the recorded HR-23 override alone changes the final verdict to CHALLENGE.'
        : 'The evidence does not isolate HR-23 as the decisive mechanism.',
    },
    episodes,
    scoreTraceLimit: evidence.scoreTraceLimit,
    gaps: [
      'The external-input score receipt does not retain behavioral contributors or the reduced BehaviorSummary.',
      'The current BehaviorSummary retains marginal summaries but no phase-ordered or cross-modal sequence representation.',
      'Core scoring has no run-bound operating-system input lineage in its BehaviorSummary.',
    ],
    candidates: freezeSafeBlueCandidates,
    ceiling: 'Coherent real-browser execution at human pace remains the intentional T4 ceiling; no claim is made that Blue can distinguish it from a human.',
  });
}
