#!/usr/bin/env node
/**
 * Dependency-free cross-provider PreToolUse guard.
 *
 * stdin: provider hook payload JSON
 * allow: no output, exit 0
 * block: stderr reason, exit 2
 */
import { createHash } from 'node:crypto';
import { appendFileSync, mkdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
export const AUDIT_LOG =
  process.env.HUMANYMOUS_HOOK_LOG || join(ROOT, '.agent-runs', 'hooks', 'pre-tool-guard.jsonl');

const BLOCK_PATTERNS = [
  [/\bcurl\s+.*\|\s*(?:ba)?sh\b/i, 'blocked: curl|sh pipe'],
  [
    /\bnmap\s+(?:-[a-z0-9]+\s+){0,6}(?!127\.|localhost|10\.|192\.168\.|172\.(?:1[6-9]|2[0-9]|3[0-1])\.)\d/i,
    'blocked: nmap against non-local target',
  ],
  [
    /\b(?:sqlmap|hydra|nikto)\b.*https?:\/\/(?!127\.|localhost|\[::1\])/i,
    'blocked: offensive tool against non-local URL',
  ],
];

const PROTECTED_BRANCH = /(?:^|[^\w/-])(?:main|master)(?![\w/-])/i;
const FORCE_FLAG = /(?:^|\s)(?:--force(?:-with-lease)?|-f)(?=$|\s)/i;
const ROOT_TARGET =
  /(?:^|\s)(?:--?\w+\s+)*["']?(?:\/|~|\$home|\$env:(?:userprofile|homedrive)|[a-z]:[\\/]+)["']?(?=$|\s)/i;

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function audit(raw, payload, command, reason) {
  const entry = {
    version: 1,
    timestamp: new Date().toISOString(),
    pid: process.pid,
    ppid: process.ppid,
    hookEvent: typeof payload?.hook_event_name === 'string' ? payload.hook_event_name : null,
    toolName: typeof payload?.tool_name === 'string' ? payload.tool_name : null,
    inputBytes: Buffer.byteLength(raw),
    inputSha256: sha256(raw),
    commandBytes: Buffer.byteLength(command),
    commandSha256: command ? sha256(command) : null,
    decision: reason ? 'block' : 'allow',
    rule: reason,
    exitCode: reason ? 2 : 0,
    stdoutBytes: 0,
    stderrBytes: reason ? Buffer.byteLength(`${reason}\n`) : 0,
  };

  try {
    mkdirSync(dirname(AUDIT_LOG), { recursive: true });
    appendFileSync(AUDIT_LOG, `${JSON.stringify(entry)}\n`, 'utf8');
  } catch {
    // Observability must never change the safety decision or hook output.
  }
}

export function extractCommand(payload) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) return '';

  for (const key of ['command', 'cmd', 'shell_command']) {
    if (typeof payload[key] === 'string') return payload[key];
  }

  const toolInput = payload.tool_input ?? payload.input ?? {};
  if (toolInput && typeof toolInput === 'object' && !Array.isArray(toolInput)) {
    for (const key of ['command', 'cmd', 'shell_command']) {
      if (typeof toolInput[key] === 'string') return toolInput[key];
    }
  }

  if (('tool_name' in payload || 'tool' in payload) && typeof payload.tool_input === 'string') {
    return payload.tool_input;
  }
  return '';
}

export function evaluateCommand(command) {
  const normalized = String(command).trim().split(/\s+/).join(' ');
  const lower = normalized.toLowerCase();

  if (/\bgit\s+push\b/.test(lower) && FORCE_FLAG.test(normalized) && PROTECTED_BRANCH.test(normalized)) {
    return 'blocked: force-push to main/master';
  }

  if (/\brm\b/.test(lower)) {
    const recursive = /(?:^|\s)(?:-[a-z]*r[a-z]*|--recursive)(?=$|\s)/i.test(lower);
    const force = /(?:^|\s)(?:-[a-z]*f[a-z]*|--force)(?=$|\s)/i.test(lower);
    if (recursive && force && ROOT_TARGET.test(normalized)) {
      return 'blocked: recursive delete of filesystem/home root';
    }
  }

  if (/\bremove-item\b/.test(lower)) {
    const recursive = /(?:^|\s)-(?:recurse|r)(?=$|\s)/i.test(lower);
    if (recursive && ROOT_TARGET.test(normalized)) {
      return 'blocked: PowerShell recursive delete of filesystem/home root';
    }
  }

  if (/(?:^|[^\w-])(?:rd|rmdir)\b/.test(lower)) {
    const recursive = /(?:^|\s)\/s(?=$|\s)/i.test(lower);
    if (recursive && ROOT_TARGET.test(normalized)) {
      return 'blocked: cmd recursive delete of filesystem root';
    }
  }

  for (const [pattern, reason] of BLOCK_PATTERNS) {
    if (pattern.test(normalized)) return reason;
  }
  return null;
}

export function main(raw = readFileSync(0, 'utf8')) {
  let payload;
  if (raw.trim()) {
    try {
      payload = JSON.parse(raw);
    } catch {
      payload = { command: raw };
    }
  } else {
    payload = {};
  }

  const command = extractCommand(payload);
  const reason = command ? evaluateCommand(command) : null;
  audit(raw, payload, command, reason);
  if (!reason) return 0;

  process.stderr.write(`${reason}\n`);
  return 2;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main();
}
