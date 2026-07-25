# AGENTS.md — `test/`

Inherits root `AGENTS.md`. Self-validation only.

## E2E policy (non-negotiable)

- **Authoritative e2e is Docker-only** via `deployments/compose.yaml` and `scripts/e2e-docker.sh` / `make e2e`.
- Host commands (`node test/e2e/runner.mjs`, `node test/gate/e2e.mjs` against a local binary) are for profile authoring / debugging — **not** completion authority.
- Lab networks are `internal: true`; bots cannot reach the public internet.

## Rules

- **Defense-only**: profiles target the compose stack / own deployment only.
- One red profile ≈ one automation class under `test/redteam/*.mjs`.
- Human baseline is first-class; designs that DENY baseline have not improved.
- Catalog entry point in CI/bots image: `deployments/bots/run-attack.sh` → `e2e/runner.mjs` with `HM_BASE=https://core:8443`.
- Keep launcher ↔ catalog lists in parity (launch parity tests).

## Skills

- Full Docker e2e / pre-merge measure: `red-blue-validate`
- Sequential multi-round red→Docker→blue series: `red-blue-wargame-round` (rule `61`)
- Pass challenge rounds only: `pass-wargame-round` (SoT-36; Docker pass-test profile for live engine)

## New Core profile checklist

1. `test/redteam/<name>.mjs` exports `run`, `label`, `needsBrowser`
2. Register in `test/e2e/runner.mjs` `PROFILES`
3. Register in `cmd/server/launch.go` `launchProfiles`
4. Register in `web/playground.html`
5. Prove with Docker `make attack` + `make e2e-assert` (not host runner alone)
