#!/usr/bin/env node
// No-cost routing proxy: evaluates explicit intent patterns against the checked-in
// positive and near-miss corpus. It intentionally performs no model/API calls.
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const datasetPath = process.argv[2] || resolve(ROOT, '.agents/evals/trigger-queries.json');
const rulesPath = process.argv[3] || resolve(ROOT, '.agents/evals/trigger-rules.json');
const dataset = JSON.parse(readFileSync(datasetPath, 'utf8'));
const rulesDoc = JSON.parse(readFileSync(rulesPath, 'utf8'));

const compiled = new Map(
  Object.entries(rulesDoc.rules || {}).map(([skill, patterns]) => [
    skill,
    patterns.map((pattern) => new RegExp(pattern, 'iu')),
  ]),
);

const results = dataset.queries.map((item) => {
  const patterns = compiled.get(item.skill);
  const predicted = !!patterns?.some((pattern) => pattern.test(item.query));
  return {
    ...item,
    predicted,
    pass: predicted === item.should_trigger,
  };
});

const failed = results.filter((item) => !item.pass);
const bySplit = Object.fromEntries(
  [...new Set(results.map((item) => item.split))].map((split) => {
    const rows = results.filter((item) => item.split === split);
    return [split, { passed: rows.filter((item) => item.pass).length, total: rows.length }];
  }),
);

console.log(JSON.stringify({
  ok: failed.length === 0,
  costPolicy: 'no-llm-api',
  passed: results.length - failed.length,
  total: results.length,
  bySplit,
  failures: failed.map(({ query, skill, should_trigger, predicted }) => ({ query, skill, should_trigger, predicted })),
}, null, 2));

if (failed.length) process.exitCode = 1;
