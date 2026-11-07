# Root integration

## Coordinated adapter-family naming handoff

The accepted internal target package name is `humanymous-adapter-go`; the public
Rust package/crate is `humanymous`, with Go access through
`humanymous::adapters::go`. The current internal package remains
`humanymous-go-adapter` until Root's single graph migration. No rename is
performed by this handoff, and the frozen c321 hosted receipt retains its
original names. Root approved optional feature `adapter-go` and exports
`Deployment`, `Host`, `Configuration`, `Rejection` at that public Go path. Root
still owns the machine-checked build-product manifest and facade wiring;
neither is qualified by this table.

| Surface | Exact migration impact and owner |
| --- | --- |
| Package and dependencies | Root coordinates the Go leaf package rename, public facade dependency/feature, lab consumer dependency and root Cargo.lock in one graph candidate. Lab consumes the public facade rather than exposing the renamed internal leaf as the public API. No standalone lock exists in test/lab/runtime at this checkpoint |
| Rust library identifier | No explicit `[lib].name` or crate-type is declared. The internal default identifier changes from `humanymous_go_adapter` to `humanymous_adapter_go`; public lab imports in go/client.rs, gate.rs, mod.rs and receipt.rs move together to `humanymous::adapters::go`. The leaf's tests/deployment_contract.rs external import must also change in that atomic batch |
| Build artifact | This package is an ordinary Rust library, not a cdylib/staticlib or executable host module. Its inferred rlib/rmeta target identity changes; Cargo regenerates those dependency artifacts. No Go-facing shared-library filename or exported C symbol is declared here |
| Consumer commands | `.github/workflows/go-validation.yml` scoped test/Clippy selectors and `scripts/ci/delivery/src/policy.rs` exact command expectations must change atomically. Historical evidence commands remain unchanged |
| Registry and ABI | `publish=false`; no crates.io publication, Go SDK, cgo binding, Python/Ruby module, Node export or public ABI symbol is renamed by this package change. The bounded HTTP receipt profile/schema and Go host labels are separate contracts, not Cargo names |
| Paths and claims | Current workspace member path `crates/go-adapter`, adapter resources `adapters/go`, lane adapter-go and its registry paths stay unchanged unless Root explicitly maps a directory move with M11. A package rename alone does not authorize a path migration |

The product recipe copies runtime, CLI, runtime-pilot and audit-witness binaries,
not a standalone Go adapter binary. Those consumer executable names must not be
mechanically renamed. Revalidate the new locked graph, scoped Go units and lint,
and affected actual runtime host integration before promoting a renamed tuple.
The prior c321/42375310198 result is historical source-bound evidence, not proof
that a future renamed artifact already passed. Root owns common graph/lock;
product-lab owns consumer imports, CI owns workflow/policy and M11 owns registry.

Requalification must exercise a real consumer of the public facade and the
actual hosted Docker eight-case runner at the renamed candidate. Retain the
exact new source, selected feature profile, artifact/image and independent
fixture/configuration/engine bindings, case/check counts, owner exit and cleanup.
Build-only and pure-test results remain separate from actual host execution.
Do not change the receipt schema/profile, host labels or prior evidence merely
to match a Cargo identity. Go remains an HTTP integration, without a Go SDK,
cgo library, shared-library loader or newly authored foreign implementation.

The authorized source predecessor now publicly re-exports Configuration and
Rejection from the runtime. External Rust compile-time signatures require the
same owning types without conversion; one new pure test checks all six host
families' distinct pooled-context/borrowed-entity premises. These checks are
authored, not executed: ten existing unit tests plus one new integration test
need a Root-assigned compiler slot. Direct Rust formatting and scoped Specfold
passed. This predecessor does not rename the package or qualify the facade.

Current accepted result: Rust Gate Go integration adapter, bounded net/http8,
source c3211e27ba941f3f11809e420d07d61ececad623, actual hosted run42375310198.
Evidence:root/20261107132453:8 evaluated/pass,40 checks,0failed/not-run/unknown,
owner0, receipt valid, source unchanged and owned cleanup observed. Same-source
Go10/lab20/delivery74 units, Unix actor compile and strict Clippy/policy passed.
Full product/release remain false. Public projection binds source/private ledger
hash, not individually exported runtime-image/fixture/configuration/engine values;
do not substitute the mount-probe image. Other host families/native SDK remain
unqualified. This is not legacy Go core work. Historical pending/quiet wording
below describes earlier checkpoints, not current Root-authorized execution.

