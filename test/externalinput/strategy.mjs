import { STRATEGY_VERSION, TASK_SUITE, TASK_SUITE_VERSION } from './contracts.mjs';
import { ContractViolationError } from './errors.mjs';
import { createHumanMimicPolicy } from './human-mimic-policy.mjs';

const SYNTHETIC_TEXT_PREFIX = 'HUMANYMOUS SYNTHETIC TAS';

function targetFor(state, token, optional = false) {
  const targets = state.targets.filter((target) => target.token === token && target.visible && target.enabled);
  const target = targets.find((item) => item.source === 'dom') || targets[0];
  if (!target && !optional) throw new ContractViolationError(`visible target is missing: ${token}`);
  return target;
}

function wait(delayMs) {
  return new Promise((resolve) => setTimeout(resolve, delayMs));
}

function hasRequiredTargets(state, task) {
  return (task.targetTokens || []).every((token) =>
    state.targets.some((target) =>
      target.token === token && target.visible === true && target.enabled === true));
}

async function approachAndActivate(input, target, keyboardOnly, policy, {
  purpose = 'activate',
  taskIndex = 0,
} = {}) {
  if (keyboardOnly) {
    const tab = policy.keyboardNavigationTiming('Tab');
    const enter = policy.keyboardNavigationTiming('Enter', 1);
    await input.perform({ kind: 'keyStroke', key: 'Tab', modifiers: [], ...tab });
    await input.perform({ kind: 'keyStroke', key: 'Enter', modifiers: [], ...enter });
    return;
  }
  const plan = policy.pointerPlan(target.rect, { purpose });
  for (const action of plan.moves) await input.perform(action);
  const hesitationMs = policy.hesitation(taskIndex, { challenge: purpose === 'challenge' });
  if (hesitationMs) await input.perform({ kind: 'pause', durationMs: hesitationMs });
  await input.perform({
    kind: 'pointerClick',
    button: 'left',
    dwellMs: plan.clickDwellMs,
  });
}

async function typeSyntheticCorrection(input, policy) {
  const text = `${SYNTHETIC_TEXT_PREFIX}J`;
  for (let index = 0; index < text.length; index += 1) {
    const key = text[index] === ' ' ? 'Space' : text[index];
    const timing = policy.keyTiming(key, index);
    await input.perform({
      kind: 'keyStroke',
      key,
      modifiers: [],
      ...timing,
    });
  }
  await input.perform({
    kind: 'keyStroke',
    key: 'Backspace',
    modifiers: [],
    ...policy.keyTiming('Backspace', text.length, { correction: true }),
  });
  await input.perform({
    kind: 'keyStroke',
    key: 'K',
    modifiers: [],
    ...policy.keyTiming('K', text.length + 1, { correction: true }),
  });
  await input.perform({
    kind: 'keyStroke',
    key: 'Enter',
    modifiers: [],
    ...policy.keyTiming('Enter', text.length + 2),
  });
}

