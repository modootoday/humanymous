---
name: handover-pack
description: >
  Use when writing a self-contained handover for the next LLM or human: mission,
  state, verify commands, and frontier. Prefer for session handoff or multi-session work.
when_to_use: >
  "handover", "next LLM", "session handoff", "continue later", "brief the next agent".
---

# Handover pack

No prior chat assumed. Structure in `assets/handover-template.md`.

## Required

Mission · Done (+ evidence) · In progress · Next actions · Verify commands · Files to read · Out-of-scope.

If detection intentionally changed, state it loudly. Prefer updating `plan/*` handover when in tree.

## Multi-provider

- Point the next agent at `.agents/lessons/HARD-WON.md` and `AGENTS.md`, not only Claude memory.
- If work spanned Claude/Grok/Codex/Gemini, note which provider holds residual session state
  (skill `survey-provider-history`).
- Never put MCP tokens or OAuth material in the handover.