Register crates/go-adapter as a workspace member and update Cargo.lock under the
root Cargo slot. Dependencies reuse runtime and serde workspace declarations.
No Go producer profile, new authority transport or unsafe/foreign glue is needed.

The pre-probe source passed three locked pure units and scoped all-target Clippy on
the explicitly assigned B target/node-native/build, jobs1/incremental0. B was
returned. Evidence:root/20261107075922 retains the exact scope and one concurrent
runtime test-file drift; it is not an immutable full-workspace result. Actual
execution follows Root's latest user-authorized admission policy. The current
benchmark quiet window forbids new workload commands, while this goal still
requires actual hosted Go eight-case evidence.

Commands used for that source, to rerun after later changes in an assigned slot:

```
cargo test --locked -p humanymous-go-adapter --lib
cargo fmt -p humanymous-go-adapter -- --check
cargo clippy --locked -p humanymous-go-adapter --all-targets -- -D warnings
```

Manifest/dependencies frozen at root request while workspace integration waits
for the current wargame Cargo slot. Direct rustfmt and owned spec checks passed.

The crate is a deployment contract and receipt qualifier, not a Go SDK or proxy.
No tuple is qualified. It transfers validated runtime Configuration into the
existing composition root. Receipt structs never execute tests or authenticate
evidence. Hosted integration must supply source/image/external-fixture digests
and independent origin contact counters for all eight cases, retaining failures.
Corpus version2 replaces misleading origin_effects with origin_contacts and
requires handler_response_verified for the two positive cases (false for all
negative cases). Contact counts alone cannot prove binary echo or HEAD/cookies.
Version1 receipts do not qualify; this corpus does not measure application
mutations, human/browser provenance or 204/304/streaming support.
ExpectedTuple additionally binds host/Go version, actual OS/architecture and
sanitized configuration/frozen-engine digests. Expected values must come from
the trusted runner independently of receipt contents. Empty or stale values fail.

Pinned untouched candidate sources are in adapters/go/upstream-fixtures.json.
Their entrypoint SHA256 values were computed from GitHub API decoded bytes;
they are not full fixture/dependency/build artifact identities. Most hello-world
examples cannot meet the full corpus. No upstream Go file was authored or copied
into this checkout, and no native Go host bridge is implied.

Public docs delta for docs owner: distinguish net/http/chi/Gin/Echo from pooled
Fiber/fasthttp; universal Gate deployment does not imply native method-set,
ResponseWriter/hijack/cancellation compatibility. No native Go support claim.

## Historical handover: execution authorization gate

Mission remains complete Go HTTP Gate integration, all named host contracts,
owned unit/lint results and actual hosted pilot/product evidence. No detection,
accessibility, fail-closed or T4 rule changed. Read AGENTS.md and
.agents/lessons/HARD-WON.md before resuming; this chat is Codex-owned.

Done: source307e9cc, contact-proof correction ddda924, hosted build recipe31d9f69,
English-only authored prose, direct Rust formatting, LF worktree/index and scoped
Specfold check. Root reported two units for the older307e9cc source; that is not
evidence for the current three-test corpus. Native SDK/foreign glue is absent.

At that checkpoint: no explicit Cargo slot for this owner and no live Go-specific
hosted execution handle/result. Root and lab chat activity alone does not prove
a Go compiler/test job is running. Local Cargo and actual local processes/Docker
were unauthorized at that checkpoint. The runtime pilot now declares the Go path dependency;
its Go execution path and source-bound result still need implementation/validation.
Do not count an unexecuted recipe or synthetic receipt as pilot completion.

Next: root assigns a current pure-test/lint slot or runs the exact commands above.
Then product-lab consumes Deployment in the real runtime construction and checks
Receipt against independently selected ExpectedTuple; build the untouched fixture
with compiler-input.json/Dockerfile.fixture on the free hosted runner, retain
full fixture/binary/image/config/engine hashes and all eight case denominators.
Keep synthetic session HTTP compatibility separate from browser provenance,
contact counters separate from mutations, and non-net/http tuples NOT_RUN until
their actual lifecycle corpus is complete. Root owns push/dispatch/integration.

Read first: .spec/sot/go-adapter.sot.md, owned timestamp plan,
adapters/go/pilot-contract.json and compiler-input.json. Reclaim adapter-go before
writing after handover; use the global git mutex for any attributed owned commit.
No changes outside the claimed lane or common Cargo/lock/toolchain are authorized.

