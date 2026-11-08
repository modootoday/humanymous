---
id: plan:root/20261106222418
title: Rust wargame engine simulation
status: active
created: 2026-11-06T22:24:18Z
updated: 2026-11-07T07:17:45Z
rel:
  governs: [crates/attacker]
  depends_on: [sot:root/rust-wargame]
---

# Rust wargame engine simulation

## Goal

### Final-layout role/accounting negative candidate (not executed)

Private evidence20261108024500 pins four current final-layout Root assembly
files and three producer/builder assumptions. It contains a real four-file Rust
patch (SHA256e1a101ed53019c1211f6349dc229315c03e9e77da0d75a0f6c36557184611044)
and source ZIP (SHA2568aebeaf42f3be80c705a74e0d3cfc37a6ddac26fb1a8b7c1f0b7c2d361fb65df).
No live source/Cargo/Root snapshot change or compiler/workload was performed;
branch codex/rust was checked before writes. Four preimages remained unchanged.

Canonical case process counts follow existing execution:1ordinary child,
2forRestart. The candidate rejects excessive counts and missing receipts for
evaluated/attempted-step rows while preserving pre-step failure prefixes. This
is receipt-plan consistency, not process authentication or a scoring predicate.
Two new verifier tests cover missing/inflated/truncated receipts and meaningful
accepted failed controls, including corrected case/class totals. One old fixture
receives its missing receipt before adding steps; no test is dropped. A report
test verifies ignored ordinal validation cannot be bypassed by terminal accounting
and valid ordinals remain aggregate-neutral after storage removal.

Static source-local inventory17existing+3new=20, removed0. Evidence parsing and
patch applicability pass; red-before/green-after is NOT executed or claimed.
Root-only scoped pure verifier/report test/check/Clippy allocation is requested,
without any new heavy/native/hosted run. Immutable producer evidence and all
open native-publication/OS-census/Unix-producer gates are preserved.

### Tools role-composition Rust candidate (evidence-only, not integrated)

Follow-up Windows portable-test scope candidate is registered in the SAME private
evidence as windows-scope.patch (SHA256
33ecba96c7b6661f33ddb4431161633f2371da0c5fdbdf305c145708479c39ed) and
windows-scope-rust.zip (SHA256
a678cfbad186021904197f489c6924f7f376b4f4d38b7074d37f37e02654bf8f), with new
two-file input pins. It cfg(unix)-scopes only digest/OwnedRuntime::spawn/
VerifiedSnapshot::read and their exclusive imports. Keep portable type/readiness/
rollback tests and runtime root cfg(any(unix,test)) links; non-Unix production
fallback remains exit2. No blanket allowances, fake calls or dropped IDs.
Source-local two test names remain; full role/OS case census is still required.
Root reported actual isolated adoption and checks, but this follow-up author ran
only evidence parsing/patch checks, not compiler/Clippy/tests. Original artifacts
remain pinned to their original input; do not reapply them to an evolved assembly.

Root requested actual Rust candidate diffs after isolated14bin checks showed
private full-context composition warnings. Private evidence20261108013000 holds
candidate.patch,37proposed Rust sources in candidate-rust.zip, exact26Root input
pins, file-map and static test identity inventory. Root assembly
tools-integration-5f1010d4 was never edited; checkpoint is
5f1010d42f4938ed2ea30ce2e84945af7aedb180. Before/after source trees are review
artifacts only, not per-role implementation copies. Patch SHA256:
ba89226f27b48c8cc2e91ce67fc3f9cac00f647fa17d53ee865cb36d50d0e8c3;
Rust ZIP SHA256:
bad68875ff804177babdccd358d6686644c8ad8df296c41a3e7bbc32ee405de0.

Candidate isolates replay/evaluation, network contract/data verification, actual
runtime actor and legacy-only report. Sole physical status/shuffle/evaluation/
error/data sources are extracted; case contract and actual execution separate,
with a shared canonical builder. Offline verification tests use a cfg(test)
failed ledger fixture instead of linking the process actor. All40existing test
function identities and81ordered catalog quoted literals remain; actual actor
and provider tests stay in the runtime role. Native ignored gates remain open.

Private VerifiedSnapshot fields/Log construction stay provider-only; serialized
observation data is not authenticated authority. No blanket allowance, dummy
call, public SDK helper, implementation clone, Cargo/feature change or live source
edit. Root's corrected report command/root remains legacy-only. Candidate removes
unused report reexports and validates then discards the unused internal run
ordinal; one assertion in the same test is adjusted. Root must explicitly review
that storage narrowing and role-test discovery, not infer semantic qualification.

