#!/usr/bin/env python3
"""Cross-provider pre-tool guard for coding agents.

Reads JSON on stdin (tool payload). Exit codes:
  0 — allow
  2 — block (message on stderr; Claude PreToolUse convention)

Blocks high-risk patterns: recursive delete of roots, third-party offensive
scanning framed as external targets, force-push to main/master.
Defense-only redteam against localhost/compose remains allowed.
"""
from __future__ import annotations

import json
import re
import sys

# Commands that should never run without explicit human override outside this guard.
BLOCK_PATTERNS = [
    (re.compile(r"\bcurl\s+.*\|\s*(ba)?sh\b"), "blocked: curl|sh pipe"),
    # Obvious third-party offensive framing (not local compose/localhost).
    (re.compile(r"\bnmap\s+(-[a-zA-Z0-9]+\s+){0,6}(?!127\.|localhost|10\.|192\.168\.|172\.(1[6-9]|2[0-9]|3[0-1])\.)\d"), "blocked: nmap against non-local target"),
    (re.compile(r"\b(sqlmap|hydra|nikto)\b.*https?://(?!127\.|localhost|\[::1\])", re.I), "blocked: offensive tool against non-local URL"),
]

PROTECTED_BRANCH = re.compile(r"(?<![\w/-])(main|master)(?![\w/-])", re.I)
FORCE_FLAG = re.compile(r"(?<!\S)(--force(?:-with-lease)?|-f)(?!\S)", re.I)
ROOT_TARGET = re.compile(
    r"""(?ix)
    (?<!\S)(?:--?\w+\s+)*
    ["']?
    (
      / |
      ~ |
      \$home |
      \$env:(?:userprofile|homedrive) |
      [a-z]:[\\/]+
    )
    ["']?(?!\S)
    """
)


def extract_command(payload: dict) -> str:
    # Claude / Grok-style shapes
    for key in ("command", "cmd", "shell_command"):
        if key in payload and isinstance(payload[key], str):
            return payload[key]
    tool_input = payload.get("tool_input") or payload.get("input") or {}
    if isinstance(tool_input, dict):
        for key in ("command", "cmd", "shell_command"):
            if key in tool_input and isinstance(tool_input[key], str):
                return tool_input[key]
    # Nested tool_name + tool_input
    if "tool_name" in payload or "tool" in payload:
        ti = payload.get("tool_input") or {}
        if isinstance(ti, str):
            return ti
    return json.dumps(payload)


def evaluate_command(command: str) -> str | None:
    """Return a stable blocking reason, or None when the command is allowed."""
    command = " ".join(command.strip().split())
    lower = command.lower()

    if re.search(r"\bgit\s+push\b", lower) and FORCE_FLAG.search(command) and PROTECTED_BRANCH.search(command):
        return "blocked: force-push to main/master"

    if re.search(r"\brm\b", lower):
        has_recursive = bool(re.search(r"(?<!\S)-[a-z]*r[a-z]*(?!\S)|(?<!\S)--recursive(?!\S)", lower))
        has_force = bool(re.search(r"(?<!\S)-[a-z]*f[a-z]*(?!\S)|(?<!\S)--force(?!\S)", lower))
        if has_recursive and has_force and ROOT_TARGET.search(command):
            return "blocked: recursive delete of filesystem/home root"

    if re.search(r"\bremove-item\b", lower):
        has_recursive = bool(re.search(r"(?<!\S)-(?:recurse|r)(?!\S)", lower))
        if has_recursive and ROOT_TARGET.search(command):
            return "blocked: PowerShell recursive delete of filesystem/home root"

    if re.search(r"(?<![\w-])(?:rd|rmdir)\b", lower):
        has_recursive = bool(re.search(r"(?<!\S)/(?:s)(?!\S)", lower))
        if has_recursive and ROOT_TARGET.search(command):
            return "blocked: cmd recursive delete of filesystem root"

    for pattern, reason in BLOCK_PATTERNS:
        if pattern.search(command):
            return reason
    return None


def main() -> int:
    raw = sys.stdin.read()
    if not raw.strip():
        return 0
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        # Non-JSON stdin: treat whole line as command string
        payload = {"command": raw}

    cmd = extract_command(payload if isinstance(payload, dict) else {})
    if not cmd:
        return 0

    reason = evaluate_command(cmd)
    if reason:
        sys.stderr.write(reason + "\n")
        # Claude/Grok convention; Codex treats a failed PreToolUse hook as a guardrail.
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
