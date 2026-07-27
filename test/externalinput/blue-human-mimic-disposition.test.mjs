import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import {
  blueDisposition,
  freezeSafeBlueCandidates,
} from './blue-human-mimic-disposition.mjs';

const evidencePath = new URL('./blue-human-mimic-20260726c.evidence.json', import.meta.url);
const hardRulesPath = new URL('../../internal/scoring/hardrules.go', import.meta.url);
const policyPath = new URL('../../internal/scoring/policy.go', import.meta.url);
const signalsPath = new URL('../../internal/signals/signals.go', import.meta.url);
const collectorPath = new URL('../../web/js/collector.js', import.meta.url);

async function evidence() {
  return JSON.parse(await readFile(evidencePath, 'utf8'));
}

test('measured Chromium challenge is attributed to HR-23, not the score band', async () => {
  const disposition = blueDisposition(await evidence());
  assert.equal(disposition.verdict, 'needs-work');
  assert.deepEqual(disposition.attribution, {
    decisiveMechanism: 'HR-23',
    behaviorTriggeredChallenge: false,
    behaviorSignalAbsenceProven: false,
    explanation:
      'Each measured risk score is below the challenge band; the recorded HR-23 override alone changes the final verdict to CHALLENGE.',
  });
  assert.deepEqual(
    disposition.episodes.map(({ scoreBandVerdict, finalVerdict, decisiveRule }) => ({
      scoreBandVerdict,
      finalVerdict,
      decisiveRule,
    })),
    [
      { scoreBandVerdict: 'ALLOW', finalVerdict: 'CHALLENGE', decisiveRule: 'HR-23' },
      { scoreBandVerdict: 'ALLOW', finalVerdict: 'CHALLENGE', decisiveRule: 'HR-23' },
    ],
  );
});

test('attribution remains bound to the current frozen scoring facts', async () => {
  const [hardRules, policy, measured] = await Promise.all([
    readFile(hardRulesPath, 'utf8'),
    readFile(policyPath, 'utf8'),
    evidence(),
  ]);
  assert.match(policy, /ChallengeAt:\s*30,/);
  assert.equal(measured.challengeAt, 30);

  const start = hardRules.indexOf('{"HR-23"');
  const end = hardRules.indexOf('// HR-24', start);
  assert.ok(start >= 0 && end > start, 'HR-23 rule block is missing');
  const hr23 = hardRules.slice(start, end);
  assert.match(hr23, /l2\.adv\.no_widevine/);
  assert.match(hr23, /l2\.adv\.no_media_devices/);
  assert.match(hr23, /l2\.adv\.no_voices/);
  assert.doesNotMatch(hr23, /l4\./);
});

test('Blue gaps match the data actually retained by Core scoring', async () => {
  const [signals, collector] = await Promise.all([
    readFile(signalsPath, 'utf8'),
    readFile(collectorPath, 'utf8'),
  ]);
  const start = signals.indexOf('type BehaviorSummary struct');
  const end = signals.indexOf('// MouseFeatures', start);
  assert.ok(start >= 0 && end > start, 'BehaviorSummary schema is missing');
  const behaviorSummary = signals.slice(start, end);

  assert.match(behaviorSummary, /Mouse\s+MouseFeatures/);
  assert.match(behaviorSummary, /Key\s+KeyFeatures/);
  assert.match(behaviorSummary, /Events\s+EventFeatures/);
  assert.doesNotMatch(behaviorSummary, /Provenance|Sequence|CrossModal|Phase/);
  assert.match(collector, /return \{\s*mouse,\s*key:/s);
  assert.doesNotMatch(collector, /operatingSystemProvenance|inputBackendAttestation/);
});

test('all candidate Blue work is freeze-safe, accessibility-safe, and limits-first', () => {
  assert.deepEqual(
    freezeSafeBlueCandidates.map(({ status }) => status),
    [
      'shadow-only-pending-human-baseline',
      'lab-corroboration-only',
      'catalog-regression-only',
    ],
  );
  const policy = JSON.stringify(freezeSafeBlueCandidates).toLowerCase();
  assert.match(policy, /never challenge on motor richness/);
  assert.match(policy, /assistive technology/);
  assert.match(policy, /not proof of a human/);
  assert.match(policy, /must not become a production tell/);
});

test('a score-band challenge is never misattributed to HR-23', async () => {
  const changed = await evidence();
  changed.episodes[0].riskScore = changed.challengeAt;
  assert.equal(blueDisposition(changed).verdict, 'block');
});
