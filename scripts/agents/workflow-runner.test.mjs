import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { parseWorkflow, validateWorkflow } from './workflow-runner.mjs';

const ROOT = resolve(import.meta.dirname, '..', '..');
const CLI = join(ROOT, 'scripts', 'agents', 'workflow-runner.mjs');

function cli(args) {
  const result = spawnSync(process.execPath, [CLI, ...args], { cwd: ROOT, encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  return JSON.parse(result.stdout);
}

test('parses and validates the executable workflow subset', () => {
  const workflow = parseWorkflow(readFileSync(join(ROOT, '.agents', 'workflows', 'feature-loop.yaml'), 'utf8'));
  assert.equal(workflow.id, 'feature-loop');
  assert.equal(workflow.phases[0].id, 'coordinate');
  assert.deepEqual(validateWorkflow(workflow), []);
});

test('persists state, journal, retry, and approval without an LLM API', () => {
  const runs = mkdtempSync(join(tmpdir(), 'hmn-agent-runs-'));
  const started = cli([
    'start', '--workflow', 'feature-loop', '--objective', 'local state test',
    '--provider', 'codex', '--runs-root', runs,
  ]);
  assert.equal(started.state.costPolicy, 'no-llm-api');
  assert.equal(started.state.currentPhase, 'coordinate');

  let state = cli(['complete', '--run', started.runId, '--runs-root', runs, '--evidence', 'claim-ok']);
  assert.equal(state.currentPhase, 'spec');

  state = cli(['fail', '--run', started.runId, '--runs-root', runs, '--reason', 'draft retry']);
  assert.equal(state.phases[state.phaseIndex].attempts, 2);

  state = cli(['complete', '--run', started.runId, '--runs-root', runs, '--artifact', 'plan/test.md']);
  assert.equal(state.status, 'awaiting_approval');

  state = cli(['approve', '--run', started.runId, '--runs-root', runs, '--by', 'test-human']);
  assert.equal(state.currentPhase, 'critique');
  assert.equal(state.approvals[0].by, 'test-human');

  const journal = readFileSync(join(runs, started.runId, 'journal.jsonl'), 'utf8');
  assert.match(journal, /phase_retried/);
  assert.match(journal, /phase_approved/);
});
