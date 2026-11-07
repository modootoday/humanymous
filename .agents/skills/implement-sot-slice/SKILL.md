---
name: implement-sot-slice
description: >
  Use when implementing an accepted SoT/plan slice into packages and tests:
  wire a feature, file decomposition, or vertical slice after design is ready.
  Implement Rust 1.99.1/edition 2024 responsibilities with scoped tests and actual
  boundary pilots. Not for design-only requests or explanations without code changes.
---

# Implement SoT slice

## Preconditions

- Accepted SoT/plan (or user-pasted design).
- Detection freeze (`.agents/rules/20-detection-freeze.md`) unless scoring is in scope.
- **Lane claimed** (`coordinate-sessions` / `session-board claim`) if other LLM sessions may be active.

## Steps

1. Run `spec context <path>`; map the accepted `.spec/sot`/`.spec/plan` requirement to the current crate. Read `.agents/rules/10-rust-conventions.md`.
2. Prefer existing seams: signals → scoring → collector/network → runtime/authority/audit. Reuse one owning implementation.
3. Pure logic + unit tests first; wire HTTP last.
4. Run locked scoped Cargo tests, fmt and Clippy. Authored code/tests are Rust only; executable script strings and generated foreign templates are not a workaround.
5. Run an actual native/process/browser pilot for the touched boundary. Product e2e → `red-blue-validate` (**Docker only**). Mock/unsupported/component success is not product success; retain exact source/image and failed denominators.
6. Run `spec check` before the path-scoped attributed commit. Common Cargo/lock/toolchain changes go to root via INTEGRATION.md, not a concurrent writer.

## Gotchas

- Gate must not fork scoring.
- New signal ids: registry + avoid silent string typos (resolution tests).
- WASM path: check `humanymous-collector` for `wasm32-unknown-unknown` and run the isolated browser pilot; a native unit result does not establish browser behavior.
- Per-plane premises (Core vs Gate) — see `.agents/lessons/HARD-WON.md` §1–2.
- If implementing SoT-38 work packages, re-read the B1–B6 table before “one-line” fixes.

## Done checklist

- [ ] Behavior covered by tests or red profile
- [ ] No always-on folklore dumped into root AGENTS.md
