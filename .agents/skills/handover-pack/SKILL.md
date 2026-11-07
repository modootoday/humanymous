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

If detection intentionally changed, state it loudly. Use the owning `.spec/plan` and INTEGRATION.md; retain raw source-bound evidence privately under `.spec.local` and sanitized public attachments. Distinguish Rust unit/component, actual pilot, full Docker product e2e, mock, ignored and unsupported results. New implementation/test code remains Rust 1.99.1/edition 2024.

## Multi-provider / nested sessions

- Point the next agent at `.agents/lessons/HARD-WON.md` and `AGENTS.md`, not only Claude memory.
- **Write a handover block onto the session board** (`.agents/sessions/ACTIVE.md`) and
  `release` the lane via `session-board` so overlapping sessions see the state.
- If work spanned Claude/Grok/Codex/Gemini, note which provider holds residual chat state
  (skill `survey-provider-history` / `coordinate-sessions`).
- Never put MCP tokens or OAuth material in the handover.
