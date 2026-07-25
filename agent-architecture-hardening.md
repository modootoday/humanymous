# Agent architecture hardening plan

## Goal

Make the multi-provider agent architecture executable and fail-closed while keeping every authoritative E2E check inside Docker and making no paid LLM API calls.

## Source of truth for this handoff

- Claude → Grok: `plan/08-sot38-implementation-handover.md` (SoT-38 v1.2.0 implementation sequence)
- Codex WIP folded here: Docker-native attack/Pass/swarm/overlay asserts

## Tasks

- [x] Make the attack catalog fail on profile errors, skips, missing baseline, or escaped bots. (`scripts/assert-attack.mjs` + unit tests)
- [x] Fix the watermark-strip profile to use the operator-authenticated trace contract.
- [x] Move attack assertions, Pass suites, swarm assertions, and overlay assertions into Docker services.
- [x] Give each E2E run an isolated Compose project and deterministic cleanup.
- [x] Codex project auto-approve (lower-risk still preferred long-term; see `.agents/lessons/CODEX-AUTO-APPROVE.md`) — committed earlier; optional: tighten off danger-full-access later.
- [ ] Extend the guard for PowerShell/cmd destructive roots and force-push argument order; add semantic tests.
- [ ] Add a no-LLM workflow state machine with schema, journal, retries, approvals, and local tests.
- [ ] Execute deterministic skill-trigger evaluations and workflow validation in layout verification.
- [ ] Fail CI when adapter synchronization would change tracked generated files.
- [ ] Run the full Docker-only red/blue suite end-to-end on a clean machine (long); static checks + compose config done locally.
- [x] Session overlap + git-ops contention protocols (Grok; commits `78c9004` / board tools).

## Verification

`node --test scripts/assert-attack.test.mjs`, `docker compose … config -q`, agent layout verifiers, and `bash scripts/e2e-docker.sh` (authoritative). No host Node E2E assert is completion authority.

## Notes for implementers

- Do **not** ship P0-4 scoring one-liners without reading `plan/08` + Gate loader `internal/gate/control.go`.
- Concurrent Codex/Claude/Grok writes: use `session-board` + `git-coord` before further commits.
