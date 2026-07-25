# AGENTS.md — `cmd/gate`

Inherits root `AGENTS.md`. Edge reverse-proxy binary.

## Scope

- TLS termination (BYO/ACME/dev), route presets, streaming inject, enforcement before origin.
- Admin plane is a **separate trust domain** (authenticated listener, RBAC, dual-control).
- Shares `collector.Store` + `scoring.Engine` with core — no local verdict fork.

## Rules

- Prefer fail-closed on strict/mutating routes when signals missing.
- Audit-before-effect patterns stay structural (`EmitAndAct` style).
- Do not log raw IP/UA when pseudonymized paths exist.
- Topology honesty: full L5 JA3/JA4 often requires raw TLS on **core** (`cmd/server`), not always gate.

## Verify

- Unit: `go test ./cmd/gate/ ./internal/gate/ ./internal/audit/`
- Conformance: gate-e2e (34) via skill `red-blue-validate` / CI `detector-vs-bots`