Only evidence Rust parsing/formatting and patch applicability were checked.
Compiler/Clippy/tests/new warnings were NOT measured. Root's26input hashes stayed
unchanged. The patch applies to the pinned before tree without modifying it.
Root must integrate in a fresh coherent source and check every role/feature/test
closure under strict warnings, retaining source-specific native/Docker gates.
Verifier feature inheritance may still enable runtime dependencies independently
of module selection; Root owns that graph decision. This artifact is ready for
integration review, not a claim that61warnings have actually been eliminated.

### Tools freeze contract: shared qualification lab support (source audit only)

Root's requested destination is CLI src/qualification/lab_support/{attacker,
defender}. This is an input contract, not a move or graph implementation.
Read-only source checkpoint27f6070207e6b77dda3834e34fb416ac0c746e21 pins all21Rust
files below. Both source trees were clean at inspection. Current active ownership
is the exact attacker-file loan and defender/src/** loan recorded in pass;
unclaimed attacker files below are inventory inputs, not additional write grants.
Root owns physical relocation, package/target/features and source-link selectors.

| Role | Current physical source inputs, relative to src/ |
| --- | --- |
| attacker shared logic | lib.rs; catalog.rs; report.rs; verification.rs; verification/json.rs; network/mod.rs; network/cases.rs; network/report.rs; report/legacy/mod.rs; report/legacy/aggregate.rs; report/legacy/html.rs; report/legacy/tests.rs |
| attacker CLI roles | main.rs; bin/humanymous-wargame-verify.rs; bin/humanymous-report.rs |
| defender shared logic | lib.rs; lab/mod.rs; lab/client.rs; lab/isolation.rs; lab/process.rs; lab/snapshot.rs |

Same physical module source may be linked into explicitly assigned role targets;
do not copy its implementation, retain public SDK helper packages or expose the
owned-process capability as a product SDK. Attacker's cross-root references to
humanymous_defender and three CLI references to humanymous_attacker must be
resolved by the root-selected module/target contract. They must not create an
old-package alias or duplicate decoder/scoring implementation. Actual Lab runtime
test/lab/runtime/src/wargame.rs consumes network::OwnedLab/run, verification and
defender lab types; only Root coordinates that consumer with the Lab owner.

#### Data readers and path bases

- Neither attacker nor defender source currently contains CARGO_MANIFEST_DIR,
  current_dir, env! or option_env!. Moving/linking under CLI changes any future
  manifest-derived base to CLI, not the old package or repository root; no such
  fallback should be invented. Existing include_str resolves relative to its
  physical Rust file. Recalculate paths once for the actual relocation, checking
  all eight references/six unique fixture bytes, not runtime CWD.
- catalog.rs embeds six frozen scoring JSONs; network/cases.rs embeds two of the
  same six. The owner is Core scoring, actual location is
  crates/sdk/humanymous-core/tests/scoring/fixtures/frozen. The preceding raw/Git
  table pins them independently; keep exact pre-move worktree bytes and canonical
  Git identity distinct. No authored fixture replacement or private payload copy.
- Legacy load reads an explicit input Path bounded to8MiB+1; paths/caption are
  bounded4096bytes,50krows/top128 and rendered16MiB. CLI defaults
  test/e2e/results.json and docs/report.html are CWD-relative historical options,
  not embedded data or permission to write repo docs during a pilot. Explicit
  isolated fixture/output paths are required for actual qualification.
- Legacy write_atomic uses create_new own sibling temporary,32collisions,
  write/sync/rename, and cleans only its created temporary; denied replacement
  retains the existing output. Temp tests use PID-scoped OS temp directories and
  create_dir ownership checks. Preserve those native tests and their ignore flags.
- Verify CLI reads an explicit report path with4MiB+1 cap, emits verification only
  after the sole platform::testing::json strict decoder, and returns2 for scoped/
  full rejection. Its parser must never accept a target URL. Replay CLI accepts
  only optional u64 seed and cannot acquire network mode through arguments.
- Defender process.rs hashes regular installed runtime/supervisor files with a
  256MiB bound and locates fixed sibling humanymous-runtime from current_exe's
  canonical parent, not CWD/Cargo manifest. It clears child environment, sends
  bounded private configuration bytes and checks complete runtime-owned readiness.
  Root's Runtime/CLI binary placement must satisfy this actual executable contract
  or explicitly version it; a linked module must not silently change the target.
- Linux isolation reads /proc/self/net/dev, route, ipv6_route and own/child network
  namespace links; process.rs reads /proc/PID/exe for image identity. Only loopback,
  no external route and the same namespace are admitted. Other OSes reject this
  network mode. Network client reads only the retained owned child's socket with
  bounded framing/deadlines, not arbitrary supplied addresses.
- VerifiedSnapshot reads an owner-provided authenticated Log, projects redacted
  effects and rejects prefix rollback. The actual installation's WAL, witness
  key/state and counted origin are Lab-owned inputs; they cannot be manufactured
  by pure tests or selected through a new public helper API.

#### Role test inventory and actual-boundary mapping

These are statically inspected inventories, NOT new executed counts. Preserve
module-qualified identities and target/cfg differences when linking shared source.

| Existing role | Inventory | Actual-provider obligation |
| --- | --- | --- |
| attacker library replay/catalog/report | 1seed reproduction+1catalog+3report tests | Pure actual frozen Engine replay only, not socket/product success |
| attacker network library | 1case-plan+4accounting/transport/verifier tests | Unavailable/synthetic failed ledger fixtures remain pure; actual19programs+7unsupported require the separate Lab supervisor |
| attacker verification library | 6semantic/duplicate/schema/trace tests+1safe JSON diagnostic | Preserve strict exact-byte verifier budgets, both schemas and independent total recount |
| legacy report library | 8pure parity/privacy/bounds/escaping tests;1ignored portable publication/collision;1ignored Windows sharing denial | Real owned files, replacement retention and cleanup remain native gates; Windows case is cfg(windows), not a Linux zero-test success |
| replay CLI main.rs | 1argument rejection test | Actual CLI/process/output pilot remains separate |
| verify CLI | 1argument/mode rejection test | Actual bounded original-file read/exit/stdout pilot remains separate |
| report CLI | 1argument/default test | Previously compiled, not executed in report L1 receipt; actual generation/file publication remains open |
| defender library | 2Engine+2client+2isolation+1readiness+1snapshot tests;extra non-Linux isolation rejection test | 8onLinux/9onnonLinux static cfg inventory; pure unit pass does not call OwnedRuntime::spawn or prove actual OS process isolation |
| Lab runtime wargame role | actual Installation+fixedseed42+sole raw-byte verification, plus setup-failure unit | Docker network-none supervisor, provisioned custody, authenticated WAL, counted origin, sibling runtime, real child cleanup and image receipt |

The21source files must not accidentally have their unit inventories discovered in
every role target and then summed as distinct coverage. Root chooses explicit
role target inclusion and preserves per-role test identity/ignored/cfg accounting.
Do not erase a required inventory to reduce duplicate execution; record shared
implementation with distinct role admission where appropriate. This source audit
ran no compiler, formatter, CLI, process, Docker or other workload. Historical306
is the scoped actual19/7 proof; none of it qualifies the prospective Tools layout.
No moves/source/Cargo edits or new public SDK helpers were made. No live handles
or target leases exist; exact Root destination/linking freeze remains the next
integration dependency, while native report qualification remains an open gate.

### RUNTIME-SLICE-02 assigned defender consumers (source only)

No defender/src writer existed in the actual claims inventory. Root's explicit
grant was recorded by normal own release/reclaim adding only crates/defender/src/**;
existing attacker loans remain unchanged. Root's dirty defender Cargo.toml was
preserved and is excluded from this source slice.

First, six old runtime root references now select gateway configuration/service/
http/session paths. Then one old audit reference now selects runtime::audit.
Four defender lab files changed; attacker Rust files have no matching references
and needed no artificial edit. No authority/network/capture/native-security crate
reference remains in these consumer sources. No alias or new policy was created.

| Changed current path | Source-ready SHA256 |
| --- | --- |
| crates/defender/src/lab/process.rs | 0d3948566ac481aee9f1b32ca4733e2b2982a99bb762bed760052260a9a4f16e |
| crates/defender/src/lab/mod.rs | cd8d10726fcf78b5b53a8b838cc720d50a6681903f3b1e4a5a584b67d32cb404 |
| crates/defender/src/lab/client.rs | e1f6b48982e33633a783a124084208d2091744f5cb501dba749e5325b78057f9 |
| crates/defender/src/lab/snapshot.rs | cc0be297ce234b7f3548a0a4a89189920ceec49546b062157d9657a4b702f19f |

These are seven mechanical reference changes, preserving existing exports,
complete readiness equality, canonical request signing, authenticated Log
recovery, fixtures, process ownership and behavior. No source path was moved;
new Runtime sources, all manifests/lock and six-context relocation remain root
owned. Whitespace and scoped spec checks pass. Formatter/compiler/unit/Clippy/CI
were not run during transition; no execution target or process handle is owned.
Historical306 and report L1 proofs remain source-specific. New coherent graph
qualification requires root allocation; these imports alone do not qualify it.

### PLATFORM-SLICE-02 assigned decoder consumers (source only)

Root assigned verification/json.rs and report/legacy/mod.rs. Normal own-lane
release/reclaim added exactly verification/json.rs; legacy/mod.rs was already
owned. Both imports now consume humanymous_platform::testing::json instead of
humanymous_test_support::json. No windows-object-bridge reference exists in
attacker Rust files and no old test-support identifier remains there.

| Changed current path | Source-ready SHA256 |
| --- | --- |
| crates/attacker/src/verification/json.rs | a2942231d6d1d29045261d7d0de1636f1a6f29099eb353bb5b908e37087cd475 |
| crates/attacker/src/report/legacy/mod.rs | 57d1b16589a0a3a1e00c5c8ca345f4060afedfc7ad965cd0b6c8cfdf33684f6a |

Exactly two import lines changed. StrictJsonError mapping, 4MiB verification and
8MiB legacy bounds, duplicate-key semantics, fixtures, visibility and behavior
remain unchanged. No manifests/features/dependency categories or paths moved;
Root owns all28 frozen-file moves and shared graph edits. Whitespace/spec checks
pass. No formatter/Cargo compiler/unit/Clippy/global CI ran during the transient
graph. Scoped Cargo fmt and the new source-bound gates wait for root coherence
and execution allocation. Historical pre-platform receipts do not qualify this
new dependency graph. No compatibility aliases or decoder copies were created.

### CORE-SLICE-01 assigned consumer references (source only)

Root later supplied the actual fixture destination:
crates/sdk/humanymous-core/tests/scoring/fixtures/**. Updated eight include_str
references across catalog.rs and network/cases.rs without copying fixtures.
Current source hashes after fmt: catalog.rs
f3c7c65c60d57efa2a45dceb3820245adbb5db35cd92d588bd8d0d767c7498d7;
network/cases.rs
52ded3c3a94ba24caff7a5c81ab893516a4b063b45bea90eddccf186f7f04a8d.

The initial cross-representation equality check found different bytes between
canonical306 Git blobs and moved CRLF worktree fixtures. It did NOT demonstrate
migration drift: the pre-move worktree was already CRLF. The initial inference
that root must restore LF was incorrect and is withdrawn. Human-chrome-allow is
5524canonical Git bytes versus5742pre/post worktree bytes; CR counts for the six
consumed fixtures are218/140/120/209/224/202. No fixture normalization is authorized.

Independent follow-up reads root's retained private20261108054000-core-consolidation/
file-map.before.json: all six consumed files match their PRE-MOVE rawSHA256 and
byte lengths, with0differences. Separately, filtered Git hashes match original306
blobs for all six. Preserve both provenance representations without conflating
them; no fixture was edited or copied by this consumer. Root reports all26moved
non-Rust files stable; this consumer independently checked its six-file subset.
Direct fmt and whitespace checks pass; no consumer compiler/Clippy/tests ran.

| Consumed frozen filename | Pre/post bytes | Equal raw SHA256 |
| --- | --- | --- |
| human-chrome-allow.json | 5742 | dbd103e638f27095c68f9a7b5be447b2df74600a6ce9deb5ed5377b83ac2e963 |
| go-http-client.json | 3549 | 5fd5941b45ee6a43a113a91c132f5928a93bdaedc5f1fb447e13e3eca11a0725 |
| no-client-hr10.json | 3066 | 87538d48a61fb2f4f99321e9c168b01264501b61bfec077073ec70da0c224ee2 |
| ua-tls-h2-hr2.json | 5367 | 3a8474c8e3de9dd33d56a470ec79a20743b184699401c6311720f2af38be2236 |
| selenium-hr1.json | 5789 | 368253a8ef8f58bb580d7318e41dd00e2152194ddb276086963bf225bc2a9418 |
| bot-webdriver-cdp-hr9.json | 5209 | 04eb9bd2a02d70ea1dd7796c8128ef03b2cc10f41bcafb97d91c61873ef264b2 |

Root explicitly assigned attacker Rust consumer references including catalog and
network. The existing pass loan was normally released/reclaimed to add exactly
src/catalog.rs and src/network/cases.rs; report.rs was already owned. Six Rust
references changed from humanymous_signals to humanymous_core::signals across
these three files, with rustfmt-only import ordering/line wrapping. No alias,
behavior, scoring, visibility, manifest or path move changed. Search finds no
remaining references to the seven old crate identifiers in attacker Rust files.

| Changed current path | Source-ready SHA256 |
| --- | --- |
| crates/attacker/src/report.rs | 2ca5cc61265049ef9177f366353de8fce32f996a4c48f247398243fbc07d6689 |
| crates/attacker/src/catalog.rs | a8dd6e0107fcac207ade26339a116678ff30fc928b421a45a38ceafc1c7df891 |
| crates/attacker/src/network/cases.rs | 15dafef79ce9dd1dd03e863d0d77e7bab1a1ca0c1e56b3eb5e03e3cf0ac25471 |

Direct fmt check and diff whitespace check pass. An initial transient Windows
mapped-file write refusal1224 prevented formatting; a later ordinary retry
succeeded without terminating processes or replacing ownership. No compiler,
Clippy, tests or CI ran: Root must first declare the shared workspace coherent.
Root owns manifests/lock and the seven-context relocation. Relative scoring
fixture include_str paths are unchanged pending the root's concrete fixture
relocation contract; their bytes and resolution must be checked in integration.
Historical hosted306 and reportc529 proofs do not qualify this new graph.

### Eight-package integration handoff (2026-11-08 Asia/Seoul)

Accepted topology baseline v0 is root-owned structural-development. This handoff
does not choose a destination package, expand visibility, authorize a move or
qualify the future graph. Read checkpoint8affb45aeb4d31330fda8f2e5135043c6b252c05:
the six owned Rust files below and owning SoT/plan were clean before this
handoff-only plan edit. Report source commitc5294b5d and generic receipt proof
8149d641 are preserved. There are no live compiler/process handles or target
leases; validation-authority was returned at terminal. No move is in flight.

The pass/rust-report-migration loan grants these six source files, the owning
SoT/plan and its exact named report/hosted evidence bundles. It grants no broad
attacker/defender directory, manifest, fixture, test-support, runtime, CI or lab
path. Root must freeze and transfer the specific source slice before moving it.

| Owned source path | Exact current SHA256 |
| --- | --- |
| crates/attacker/src/report.rs | c387c7c00155107eea711d2fd45041672649e8ed3da9d5bb1f6454c06cc74562 |
| crates/attacker/src/report/legacy/mod.rs | 76f3c7921834a658254c1a8013cdcb8149868a885784d2faf1bd8d08a11eb40d |
| crates/attacker/src/report/legacy/aggregate.rs | 7db9a4094bca3ba2742078e7962a2bd8adb88a9b3683449adc03295c9fc6660b |
| crates/attacker/src/report/legacy/html.rs | ebde9669e4d44ba70d0562f52d2b89c4b412e3d2c1512d34058b46d140d5a720 |
| crates/attacker/src/report/legacy/tests.rs | 922ed14d182f320b14382514c1f38f705261ae4ef6801b9563ebcd312fd4045e |
| crates/attacker/src/bin/humanymous-report.rs | a23096cd699f82384fabddfe1e79dae7ca2cc601452635569bcb0a0d8836d199 |

Read-only manifest anchors, not write grants: attacker Cargo.toml SHA256
9d73631194e2f66ec136fce68ed3227107029667b7883ca8ba5136242cf8e55d;
defender Cargo.toml SHA256
828862fea623f02cf2214c252ea4b2464ba173ac46b102bc9d8f37b817fbfd2e.

Integration hazards and consumer obligations:

- Both existing packages are publish=false. The accepted family must absorb
  responsibilities rather than publish these historical helper packages. Root
  decides final placement and feature/API admission; no speculative shim follows.
- Pure legacy decode/aggregate/render currently share a package with filesystem
  publication and process/network labs. Preserve their boundary when selecting
  features/targets: the report CLI is a maintenance tool, not scoring authority,
  and OwnedLab/OwnedRuntime are owned-deployment test capabilities, not a remote
  attacker or production-installation API. Keep WASM collector free of native
  process/file lab dependencies.
- Existing public Rust structs/modules support lab/tool consumers. Public package
  status is not permission to expose every module. Record intentionally omits
  Debug/Serialize for raw reason/error; keep redaction, closed strict JSON and
  unknown unsupported accounting. No unsafe block is present in this owned slice;
  do not move platform/FFI exceptions into it or broaden their permissions.
- Attacker depends on defender/test-support/signals/serde/serde_json. Defender
  reaches runtime/scoring/Integrity/audit and crypto dependencies. Consolidation
  must avoid a core-to-runtime/tool dependency cycle and reuse the sole strict
  decoder and sole frozen scoring engine. Check Rust visibility after merging;
  pub(crate) shuffle/catalog helpers must not silently become public contracts.
- catalog.rs and network/cases.rs use relative include_str scoring fixture paths.
  The lab runtime consumes attacker network/verification APIs; CI and the lab own
  the public receipt contract and independent file hashes. Preserve fixture raw
  bytes, seed42 ordering, schemas/exit codes and source/image binding across moves.
  Those paths belong to their owners and require explicit root handoffs.

Required acceptance after a root-directed move: scoped locked pure tests,
fmt/check/strict all-target Clippy on the new package/features, correct fixture
resolution and negative strict-byte/accounting tests; execute the report CLI unit
that was only compiled. Report8pure/2ignored/17filtered is historical evidence154600,
not proof of the new graph. Actual CLI publication, 32-collision retention and
Windows sharing-denial tests remain unrun and need a bounded native allocation.
The generic26 receipt at source306eedc8 is a completed historical scoped proof
(19evaluated/7unsupported/0errors/skips/mismatches), not qualification of a moved
graph. CI/lab must bind any required post-move actual run to its new source and
preserve full-product blocked status until missing capabilities are qualified.

Lowest ready unmet task remains the bounded report CLI/native file boundary
qualification, unless root first freezes this slice for integration. No current
path move, new workload, branch change, push or graph edit is requested here.

Provide a private, owned-lab-only Rust observation simulation using the existing
verdict engine, with honest unavailable-runtime and human-control accounting.

## Steps

- [x] V0.9: root accepted exact four legacy report baseline rows and six source paths, one module wire and auto-bin; SoT records compatibility/honesty/bounds/atomic-output contract before code.
- [x] V0.9: implement legacy import/aggregate/deterministic escaped bounded HTML and thin CLI, using existing dependencies only.
- [x] V0.9: allocated Windows pure parity/security/accounting8pass/0fail/2ignored/17filtered; package all-target check/strict Clippy/fmt pass, evidence154600. Basee34cee8a plus eight overlays;2715source files0differences, no registry/tool/config closure claim.
- [ ] V0.9: execute the compiled CLI unit and actual CLI publication/Windows replacement/failure qualification under a new bounded allocation. Native ignored cases remain NOT_RUN.

- [x] Audit accepted attack/ordinary/human/AT scope:19owned programs and7unsupported participant/producer obligations remain visible.
- [x] V0.8 source: reject transport-as-evaluated, attempted-as-skipped, post-failure steps and response verdicts without completed responses.
- [x] V0.8 root-assigned28pure/strictClippy pass;120155private/120401public records actual dependency graph and2doc-only drifts.
- [x] V0.8 actual hosted306eedc8/run42387564172 strict-byte receipt:26scheduled/19evaluated/7unsupported/0errors/skips/mismatches, scoped true/full false, evidence154100. Outer required gate remains blocked; no Docker allocation retained.

- [x] V0.6: approved shared decoder consumption; remove duplicate visitor, preserve report budget/errors.
- [x] V0.6: root lock integration and new allocated28pure/Clippy;13-crate snapshot and3file drift retained in075313/075756.
- [x] V0.6: shared decoder f44b32b blob matches captured/current source and measured shared2pure checks.
- [x] V0.6: actual813/run42355168729 receipt recovered; source/rawhash/26indices/totals checked in evidence083903. Result0evaluated/19errors/7unsupported/scopedfalse.
- [x] Repair ServiceHost/defender readiness through the runtime-owned builder and exercise coherent hosted306eedc8;19evaluated expectations match. The original813failure remains preserved.
- [x] Consume runtime709b0e2 standalone_profile with full equality and core-only/witness/unknown-field/scope rejection fixtures; source-only pending new validation.
- [x] Root-allocated27pure/Clippy passed for743494c1/runtime709b0e2, with ordered captured source indices stable; evidence084733/085126.
- [x] Coherent306eedc8 hosted candidate actually executed; public strict receipt independently recounted with source/raw/Lab-manifest binding. Private original body remains unavailable locally.
- [x] Preserve actual candidate a26d1c3/run42354873001 startup_failure with0jobs in evidence081913; no measured case ledger.

- [x] V0.5: bind network step identities/order/roles to catalog; preserve error prefixes.
- [x] V0.5: allocated26pure/Clippy passed; evidence073022/073058, dirty dependency scope retained.
- [ ] V0.5: source-bound hosted acceptance and private raw recovery.
- [ ] Link actual hosted serialized network bytes to strict verification evidence via lab owner.

- [x] Claim red-catalog/rust-wargame; inspect test taxonomy and sole Rust engine.
- [x] Write normative scope, serial persona synthesis and adversarial blockers.
- [x] Implement non-publishable attacker/defender contexts and seeded CLI ledger.
- [x] Test actual engine results, errors, missing coverage and human/ceiling split.
- [x] Run unit tests, Clippy and a network-none Docker engine pilot twice.
- [x] Independently verify ledger accounting and identical-seed bytes.
- [x] Root integrated members/lock; current scoped tests run in actual workspace.
- [ ] Full Rust Gate/browser/challenge runtime, signed installation binding and
  internal-topology escape negative tests. These block network-runner enablement.

## Record

## Owned-process continuation (current; not full scope completion)

- [x] Read roadmap1.2/current source; coordinate sequential lane with Compose.
- [x] Preserve accepted Compose/runtime API, ownership and schema in INTEGRATION.
- [x] Implement fixed sibling/private-pipe child, namespace/liveness guard,
  bounded native client and existing canonical/token calls in defender.
- [x] Implement actual-Log prefix-validating snapshots and attacker OwnedLab.
- [x] Define19network cases+7unavailable producers and Rust JSON verifier.
- [x] Scoped19unit/Clippy passed; record working-source private/public checkpoint.
- [x] Checkpoint1ee2c92/release to Compose for actual context/image integration.
- [x] Actual54017 source:26scheduled19evaluated7unsupported, no errors/mismatches;
  independent Rust verifier passed, strict full request exit2.
- [x] Compose60627 source:71outer assertions, same19/7matrix, cleanup/strict2;
  component witness proof remains separate from runtime fusion and human/AT.
- [x] Reclaim after Compose and author final guard/replay source corrections.
- [ ] Validate current source and run a new actual Docker/collector/audit pilot;
  preserve every failed/retried source result separately.
- [ ] Verify current code after integration, sanitize/attach summary, validate
  LF/Git bytes, commit owned paths and hand off unresolved producer requirements.

Compose owns shared custody/config/origin extraction, OwnedLab implementation,
image and Docker invocation. Runtime owns Pass/browser recovery and remote
attestation. Human/AT results require actual participants. Root owns Cargo/lock/
toolchain and global qualification. No component green closes seven unsupported
rows. Accepted status: checkpoint Windows unit19passed and source-bound Docker
network19/7 passed. V0.3 was uncompiled at47499a5; current v0.4 pure validation is
recorded above and hosted requalification remains pending. Common manifest freeze is respected after root's
direct-edge refresh. Initial --locked refusal, missing Deserialize compile error,
and unused-import/collapsible-if Clippy failures were observed and corrected.

## Current memory-priority handover and source delta

### Strict report follow-up after47499a5 — current source

Pages review identified v1 duplicate loss during Value decoding and v2 permissive
unknown fields. Root authorized correction within attacker/defender scope. V0.4
adds a report-specific recursive Serde visitor that rejects decoded duplicate
keys before constructing Value, plus unknown-field rejection on all7owned network
DTOs. V1 still compares the entire current replay; number/ordering/whitespace
semantic compatibility is intentional, not a canonical-byte/JCS claim. Source
hash authenticity, full WAL signatures, actual network and real participants
are not established by internal-consistency JSON verification alone.

- [x] Confirm source issue and preserve the distinction from actual63f69.
- [x] Implement strict decoded-key uniqueness and fixed DTO fieldsets.
- [x] Author4pure tests for root/nested/escaped/dynamic duplicates, unknown fields
  at all owned DTO levels and valid failed-ledger acceptance without runtime mocks.
- [x] Format owned source and perform diff whitespace checks.
- [x] Root allocated Cargo1slot; actual25pure passed on first attempt,0failed/
 0ignored with locked/-j1/test-threads1/incremental0; scoped Clippy passed.
- [ ] Record actual results and a separate owned checkpoint; do not attach old
  green or hosted execution to this later source.

No dependency, scoring, shared runtime wire, local runtime/Docker or dispatch
changes. Existing first-run and source-only v0.3 records below are historical,
not proof that the current strict decoder has compiled. Root received the exact
25pure-test inventory. Actual25passed=14attacker lib+2CLI+9defender lib; zero-test
doctest harnesses are separate. No socket/process scenario or runtime/Docker/
browser/SCM test executed. Working-source base6733128bebf393a8873db7298681381fbb7a4c14
plus owned patch/new parser copy and dirty status are in private bundle
`.spec.local/root/evidence/20261107070124-rust-wargame-strict-report-pure-validation/`.
This is scoped working-tree validation, not immutable full dependency-graph or
hosted proof. Public summary is [[evidence:root/20261107070333]]. Cargo slot was
returned immediately after unit/Clippy; runtime remains next hosted batch.

### Hosted validation policy — accepted, single common canon

Root received all13owner opinions and confirmed the user's policy. The sole
policy authority is [[sot:root/program-roadmap]]; the execution/DAG implementation
contract is [[sot:root/delivery]]. The prior worker proposal is superseded,
including suggested native matrix parallelism. No independent policy or workflow
implementation is maintained here; no required gate is removed.

Wargame local scope: coding/fmt, limited Clippy and explicitly classified pure
Engine/replay/ledger/CLI tests under common resource allocation. --lib alone does
not establish purity. Actual OwnedRuntime spawn, TCP/IPC/UID/permissions, custody/
WAL/restart, origin/audit/egress, Docker and browser/OS qualification belong to
standard hosted execution. Human/AT/IME/hardware and independent-organization
witness proof require direct evidence, never a runner label. All7unavailable
rows and full-product=false remain.

V0.3 was uncompiled at its checkpoint; the later v0.4 pure run is recorded above.
Root integrates
source closure before rust-branch push and first hosted observation. Prior green
remains source-specific. No duplicate dispatch, private raw public upload, paid
provisioning or source/gate deletion is authorized here. Missing required raw
recovery stays explicit and cannot silently qualify the associated gate.

Root explicitly excludes this v0.3 draft from the first hosted candidate that
contains runtime67c1818. After root's first-candidate Git transaction completes,
the wargame owner may create a separate path-scoped not_run checkpoint. Its code
must be tested only as a later source/batch after runtime/CI results are collected.
Do not assign the first run's SHA, results or artifact identities to this draft.

Root reported first push transaction complete at
bfc9f62335ae41906996c6b01b0ae9e7993feeed, hosted run42347732666/attempt1
on Ubuntu24.04 policy in progress. That source excludes v0.3. Root authorized a
separate source-only not_run checkpoint after releasing the Git mutex; no local
pure-unit rerun is required before handing this draft to later hosted validation.
This owner does not push or dispatch while root observes the first run.

Root relayed the user's memory-pressure instruction: no new cargo/rustc/build,
browser, Docker, spec-global or agents until resource scheduling changes. No own
active PID/container/session requires retention; previous helper/build commands
and engine-pilot containers exited. A terminated helper executable remains only
as a file in OS temp. Never kill another chat, the desktop apps or all WSL.

Compose released red-catalog; rust-wargame-final-boundaries reclaimed only its
declared paths. Peer deployments/lab,test/lab,container SoT/plan/evidence remain
unmodified. Source writes permitted by root added strict IPv4 evidence-header
validation, fallible reader spawning, combined error retention and Rust v1 replay
verification. Two new tests plus strengthened route negatives are authored but
unrun. Local owned-source fmt completed after the accepted policy change;
scoped tests/Clippy, LF/Git bytes, hosted Docker/collector and fresh
image/Cargo/secret scans are pending. Do not attribute checkpoint19unit or old
Docker successes to this dirty source. No current-source completion is claimed.

Failure lineage supplied by Compose:54017 success artifacts remain intact.
55200 first witness setup failure lost its dirty source graph after an accidental
same-folder retry; remaining failure report/transcript/image are55537 failure-*.
55200 now identifies a separate empty-attach retry failure.60036 retains a later
EOF/timeout failure.60627 is a distinct new successful source snapshot. Later
host report/parser edits are unvalidated. Do not overwrite or retroactively
qualify any of these records. See deployments/lab/INTEGRATION.md for exact owner
contracts. Further heavy execution awaits root, not elapsed-time approval.

Historical v0.1 Rust1.88 results below are not current-source qualification.

2026-11-07 Asia/Seoul: isolated snapshot passed 8 unit tests and Clippy with
warnings denied. Docker build uses Rust 1.88; both identical-seed executions
produced 12 scheduled cases: 8 evaluated, 4 unsupported, zero errors/skips or
regression mismatches. Evaluated outcomes: ALLOW 2, CHALLENGE 3, DENY 3. The
human baseline ALLOW and T4 ceiling ALLOW are distinct; long-window zero-motor
accessibility is existing HR-12 CHALLENGE. `human_controls_all_allow` is false.
This is a disclosed existing heuristic limitation, not permission to alter it.

The Docker CLI and credential helper were outside PATH; a process-local PATH
addition enabled real build/run. Container inspection confirms network none,
unprivileged user, read-only rootfs, dropped capabilities, no published ports or
mounts. Separate same-image IPv4/IPv6 documentation-address attempts failed with
loopback only and no default route. This does not prove future Gate topology.

Private raw evidence bundle: `.spec.local/root/evidence/20261106222914-rust-wargame/`.
Root integration instructions
are `crates/attacker/INTEGRATION.md`. No common Cargo files, Go, scoring, public
docs, runtime deployments, workflows or migration scripts were modified.
