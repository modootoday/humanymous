#!/usr/bin/env node
/**
 * Cross-provider pre-tool guard for coding agents.
 *
 * Reads a tool payload from stdin. Exit codes:
 *   0 — allow
 *   2 — block (reason on stderr)
 *
 * This is intentionally dependency-free so the same command runs on Windows
 * and Linux wherever the repository's Node-based agent tooling runs.
 */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

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
  if (!raw.trim()) return 0;

  let payload;
  try {
    payload = JSON.parse(raw);
  } catch {
    payload = { command: raw };
  }

  const command = extractCommand(payload);
  if (!command) return 0;

  const reason = evaluateCommand(command);
  if (!reason) return 0;

  process.stderr.write(`${reason}\n`);
  return 2;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main();
}
