# AGENTS.md — `internal/`

Inherits root `AGENTS.md`. Overrides for the detection core packages.

## Ownership map

| Package | Role |
|---------|------|
| `signals` | Shared schema/registry (WASM + server); pure Go |
| `scoring` | L6/L7, hard rules, policy — **single** verdict authority |
| `network` | JA3/JA4/H2/header order (server TLS plane) |
| `collector` | Session merge client + network |
| `detect` | WASM L1–L3 (`//go:build js,wasm`) |
| `integrity` | RIT keys/tokens (shared canonical rules) |
| `gate` | Edge enforcement helpers (uses scoring, does not fork it) |
| `audit` | Tamper-evident chain, crypto-shred, sinks |
| `pass` / `pow` | Challenge mechanics |

## Rules

- Do **not** re-implement scoring inside `gate` or `cmd/*` handlers.
- New signals: register in `internal/signals/registry.go`; keep ids byte-stable unless intentional policy change.
- Prefer pure functions + package tests; golden fixtures for score changes.
- Detection freeze still applies (root hard constraints).

## Verify

```bash
go test ./internal/signals/ ./internal/scoring/ ./internal/network/ ...
```