export async function runStrategicPolicy({
  observation,
  input,
  seed,
  tasks = TASK_SUITE,
  strategyVariant = 'mixed-input',
  observationTimeoutMs = 5_000,
  observationPollMs = 125,
  transitionProbeMs = 350,
  sleep = wait,
}) {
  if (strategyVariant !== 'mixed-input' && strategyVariant !== 'keyboard-only') {
    throw new ContractViolationError(`unknown strategy variant: ${strategyVariant}`);
  }
  const keyboardOnly = strategyVariant === 'keyboard-only';
  const policy = createHumanMimicPolicy(seed);
  const results = [];
  let decisions = 0;
  let frames = 0;
  let recoveries = 0;
  let modalityChanges = 0;
  let visibleChallenges = 0;
  let domQueries = 0;
  const started = Date.now();

  async function observeStable(task, timeoutMs = observationTimeoutMs) {
    const deadline = Date.now() + timeoutMs;
    let state;
    do {
      state = await observation.observe(task);
      frames += 1;
      domQueries += state.domQueries;
      if (hasRequiredTargets(state, task)) return state;
      if (Date.now() >= deadline) return state;
      await sleep(observationPollMs);
    } while (true);
  }

  for (let index = 0; index < tasks.length; index += 1) {
    const task = tasks[index];
    const taskStarted = Date.now();

    try {
      // The return control is intentionally hidden until the branch is opened.
      // A human reads and acts on the visible branch first; waiting for both
      // states would manufacture a fixed timeout before every navigation.
      const initialTask = task.id === 'multi-step-navigation'
        ? { ...task, targetTokens: ['nav-branch'] }
        : task;
      const state = await observeStable(
        initialTask,
        task.optional ? 1_500 : observationTimeoutMs,
      );
      decisions += 1;
      await input.perform({
        kind: 'pause',
        durationMs: policy.decisionPause(task.id, index, {
          recovery: task.id === 'visible-challenge',
        }),
      });

      if (task.id === 'read-select') {
        await approachAndActivate(
          input,
          targetFor(state, 'choice-correct'),
          keyboardOnly,
          policy,
          { taskIndex: index },
        );
      } else if (task.id === 'form-correction') {
        if (keyboardOnly) {
          await input.perform({
            kind: 'keyStroke',
            key: 'Tab',
            modifiers: [],
            ...policy.keyboardNavigationTiming('Tab'),
          });
        } else {
          const form = targetFor(state, 'synthetic-form');
          await approachAndActivate(input, form, false, policy, { taskIndex: index });
        }
        await typeSyntheticCorrection(input, policy);
        recoveries += 1;
        if (!keyboardOnly) modalityChanges += 1;
      } else if (task.id === 'multi-step-navigation') {
        await approachAndActivate(
          input,
          targetFor(state, 'nav-branch'),
          keyboardOnly,
          policy,
          { taskIndex: index },
        );
        const returnTask = { ...task, targetTokens: ['nav-return'] };
        let next;
        for (let scan = 0; scan < 4; scan += 1) {
          next = await observeStable(returnTask, transitionProbeMs);
          if (hasRequiredTargets(next, returnTask)) break;
          if (scan === 3) break;
          if (keyboardOnly) {
            for (let count = 0; count < 3; count += 1) {
              await input.perform({
                kind: 'keyStroke',
                key: 'ArrowDown',
                modifiers: [],
                ...policy.keyboardNavigationTiming('ArrowDown', count),
              });
            }
          } else {
            await input.perform({ kind: 'scroll', dx: 0, dy: policy.scrollAmount(scan) });
          }
          await input.perform({
            kind: 'pause',
            durationMs: policy.decisionPause('multi-step-navigation', index),
          });
        }
        await approachAndActivate(
          input,
          targetFor(next, 'nav-return'),
          keyboardOnly,
          policy,
          { taskIndex: index },
        );
      } else if (task.id === 'idle-resume') {
        await input.perform({ kind: 'pause', durationMs: policy.idleBreak() });
        const resumed = await observeStable(task);
        await approachAndActivate(
          input,
          targetFor(resumed, 'resume-action'),
          keyboardOnly,
          policy,
          { taskIndex: index },
        );
      } else if (task.id === 'visible-challenge') {
        const challenge = targetFor(state, 'challenge-action', true);
        if (challenge) {
          visibleChallenges += 1;
          await approachAndActivate(input, challenge, keyboardOnly, policy, {
            purpose: 'challenge',
            taskIndex: index,
          });
        }
      } else {
        throw new ContractViolationError(`task is not in the canonical suite: ${task.id}`);
      }
      results.push({ id: task.id, status: 'PASS', elapsedMs: Date.now() - taskStarted });
    } catch (error) {
      if (task.optional && error instanceof ContractViolationError) {
        results.push({ id: task.id, status: 'NOT_PRESENT', elapsedMs: Date.now() - taskStarted });
        continue;
      }
      results.push({
        id: task.id,
        status: 'FAIL',
        elapsedMs: Date.now() - taskStarted,
        reason: error.message,
      });
      break;
    }
  }

  return Object.freeze({
    version: STRATEGY_VERSION,
    taskSuiteVersion: TASK_SUITE_VERSION,
    decisions,
    frames,
    recoveries,
    modalityChanges,
    visibleChallenges,
    domQueries,
    elapsedMs: Date.now() - started,
    tasks: Object.freeze(results),
    completed: results.length === tasks.length && results.every((task) => task.status !== 'FAIL'),
  });
}
