#!/usr/bin/env python3
"""Pre-tool guard for coding agents (Claude Code / Grok Build compatible).

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
    (re.compile(r"\brm\s+(-[a-zA-Z]*r[a-zA-Z]*f|-[a-zA-Z]*f[a-zA-Z]*r)\s+[/\\]?\s*$"), "blocked: recursive delete of filesystem root"),
    (re.compile(r"\brm\s+(-[a-zA-Z]*r[a-zA-Z]*f|-[a-zA-Z]*f[a-zA-Z]*r)\s+/($|\s)"), "blocked: rm -rf /"),
    (re.compile(r"\bgit\s+push\s+.*--force.*\b(main|master)\b"), "blocked: force-push to main/master"),
    (re.compile(r"\bcurl\s+.*\|\s*(ba)?sh\b"), "blocked: curl|sh pipe"),
    # Obvious third-party offensive framing (not local compose/localhost).
    (re.compile(r"\bnmap\s+(-[a-zA-Z0-9]+\s+){0,6}(?!127\.|localhost|10\.|192\.168\.|172\.(1[6-9]|2[0-9]|3[0-1])\.)\d"), "blocked: nmap against non-local target"),
    (re.compile(r"\b(sqlmap|hydra|nikto)\b.*https?://(?!127\.|localhost|\[::1\])", re.I), "blocked: offensive tool against non-local URL"),
]


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

    for pattern, reason in BLOCK_PATTERNS:
        if pattern.search(cmd):
            sys.stderr.write(reason + "\n")
            # Claude PreToolUse: exit 2 blocks
            return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
