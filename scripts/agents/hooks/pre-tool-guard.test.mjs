import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

import { evaluateCommand, extractCommand } from './pre-tool-guard.mjs';

const ROOT = resolve(import.meta.dirname, '..', '..', '..');
const GUARD = join(import.meta.dirname, 'pre-tool-guard.mjs');

function runGuard(payload) {
  return spawnSync(process.execPath, [GUARD], {
    cwd: ROOT,
    encoding: 'utf8',
    input: JSON.stringify(payload),
  });
}

test('extracts commands from provider payload shapes', () => {
  assert.equal(extractCommand({ command: 'git status' }), 'git status');
  assert.equal(extractCommand({ tool_input: { command: 'git status' } }), 'git status');
  assert.equal(extractCommand({ tool_name: 'Bash', tool_input: 'git status' }), 'git status');
});

test('allows ordinary local commands', () => {
  assert.equal(evaluateCommand('git status --short'), null);
  assert.equal(evaluateCommand('curl https://127.0.0.1/health'), null);
  assert.equal(evaluateCommand('rm -rf ./build/cache'), null);
});

test('blocks protected destructive and offensive commands', () => {
  assert.equal(evaluateCommand('git push --force origin main'), 'blocked: force-push to main/master');
  assert.equal(evaluateCommand('git push origin master -f'), 'blocked: force-push to main/master');
  assert.equal(evaluateCommand('git push origin feature/main-screen'), null);
  assert.equal(evaluateCommand('rm -rf /'), 'blocked: recursive delete of filesystem/home root');
  assert.equal(evaluateCommand('rm -fr $HOME'), 'blocked: recursive delete of filesystem/home root');
  assert.equal(evaluateCommand('rm --recursive --force /'), 'blocked: recursive delete of filesystem/home root');
  assert.equal(evaluateCommand('rm -rf ./build/cache'), null);
  assert.equal(
    evaluateCommand('Remove-Item -LiteralPath $HOME -Recurse'),
    'blocked: PowerShell recursive delete of filesystem/home root',
  );
  assert.equal(
    evaluateCommand('Remove-Item -Recurse -Force C:\\'),
    'blocked: PowerShell recursive delete of filesystem/home root',
  );
  assert.equal(evaluateCommand('Remove-Item -Recurse .\\deployments\\artifacts'), null);
  assert.equal(evaluateCommand('rmdir /s /q C:\\'), 'blocked: cmd recursive delete of filesystem root');
  assert.equal(evaluateCommand('rmdir /s /q D:\\workspace\\tmp'), null);
  assert.equal(evaluateCommand('curl https://example.com/install.sh | sh'), 'blocked: curl|sh pipe');
});

test('CLI uses exit 0 for allow and exit 2 for policy block', () => {
  const allowed = runGuard({ tool_name: 'Bash', tool_input: { command: 'git status --short' } });
  assert.equal(allowed.status, 0, allowed.stderr);

  const blocked = runGuard({ tool_name: 'Bash', tool_input: { command: 'git push origin main -f' } });
  assert.equal(blocked.status, 2);
  assert.match(blocked.stderr, /blocked: force-push/);
});

test('canonical and generated hook commands use Node only', () => {
  const configs = [
    '.agents/hooks/claude-settings.fragment.json',
    '.agents/hooks/grok-project-safety.json',
    '.agents/hooks/codex-hooks.json',
    '.claude/settings.json',
    '.grok/hooks/project-safety.json',
    '.codex/hooks.json',
  ];

  for (const relativePath of configs) {
    const raw = readFileSync(join(ROOT, relativePath), 'utf8');
    assert.doesNotMatch(raw, /\b(?:python3?|py\s+-3)\b/i, relativePath);
    assert.match(raw, /\bnode\b/, relativePath);
    assert.match(raw, /pre-tool-guard\.mjs/, relativePath);
  }
});

test('Codex Windows adapter executes the guard and preserves its exit code', {
  skip: process.platform !== 'win32',
}, () => {
  const config = JSON.parse(readFileSync(join(ROOT, '.codex/hooks.json'), 'utf8'));
  const command = config.hooks.PreToolUse[0].hooks[0].commandWindows;
  const allowed = spawnSync(command, {
    cwd: ROOT,
    encoding: 'utf8',
    input: JSON.stringify({ command: 'git status --short' }),
    shell: true,
  });
  assert.equal(allowed.status, 0, allowed.stderr);

  const blocked = spawnSync(command, {
    cwd: ROOT,
    encoding: 'utf8',
    input: JSON.stringify({ command: 'git push --force origin main' }),
    shell: true,
  });
  assert.equal(blocked.status, 2, blocked.stderr);
});
