#!/usr/bin/env node
// Deterministic local workflow state machine. It never imports an LLM SDK,
// opens a network connection, or invokes a provider API.
import { appendFileSync, closeSync, existsSync, mkdirSync, openSync, readFileSync, readdirSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { basename, dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import crypto from 'node:crypto';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..', '..');
const DEFAULT_WORKFLOWS = join(ROOT, '.agents', 'workflows');
const DEFAULT_RUNS = join(ROOT, '.agent-runs');
const TERMINAL = new Set(['completed', 'failed', 'cancelled']);

function scalar(raw) {
  const value = raw.replace(/\s+#.*$/, '').trim();
  if (/^\d+$/.test(value)) return Number(value);
  if (value === 'true') return true;
  if (value === 'false') return false;
  if (value === 'null') return null;
  return value.replace(/^(['"])(.*)\1$/, '$2');
}

export function parseWorkflow(text, source = '<memory>') {
  const workflow = { source, phases: [] };
  let phase = null;
  let inPhases = false;
  for (const line of text.split(/\r?\n/)) {
    if (/^phases:\s*$/.test(line)) {
      inPhases = true;
      continue;
    }
    const phaseStart = line.match(/^  - id:\s*(.+?)\s*$/);
    if (inPhases && phaseStart) {
      phase = { id: scalar(phaseStart[1]) };
      workflow.phases.push(phase);
      continue;
    }
    if (inPhases && phase) {
      const field = line.match(/^    ([a-zA-Z_][\w-]*):\s*(.*?)\s*$/);
      if (field && field[2] !== '>' && field[2] !== '|') phase[field[1]] = scalar(field[2]);
      continue;
    }
    const top = line.match(/^([a-zA-Z_][\w-]*):\s*(.*?)\s*$/);
    if (top && top[2] !== '>' && top[2] !== '|') workflow[top[1]] = scalar(top[2]);
  }
  return workflow;
}

export function validateWorkflow(workflow) {
  const errors = [];
  if (!/^[a-z0-9][a-z0-9-]*$/.test(workflow.id || '')) errors.push('id must be kebab-case');
  if (!Number.isInteger(workflow.version) || workflow.version < 1) errors.push('version must be a positive integer');
  if (!workflow.phases?.length) errors.push('phases must not be empty');
  const ids = new Set();
  for (const [index, phase] of (workflow.phases || []).entries()) {
    const at = `phases[${index}]`;
    if (!/^[a-z0-9][a-z0-9-]*$/.test(phase.id || '')) errors.push(`${at}.id must be kebab-case`);
    if (ids.has(phase.id)) errors.push(`${at}.id is duplicated`);
    ids.add(phase.id);
    if (!phase.exit) errors.push(`${at}.exit is required`);
    const timeout = phase.timeout_minutes ?? 30;
    const retries = phase.max_retries ?? 0;
    const approval = phase.approval ?? 'none';
    if (!Number.isInteger(timeout) || timeout < 1 || timeout > 1440) errors.push(`${at}.timeout_minutes is invalid`);
    if (!Number.isInteger(retries) || retries < 0 || retries > 10) errors.push(`${at}.max_retries is invalid`);
    if (!['none', 'required'].includes(approval)) errors.push(`${at}.approval is invalid`);
  }
  return errors;
}

function loadWorkflow(path) {
  const workflow = parseWorkflow(readFileSync(path, 'utf8'), path);
  const errors = validateWorkflow(workflow);
  if (errors.length) throw new Error(`${path}: ${errors.join('; ')}`);
  workflow.phases = workflow.phases.map((phase) => ({
    ...phase,
    timeout_minutes: phase.timeout_minutes ?? 30,
    max_retries: phase.max_retries ?? 0,
    approval: phase.approval ?? 'none',
  }));
  return workflow;
}

function parseArgs(argv) {
  const [command = 'help', ...rest] = argv;
  const options = {};
  for (let i = 0; i < rest.length; i++) {
    const token = rest[i];
    if (!token.startsWith('--')) throw new Error(`unexpected argument: ${token}`);
    const key = token.slice(2);
    const value = rest[i + 1] && !rest[i + 1].startsWith('--') ? rest[++i] : true;
    options[key] = value;
  }
  return { command, options };
}

function workflowPath(options) {
  if (options['workflow-file']) return resolve(String(options['workflow-file']));
  if (!options.workflow) throw new Error('--workflow or --workflow-file is required');
  return join(resolve(String(options['workflow-root'] || DEFAULT_WORKFLOWS)), `${options.workflow}.yaml`);
}

function runsRoot(options) {
  return resolve(String(options['runs-root'] || DEFAULT_RUNS));
}

function safeRunDir(options) {
  if (!options.run) throw new Error('--run is required');
  const root = runsRoot(options);
  const candidate = isAbsolute(String(options.run)) ? resolve(String(options.run)) : resolve(root, String(options.run));
  const rel = relative(root, candidate);
  if (rel.startsWith('..') || isAbsolute(rel)) throw new Error(`run must stay under ${root}`);
  return candidate;
}

function gitSha() {
  const result = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' });
  return result.status === 0 ? result.stdout.trim() : null;
}

function now() {
  return new Date().toISOString();
}

function atomicWrite(path, value) {
  const temp = `${path}.tmp-${process.pid}-${crypto.randomBytes(3).toString('hex')}`;
  writeFileSync(temp, JSON.stringify(value, null, 2) + '\n', 'utf8');
  renameSync(temp, path);
}

function journal(runDir, type, details = {}) {
  appendFileSync(join(runDir, 'journal.jsonl'), JSON.stringify({ at: now(), type, ...details }) + '\n', 'utf8');
}

function withLock(runDir, action) {
  const lockPath = join(runDir, '.lock');
  let fd;
  try {
    fd = openSync(lockPath, 'wx');
  } catch (error) {
    if (error.code === 'EEXIST') throw new Error(`run is locked: ${runDir}`);
    throw error;
  }
  try {
    const statePath = join(runDir, 'state.json');
    const state = JSON.parse(readFileSync(statePath, 'utf8'));
    const result = action(state);
    state.updatedAt = now();
    atomicWrite(statePath, state);
    return result ?? state;
  } finally {
    closeSync(fd);
    rmSync(lockPath, { force: true });
  }
}

function startPhase(state, index) {
  if (index >= state.phases.length) {
    state.status = 'completed';
    state.currentPhase = null;
    state.completedAt = now();
    return;
  }
  state.phaseIndex = index;
  state.status = 'running';
  state.currentPhase = state.phases[index].id;
  state.phases[index].status = 'in_progress';
  state.phases[index].attempts++;
  state.phases[index].startedAt = now();
}

function advance(state) {
  startPhase(state, state.phaseIndex + 1);
}

function stateSummary(state) {
  const current = state.phases[state.phaseIndex];
  let overdue = false;
  if (state.status === 'running' && current?.startedAt) {
    overdue = Date.now() - Date.parse(current.startedAt) > current.timeoutMinutes * 60_000;
  }
  return { ...state, overdue };
}

function commandValidate(options) {
  const root = resolve(String(options['workflow-root'] || DEFAULT_WORKFLOWS));
  const files = options['workflow-file']
    ? [resolve(String(options['workflow-file']))]
    : readdirSync(root).filter((name) => name.endsWith('.yaml')).map((name) => join(root, name));
  const reports = files.map((path) => {
    const workflow = parseWorkflow(readFileSync(path, 'utf8'), path);
    return { file: relative(ROOT, path), id: workflow.id, errors: validateWorkflow(workflow) };
  });
  const failures = reports.filter((report) => report.errors.length);
  console.log(JSON.stringify({ ok: failures.length === 0, workflows: reports }, null, 2));
  if (failures.length) process.exitCode = 1;
}

function commandStart(options) {
  if (!options.objective) throw new Error('--objective is required');
  const workflow = loadWorkflow(workflowPath(options));
  const provider = String(options.provider || 'local');
  const root = runsRoot(options);
  mkdirSync(root, { recursive: true });
  const stamp = now().replace(/\D/g, '').slice(0, 14);
  const runId = `${provider.replace(/[^a-z0-9-]/gi, '-').toLowerCase()}-${workflow.id}-${stamp}-${crypto.randomBytes(3).toString('hex')}`;
  const runDir = join(root, runId);
  mkdirSync(runDir);
  const state = {
    schemaVersion: 1,
    runId,
    workflow: { id: workflow.id, version: workflow.version, source: relative(ROOT, workflow.source) },
    objective: String(options.objective),
    provider,
    costPolicy: 'no-llm-api',
    gitSha: gitSha(),
    createdAt: now(),
    updatedAt: now(),
    status: 'running',
    phaseIndex: 0,
    currentPhase: workflow.phases[0].id,
    artifacts: [],
    evidence: [],
    approvals: [],
    phases: workflow.phases.map((phase) => ({
      id: phase.id,
      status: 'pending',
      attempts: 0,
      timeoutMinutes: phase.timeout_minutes,
      maxRetries: phase.max_retries,
      approval: phase.approval,
      exit: phase.exit,
    })),
  };
  startPhase(state, 0);
  atomicWrite(join(runDir, 'state.json'), state);
  journal(runDir, 'run_started', { runId, workflow: workflow.id, provider, costPolicy: 'no-llm-api' });
  journal(runDir, 'phase_started', { phase: state.currentPhase, attempt: 1 });
  console.log(JSON.stringify({ runId, runDir, state }, null, 2));
}

function commandStatus(options) {
  const runDir = safeRunDir(options);
  const state = JSON.parse(readFileSync(join(runDir, 'state.json'), 'utf8'));
  console.log(JSON.stringify(stateSummary(state), null, 2));
}

function commandComplete(options) {
  const runDir = safeRunDir(options);
  const state = withLock(runDir, (state) => {
    if (state.status !== 'running') throw new Error(`cannot complete while status=${state.status}`);
    const phase = state.phases[state.phaseIndex];
    if (options.artifact) state.artifacts.push(String(options.artifact));
    if (options.evidence) state.evidence.push(String(options.evidence));
    phase.finishedAt = now();
    if (phase.approval === 'required') {
      phase.status = 'awaiting_approval';
      state.status = 'awaiting_approval';
      journal(runDir, 'phase_awaiting_approval', { phase: phase.id });
    } else {
      phase.status = 'completed';
      journal(runDir, 'phase_completed', { phase: phase.id });
      advance(state);
      if (state.status === 'running') journal(runDir, 'phase_started', { phase: state.currentPhase, attempt: 1 });
      else journal(runDir, 'run_completed');
    }
  });
  console.log(JSON.stringify(state, null, 2));
}

function commandApprove(options) {
  const runDir = safeRunDir(options);
  const state = withLock(runDir, (state) => {
    if (state.status !== 'awaiting_approval') throw new Error(`cannot approve while status=${state.status}`);
    const phase = state.phases[state.phaseIndex];
    const by = String(options.by || 'human');
    phase.status = 'completed';
    phase.approvedBy = by;
    phase.approvedAt = now();
    state.approvals.push({ phase: phase.id, by, at: phase.approvedAt });
    journal(runDir, 'phase_approved', { phase: phase.id, by });
    advance(state);
    if (state.status === 'running') journal(runDir, 'phase_started', { phase: state.currentPhase, attempt: 1 });
    else journal(runDir, 'run_completed');
  });
  console.log(JSON.stringify(state, null, 2));
}

function commandFail(options) {
  const runDir = safeRunDir(options);
  const state = withLock(runDir, (state) => {
    if (state.status !== 'running') throw new Error(`cannot fail while status=${state.status}`);
    const phase = state.phases[state.phaseIndex];
    const reason = String(options.reason || 'unspecified failure');
    journal(runDir, 'phase_failed', { phase: phase.id, attempt: phase.attempts, reason });
    if (phase.attempts <= phase.maxRetries) {
      phase.status = 'in_progress';
      phase.attempts++;
      phase.startedAt = now();
      phase.lastFailure = reason;
      journal(runDir, 'phase_retried', { phase: phase.id, attempt: phase.attempts });
    } else {
      phase.status = 'failed';
      phase.finishedAt = now();
      phase.failure = reason;
      state.status = 'failed';
      state.failedAt = now();
    }
  });
  console.log(JSON.stringify(state, null, 2));
}

function commandCancel(options) {
  const runDir = safeRunDir(options);
  const state = withLock(runDir, (state) => {
    if (TERMINAL.has(state.status)) throw new Error(`cannot cancel while status=${state.status}`);
    const phase = state.phases[state.phaseIndex];
    if (phase) phase.status = 'cancelled';
    state.status = 'cancelled';
    state.cancelledAt = now();
    state.cancelReason = String(options.reason || 'cancelled by operator');
    journal(runDir, 'run_cancelled', { reason: state.cancelReason });
  });
  console.log(JSON.stringify(state, null, 2));
}

export function main(argv = process.argv.slice(2)) {
  const { command, options } = parseArgs(argv);
  const commands = {
    validate: commandValidate,
    start: commandStart,
    status: commandStatus,
    complete: commandComplete,
    approve: commandApprove,
    fail: commandFail,
    cancel: commandCancel,
  };
  if (!commands[command]) {
    console.log('usage: workflow-runner.mjs validate|start|status|complete|approve|fail|cancel [options]');
    return command === 'help' ? 0 : 2;
  }
  commands[command](options);
  return process.exitCode || 0;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    process.exitCode = main();
  } catch (error) {
    console.error(`workflow-runner: ${error.message}`);
    process.exitCode = 2;
  }
}
