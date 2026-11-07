# Legacy Go consumers and removal batches

This read-only inventory separates the old Go implementation from the desired
Rust Go-origin adapter. `legacy-go-inventory.json` lists all336 tracked `.go`
paths and SHA256 values at observed HEAD32914a0b; no Go paths were dirty at the
observation. Counts are cmd59, internal253, pkg2 and scripts22. This is source
inspection, not runtime parity, deletion authorization for this lane or a claim
that every consumer has been found through dynamic execution.

## Concrete consumers

| Consumer | Observed dependency | Required owner action |
|---|---|---|
| `.github/workflows/frozen-go-oracle.yml` | Dispatch-only workflow runs Go build/test/race/vulncheck and Go WASM build; its push-frequency comment is stale | Delivery retires the oracle job with its replacement/retirement evidence; do not infer active push triggers from comments |
| `.github/workflows/codeql.yml` | Go1.25 setup, Go CodeQL and autobuild on main/PR/schedule | Delivery/security owner removes or replaces the obsolete Go analysis profile |
| `.github/workflows/release.yml` images matrix | Selects `build/core.Dockerfile` and `build/gate.Dockerfile`; build-push uses matrix.dockerfile and push=true | Delivery/root replace these Go image inputs before any future publication; this inventory does not authorize a release |
| `build/core.Dockerfile`, `gate.Dockerfile`, `external-input-core.Dockerfile` | Compile cmd/server, cmd/gate and/or cmd/wasm using root go.mod/go.sum | Delivery/root migrate callers to qualified Rust image inputs, then retire old builders |
| `build/bots.Dockerfile` | Copies cmd/internal/pkg, builds redteam/tlsparrot/gate, executes demo-key and model generators | Delivery/product-lab retire the legacy bot/demo stack using actual Rust lab/wargame replacement results or explicit responsibility retirement |
| `Makefile`, legacy `deployments/compose/*.yaml` | Go build/test/training and old builder references | Root and deployment owner remove obsolete recipes and consumers in the same targeted batch |
| `scripts/migration/contracts/main.go` | Parses `internal/signals/signals.go` | Preserve current Rust report contracts and their tests; retire this one-time generator after owner review |
| `scripts/migration/oracle/*.go` | Imports old attestation/behavior/integrity/network/resource/scoring/signals and related packages | Preserve hashed generated fixture/provenance; detach regeneration from the shipping tree and retire generators |
| `scripts/migration/export-source-oracle.ps1` and `.go.txt` harnesses | Default copies cmd/server/h2monitor.go, executes go run and writes monitor.json; other named harnesses cover audit/merkle/keys | Owner reviews each input/output, retains immutable fixture hashes and historic source, then retires the regeneration path; `.go.txt` is not included in the336 count |
| `internal/gate/console.go` | go:embed console.html and styles/*.css | Runtime/browser owner reviews assets separately; removing `.go` is not authority to delete HTML/CSS |
| `scripts/assert-privacypass.mjs`, `assert-webauthn.mjs` | Error recipes refer to gen-demo-keys | Lab owner replaces/retires stale recipes; a textual recipe is not proof of a currently running generator |

The bounded Rust-source searches found no direct `.go` include_str/include_bytes
dependency. Rust parity tests already embed JSON: scoring engine/behavior/
crosschecks/derivation; network h2/monitor/reports/tls; integrity verification/
request/resource; signals observations/reports; audit checkpoints/records/merkle/
keys. Preserve these fixtures and actual Rust tests. Go-origin comments in
signals/report, scoring trust tests and audit ordering are provenance, not runtime
Go imports. This does not prove every generated value's semantic coverage.

## Targeted deletion proposals

Only the named path owner/root integration executes deletion after reviewing its
batch. Each proposal is independent: running chats or unrelated missing parity
do not indefinitely retain all336 files. Recheck references and exact contents
on the deletion candidate; preserve scoring freeze, a11y and fail-closed behavior.

| Batch from exact JSON paths | Count | Executor/review owner and concrete gate |
|---|---:|---|
| oracle-generators |20| Delivery/root plus affected Rust crate owners: preserve fixture bytes, source baseline and generator input/output provenance; execute affected Rust fixture tests and retire generator consumers |
| training-report-demo-tools |7| Root/scoring/lab: cmd/ml-train, mlcorrect-train, report and the two demo generators; replace actual required behavior in Rust or record reviewed retirement, then remove Makefile/bots/recipe consumers |
| legacy-red-lab |4| Product-lab/delivery/root: cmd/redteam and tlsparrot; qualify required owned-deployment Rust lab/wargame paths or explicitly retire unsupported responsibilities; detach bots builder |
| legacy-collector |9| Collector/root: cmd/wasm, internal/collector and detect; preserve JSON/report contracts and actual Rust WASM/browser evidence, migrate old builder/loader recipes |
| legacy-runtime-entrypoints |49| Runtime/CLI/root: cmd/server and gate; review authority/audit/enforcement/console responsibilities against actual Rust process/product profiles and remove old image/Makefile callers |
| package/internal/* and package/pkg/origincloak |247| One package per independently removable sub-batch in JSON, not a single all-or-nothing deletion: audit28 to audit owner; gate85 to runtime; network20 to root/network; scoring21/signals5/integrity6 and remaining packages to their Rust responsibility owners. Retire every inbound Go generator/entrypoint/import before removing each package |

The final package sub-batches must use the root responsibility registry's exact
file rows, not only the count table. At inspection the314 baseline product rows
were255 pending,54 ported-unit-not-runtime and5 partial-unit-not-runtime; none
of these labels proves completed runtime parity. For each reviewed file, root
updates the registry to verified_runtime_parity with actual evidence or
retired_with_review with a substantive disposition. Do not relabel existing unit
fixtures as actual runtime evidence. A retired responsibility need not acquire an
invented Rust feature just to keep an obsolete oracle alive.

`scripts/ci/delivery/src/registry.rs::migration` requires complete baseline rows
from commit981d25cf and ultimately no tracked `.go`, go.mod or go.sum. It reads
the baseline through Git history; it does not require current Go sources to
remain. Root can preserve historic oracle source by immutable commit references.
After targeted batches, root retires root go.mod/go.sum and obsolete Go shipping
inputs, rechecks the inventory, source-bound Rust gates and final matrix. Never
wait for the old Go shipping tree to keep passing after its accepted removal.

## Baseline responsibility review routing

`legacy-go-owner-routing.json` is a derived review queue for all314 baseline
responsibilities, not a replacement shipping registry. Every row preserves the
baseline path/hash, JSON row pointer, frozen test names/status, current hash,
existing Rust mapping, review owner, concrete remaining gate and explicit lack
of supplied runtime proof. All314 current hashes match the baseline. The22 extra
current Go files are the20 migration generators and2 demo generators, outside
that baseline's product responsibility rows.

| Review coordinator | Rows | Initial review scope |
|---|---:|---|
| audit-custody |28| Audit records, keys, verification and runtime custody |
| detection-core |49| Collector, signals/scoring, integrity and associated contracts |
| gate-edge with native-cli/authority-ipc |147| Server/Gate, enforcement, host lifecycle and support responsibilities |
| product-lab with root integration |4| Legacy redteam/tlsparrot responsibilities |
| root integration |23| Remaining challenge/report and cross-context responsibilities |
| resource-watermark design review |14| Legacy watermark inventory/design only; root acceptance before crate/graph/code |
| model-training-correction design review |29| mlserve/mltrain/mlcorrect and training command review only; no activation or inference allocation |
| root integration with network-capture |20| Network/TLS/H2 responsibilities and actual capture boundaries |

These are review routes, not new write claims. Each owner writes its own
INTEGRATION/SoT delta keyed by exact baseline paths. Root alone serially updates
the central314-row registry after review, preventing concurrent whole-file edits.
The user-approved watermark and model review chats are registered in THREADS.
Their observed misc and sot-plan claims cover only their distinct design/spec
artifacts. Delegating these43 rows does not qualify or authorize their deletion,
change detection scoring or approve a crate/graph implementation. The model
training/correction review is separate from developer-skill paired LLM evaluation.

The59 source-mapped rows have existing paths that all still exist, but their
recorded labels explicitly lack full runtime parity. The255 unmapped pending rows
must be classified by owners as stale mapping, missing implementation, partially
implemented responsibility or reviewed retirement. This static inventory cannot
honestly distinguish those semantic outcomes merely from an existing crate.
The queue therefore preserves uncertainty and supplies coarse existing Rust
review candidates rather than declaring blanket ported or unimplemented status.

For each reviewed row, supply the exact Rust file/module and responsibility,
exact test names and actual source-bound receipts, missing process/product gate,
or a substantive retirement disposition. An empty exact_runtime_proof list means
no proof was supplied by the baseline row; it does not assert that no evidence
exists elsewhere. Root must reconcile the owner's real receipts before changing
the shipping status. Existing fixture tests/source comments remain useful inputs,
never automatic runtime proof.

## Keep the Rust Go-origin adapter

Preserve `crates/go-adapter`, `adapters/go`, Rust lab Go backend/orchestration and
their owner contracts. The pinned external mccutchen/go-httpbin fixture is built
from an untouched upstream checkout, with its own compiler-input contract and
Go1.26 minimum. It does not consume the old humanymous cmd/internal/pkg tree or
root go.mod. Removing legacy core Go is not permission to delete this desired
adapter or its external interoperability witness. Likewise actionlint's Go-based
tool installation is distinct from compiling the retired product implementation.

No Go command, Docker build, runtime test or deletion was executed for this
inventory. The root owns final integration and serial Git operations; each
foreign path remains under its actual writer's authorization.
