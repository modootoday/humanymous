# AGENTS.md — `web/`

Inherits root `AGENTS.md`. Browser client assets + Pass UI.

## Rules

- Detection logic that must resist tampering belongs in WASM (`cmd/wasm` / `internal/detect`), not plain JS.
- JS is for load/inject/collect/transport; keep SRP files under `web/js/`.
- Pass UI (`pass.html`): keyboard + screen-reader operable; no motor-richness gate.
- Do not ship secrets or admin tokens in client assets.

## Verify

- WASM build: `GOOS=js GOARCH=wasm go build ./cmd/wasm/`
- Manual/browser paths as documented; e2e via test runners when touching Pass.
