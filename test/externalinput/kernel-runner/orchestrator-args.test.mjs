import assert from 'node:assert/strict';
import test from 'node:test';

import { parseArguments } from '../../../scripts/external-input-kernel-runner.mjs';
import {
  RUNTIME_BUILD_PLAN,
  selectedRuntimeBuildPlan,
  TAGS,
} from '../../../scripts/external-input-vusb-images.mjs';
import {
  cellRuntimeImageKeys,
} from './outer.mjs';

test('accepts exact 3V and 4V no-oracle kernel runner inputs', () => {
  assert.deepEqual(parseArguments([
    '--model', 'reference-relative-v1',
    '--mode', '4v',
    '--strategy-seed', 'strongest-human-mimic-0001',
  ]), {
    browser: 'chromium',
    modelId: 'reference-relative-v1',
    mode: '4v',
    strategySeed: 'strongest-human-mimic-0001',
  });
  assert.deepEqual(parseArguments([
    '--model', 'reference-relative-v1',
    '--mode', '3v',
    '--no-build',
  ]), {
    browser: 'chromium',
    modelId: 'reference-relative-v1',
    mode: '3v',
    noBuild: true,
  });
});

test('rejects unknown models, modes, and duplicate authority inputs', () => {
  assert.throws(
    () => parseArguments(['--model', 'comparison-latest']),
    /allowlisted/,
  );
  assert.throws(
    () => parseArguments([
      '--model', 'reference-relative-v1',
      '--browser', 'webkit',
    ]),
    /chromium or firefox/,
  );
  assert.throws(
    () => parseArguments(['--model', 'reference-relative-v1', '--mode', '5v']),
    /3v or 4v/,
  );
  assert.throws(
    () => parseArguments([
      '--model', 'reference-relative-v1',
      '--strategy-seed', 'first',
      '--strategy-seed', 'second',
    ]),
    /more than once/,
  );
  assert.throws(
    () => parseArguments([
      '--model', 'reference-relative-v1',
      '--no-build',
      '--no-build',
    ]),
    /more than once/,
  );
});

test('rebuild plan covers every exact runtime image tag once', () => {
  const plannedTags = RUNTIME_BUILD_PLAN.map(([, tag]) => tag).sort();
  assert.deepEqual(plannedTags, Object.values(TAGS).sort());
  assert.equal(new Set(plannedTags).size, plannedTags.length);
});

test('one kernel cell builds only its exact runtime image set', () => {
  const keys = cellRuntimeImageKeys('chromium', 3);
  const selected = selectedRuntimeBuildPlan(keys);
  assert.equal(selected.length, keys.length);
  assert.deepEqual(
    selected.map(([, tag]) => tag).sort(),
    keys.map((key) => TAGS[key]).sort(),
  );
  assert.ok(!selected.some(([, tag]) => tag === TAGS.browserFirefox));
  assert.ok(!selected.some(([, tag]) => tag === TAGS.browserChromiumDom));
});
