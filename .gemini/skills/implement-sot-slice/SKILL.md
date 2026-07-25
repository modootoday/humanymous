---
name: implement-sot-slice
description: >
  Use when implementing an accepted SoT/plan slice into packages and tests:
  wire a feature, file decomposition, or vertical slice after design is ready.
  Honor detection freeze unless scoring change is in scope.
---

# Implement SoT slice

## Preconditions

- Accepted SoT/plan (or user-pasted design).
- Detection freeze (`.agents/rules/20-detection-freeze.md`) unless scoring is in scope.

## Steps

1. Map requirements → packages (`plan/01` patterns; see `internal/AGENTS.md`).
2. Prefer seams: `signals` → `scoring` → `collector`/`network` → handlers → `gate`/`audit`.
3. Pure logic + unit tests first; wire HTTP last.
4. `go test` on touched packages.
5. Any e2e / network / Gate / correlation → skill `red-blue-validate` (**Docker only**; `make e2e`). Host runners are not completion authority.

## Gotchas

- Gate must not fork scoring.
- New signal ids: registry + avoid silent string typos (resolution tests).
- WASM path: also build with `GOOS=js`.
- Per-plane premises (Core vs Gate) — see `.agents/lessons/HARD-WON.md` §1–2.
- If implementing SoT-38 work packages, re-read the B1–B6 table before “one-line” fixes.

## Done checklist

- [ ] Behavior covered by tests or red profile
- [ ] No always-on folklore dumped into root AGENTS.md
