# Work lanes (parallelism catalog)

A **lane** is a coarse ownership slice. Agents claim one primary lane per session
(optional secondary read-only). Paths are guidance, not a complete ACL.

| Lane | Primary paths | May parallel with | Mutex / caution |
|------|---------------|-------------------|-----------------|
| `detection-core` | `internal/signals`, `internal/scoring`, `internal/network`, `internal/collector`, `internal/detect`, `cmd/wasm`, `cmd/server` (collect path) | `docs`, `agents-meta`, `pass` (if only `internal/pass`) | **Mutex with `gate-edge`** if changing scoring contracts |
| `gate-edge` | `internal/gate`, `internal/audit`, `cmd/gate`, admin/console | `docs`, `agents-meta` | **Mutex with `detection-core`** on shared score/audit APIs |
| `pass` | `internal/pass`, `web/pass.html`, pass handlers under `cmd/server` | `docs`, `agents-meta` | Avoid Pass + detection-core score-band changes together |
| `red-catalog` | `test/redteam`, `test/e2e`, `deployments/bots` | `docs`, `agents-meta` | Coordinate with `e2e-infra` on compose bots image |
| `e2e-infra` | `deployments/compose*`, `scripts/e2e*`, `scripts/assert-*`, `Makefile` e2e targets, CI detector job | `docs`, `agents-meta` | Coordinate with `red-catalog` |
| `docs` | `docs/**`, public README sections that are docs-only | most lanes | Mutex with itself; don't rewrite product claims without code lane |
| `agents-meta` | `AGENTS.md`, `.agents/**`, nested `**/AGENTS.md`, vendor adapter trees | most lanes | One writer for agent standard changes |
| `sot-plan` | `sots/**`, `plan/**` (local/private) | `docs` (after SoT accepted) | One writer per SoT file; SoT-38 is single-writer |
| `release` | version tags, `cliff.toml`, release workflow, cut-release | — | **Exclusive**; coordinate with `git-ops` for the actual tag/push |
| `git-ops` | `.git` index/HEAD/refs, commits, pull/rebase/merge, push, stash, branch switch | none for writes | **Global mutex for git writes**; hold only for the transaction (TTL 30m) |
| `misc` | anything not listed | only if paths declared | Prefer naming a real lane |

## Nested sessions (same provider)

Multiple Claude (or Grok) chats on this repo still share the same `ACTIVE.md`.
Each chat needs its **own claim** (different lanes) or they must be sequential.
