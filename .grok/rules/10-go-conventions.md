# Go & layout conventions

- Follow golang-standards/project-layout: `cmd/`, `internal/`, `pkg/`, `web/`, `test/`.
- Keep `internal/signals` pure Go (no platform deps) — shared WASM + server types.
- Prefer small files with one responsibility group (see `plan/01-implementation-plan.md`).
- Scoring core stays pure/testable; do not fork scoring logic into `internal/gate`.
- `cmd/server` and `internal/gate` share `collector.Store` + `scoring.Engine` — never re-implement verdicts in Gate.
- Run `go vet` and `go test` on touched packages before claiming done.