User resumed work and root assigned B: current3units and all-target Clippy passed,
slot returned. This resolves the pure-check allocation gap, not actual Go hosted
execution. Continue with product-lab's real composition and independent witness;
that earlier allocation did not authorize local server/process/Docker execution
or root-only dispatch. The later user authorization update below supersedes
the blanket local restriction without granting a slot during the quiet window.

The new public probe module takes an already-decoded outer response. binary_dump
checks the pinned dumped request's exact method/target/entity; head_cookies checks
200/zero entity/two individual a=1,b=2 cookies. The lab owns actual process identity,
transport decoding, artifact hashes and isolation. These helpers do not mint
authority or authenticate evidence. Three added tests bring the source total to
six; that new source still needs the next assigned unit/lint slot.

Lab-requested scenario API now lives in scenario::{CaseObservation,evaluate,
EvaluationError,BINARY_TARGET,BINARY_ENTITY,HEAD_TARGET}, with crate-level CASES.
The DTO matches the lab's proposed field names, has no Debug/Serialize and no
default fact substitution. evaluate returns Outcome only for complete matching
measurements; errors retain static code/assertion count for the outer ledger.
Missing counter never becomes zero. Status422 additionally requires exact
required_capability JSON, incomplete400 requires measured write-half-close and
truncated_body JSON, upgrade400 requires http_framing JSON. Cancellation needs
pre-handoff plus observation past deadline; bypass needs both kernel-denied
paths, with neither case claiming an HTTP response. Four new tests bring the
total to ten; previous six-test source and older three-test result remain
separate. No manifest changes; freeze source until assigned validation/failure.

Current validation: c959562 passed all ten pure units and scoped all-target
Clippy-Dwarnings on root-assigned A, default target/jobs1/incremental0. A returned.
Evidence:root/20261107083740 records exact owned/graph hashes and zero drift in
the216-file source subset; newly transitive capture/challenge source was omitted,
so it is not a full immutable compiled closure. Keep this source frozen for
the lab's actual8case integration; only actual failures justify changes/reruns.
Remaining: actual hosted fixture/process/namespace/contact/eight-case evidence,
not further pure-test allocation. The8132117 run predates evaluatorc959562.

Execution authorization update: user permits actual local Windows/WSL comparison
against GitHub, replacing the earlier blanket hosted-only rule. Root's benchmark
quiet window currently forbids new Cargo/native/runtime/model/Docker commands;
this owner holds no live command or slot. Follow root's measured admission policy
after the quiet window. The Go goal still needs actual hosted eight-case proof;
local comparative results must never be labelled hosted or inherit hosted=true.
Lab caller fixes are checkpoint0836bbee; read its INTEGRATION before dispatch.

Local Docker canon: when root admits a local comparison, use
C:/Program Files/Docker/Docker/resources/bin/docker.exe with explicit
--context desktop-linux. Preflight and retain context/endpoint/server ID/version/
OSType; do not fall back to WSL Docker when PATH is missing. Act's SDK must target
that Desktop engine via explicit DOCKER_HOST. WSL native Rust is a different
execution profile and does not identify its Docker engine as Desktop. Hosted
Linux jobs are distinct. None of these local observations replaces actual
hosted Go qualification; strict Go hosted-context rejection remains unchanged.

Hosted checkpoint: run42365025258 at source438a3feea0ea2f5e305aa6e3816dc7b60a069bd6
passed Go10, lab15 and delivery64 pure tests, selected Unix Clippy and workflow
policy. Product job128381682745 failed at runtime_build before any scenario:
scheduled8, evaluated0, passed0, failed0, not_run8. Owned cleanup was observed,
the receipt validated and source_changed was false. Private ledger SHA256 was
fc21fa7271d03df286bc2ac84d56526ee54f4c49a8eed3901453b9ae70f7534b.
The actual run is https://github.com/modootoday/humanymous/actions/runs/42365025258;
lab checkpoint724a7dd2 preserves its source-bound negative evidence. This is
not completed hosted Go qualification.

Root decision8f8f5e07 separates the explicit bounded Go component requirement
from the unchanged full-product requirement. Lab99dc5606 owns
--require-go-component; exit0 requires exact eight verified real hosted cases,
artifact/source binding and owned cleanup, while full_product_qualified remains
false. Never accept exit2 as a bounded success. CI438a3fee consumes that mode.
Lab0ce4d82 replaces the deprecated relay atomic update API; its fixed snapshot
passed Unix all-target compilation and strict Clippy with2288 source hashes
unchanged. CI44876534 adds native actor preflight. Actual hosted Docker rerun
with these fixes is still required. Owned evaluator sourcec959562 is unchanged.
