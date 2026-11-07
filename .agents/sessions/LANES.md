# Work lanes (parallelism catalog)

Machine authority for lane IDs and primary contexts is `lanes.json` (schema1).
The Markdown tables and existing PS/bash allowlists are compatibility projections;
the Rust maintenance drift check must compare all three. Primary paths describe
contexts, not a grant to overwrite another active owner's declared paths. Unknown
or duplicate catalog data fails closed. Legacy labels remain until coordinated
release/reclaim, never an automatic alias around an occupied context.

## Rust migration bounded contexts (2026-11-07)

These formal lanes supplement legacy lanes above. Existing owners migrate by normal release/claim; never force another owner or alias overlapping paths. Every owner also claims its own named SoT/plan/evidence. Shared Cargo/lock/toolchain remain root-owned. Machine catalog consolidation is assigned to agent-maintenance; legacy script literals are compatibility data only.

| Lane | Exclusive context |
|---|---|
| authority-ipc | crates/authority/** |
| audit-custody | crates/audit/** |
| native-cli | crates/cli/**; operational CLI only, excludes crates/native-cli/** |
| native-windows | scripts/ci/windows-native-runner/**, crates/test-support/**, crates/native-security/** |
| browser-ui | crates/browser/** and crates/native-cli/**; browser scaffold and consumer have one owner |
| product-lab | test/lab/** |
| adapter-hosts | crates/framework-adapter/**, crates/node-adapter/**, crates/python-adapter/**, crates/ruby-adapter/**, adapters/node/**, adapters/python/**, adapters/ruby/** |
| adapter-nginx | crates/nginx-adapter/**, adapters/nginx/** |
| project-coordination | root registry/coordination metadata, only after agents-meta owner agrees exact files |
| adapter-apache | crates/apache-adapter/**, adapters/apache/** |
| native-abi | crates/native-abi/**, adapters/native/** |
| adapter-go | crates/go-adapter/**, adapters/go/** |
| adapter-dotnet | crates/dotnet-adapter/**, adapters/dotnet/** |
| network-capture | crates/capture/**; existing crates/network/** read-only |

Native-security OS credential provider belongs to native-windows; CLI only selects/validates typed references, runtime consumes shared protected input. No duplicate OS secret reader. Cross-context API changes require explicit owner agreement. Scoring seams stay frozen.

Temporary root-approved compatibility loan: project-coordination is held by
codex/rust-performance-baseline for `.spec/sot/performance-optimization.sot.md`,
`.spec/plan/20261107135643-performance-optimization.plan.md`,
the exact public files `.spec/evidence/20261107135643-performance-optimization/index.evidence.md`
and `.spec/evidence/20261107135643-performance-optimization/baseline.json`, plus
`.spec.local/root/evidence/20261107135643-performance-optimization/**` only.
Root subsequently handed off exactly three additional files to this same owner:
`crates/integrity/src/request/canonical.rs`,
`crates/integrity/src/request/canonical/tests.rs` and
`crates/integrity/examples/canonical_performance.rs`. The actual claim now lists
these eight scopes. Root will not write those three source paths until returned.
This overrides the usual metadata role only for the recorded session; it grants
no whole-Integrity, .agents/rules/THREADS, graph or other benchmark path. Source
and formatting preparation do not permit workloads during the Pages heavy window;
compilation/benchmark execution still requires its own root allocation. Use
normal exact-owner handover/release; never take over this lane from its owner.

Temporary root-approved audit loan: red-catalog is held by
codex/rust-convention-audit for `.spec/sot/convention-audit.sot.md`,
`.spec/plan/20261107143000-convention-audit.plan.md`, the exact public files
`.spec/evidence/20261107143000-convention-audit/index.evidence.md` and
`.spec/evidence/20261107143000-convention-audit/baseline.json`, plus
`.spec.local/root/evidence/20261107143000-convention-audit/**` only.
The actual claim was re-read after normal owner acquisition. The lane's usual
red-code responsibility is not granted: no product/red code, rules, Pages, graph
or Cargo workload permission follows from this document/evidence loan.
The future convention-audit/performance Rust-only protocol remains draft.

Execution slots are separate from work lanes: root assigns at most four local
pipelines A/B/C/D with distinct targets, jobs1/incremental0 each. No automatic
start from a free slot or same-target overlap. Admit only at commit<85% and free
physical>20GiB; at commit>=88% or free physical<16GiB defer new compilation and
automatically reduce subsequent allocations. No CEF/native-heavy/bindgen overlap
and no native Docker/CEF/SDK heavy build alongside another cold compilation
stream. Four slots are only a ceiling; warm unit/bounded process overlap requires
explicit budget admission. Resume deferred heavy builds only after other cold
streams are terminal and commit<85%/free>20GiB. Preserve caches; no WSL shutdown,
global cache drop, Docker pruning or foreign-process termination for admission.
Root's program-roadmap/delivery canon owns
resource measurements and manifest windows; rule94 defines measured hybrid venue
selection. Supported local process/IPC/fixture checks still need allocation;
Windows Desktop act follows rule96. Required GitHub profiles and final matrix
remain distinct, and the measured2GiB profile is not a universal workload budget.
GPU inference still needs its separate bounded allocation. Lane ownership is not
permission to bypass execution allocations.


A **lane** is a coarse ownership slice. Agents claim one primary lane per session
(optional secondary read-only). Paths are guidance, not a complete ACL.

| Lane | Primary paths | May parallel with | Mutex / caution |
|------|---------------|-------------------|-----------------|
| `detection-core` | `internal/signals`, `internal/scoring`, `internal/network`, `internal/collector`, `internal/detect`, `cmd/wasm`, `cmd/server` (collect path) | `docs`, `agents-meta`, `pass` (if only `internal/pass`) | **Mutex with `gate-edge`** if changing scoring contracts |
| `gate-edge` | `internal/gate`, `internal/audit`, `cmd/gate`, admin/console | `docs`, `agents-meta` | **Mutex with `detection-core`** on shared score/audit APIs |
| `pass` | `internal/pass`, `web/pass.html`, pass handlers under `cmd/server` | `docs`, `agents-meta` | Avoid Pass + detection-core score-band changes together |
| `red-catalog` | `test/redteam`, `test/e2e`, `deployments/bots` | `docs`, `agents-meta` | Coordinate with `e2e-infra` on compose bots image |
| `e2e-infra` | `deployments/compose*`, `scripts/e2e*`, `scripts/assert-*`, `Makefile` e2e targets, `.github/workflows/ci.yml` (detector/Trivy), `codeql.yml` | `docs`, `agents-meta` | Coordinate with `red-catalog` |
| `docs` | `docs/**`, public README sections that are docs-only | most lanes | Mutex with itself; don't rewrite product claims without code lane |
| `agents-meta` | `AGENTS.md`, `.agents/**`, nested `**/AGENTS.md`, vendor adapter trees | most lanes | One writer for agent standard changes |
| `sot-plan` | `sots/**`, `plan/**` (local/private) | `docs` (after SoT accepted) | One writer per SoT file; SoT-38 is single-writer |
| `release` | version tags `v*`, `cliff.toml`, `.github/workflows/release.yml`, skills `cut-release` / `github-actions-ops` (release path) | — | **Exclusive**; coordinate with `git-ops` for the actual tag/push |
| `git-ops` | `.git` index/HEAD/refs, commits, pull/rebase/merge, push, stash, branch switch | none for writes | **Global mutex for git writes**; hold only for the transaction (TTL 30m) |
| `misc` | anything not listed | only if paths declared | Prefer naming a real lane |

## Nested sessions (same provider)

Multiple Claude (or Grok) chats on this repo still share the same `ACTIVE.md`.
Each chat needs its **own claim** (different lanes) or they must be sequential.
