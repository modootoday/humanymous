---
id: plan:root/20261107203000
title: Platform slice two-context feature and consumer audit
status: draft
created: 2026-11-07T20:30:00Z
updated: 2026-11-07T20:30:00Z
rel:
  depends_on: [sot:root/structural-development, sot:root/cross-os-runners]
---

# Root-assigned preparation only

Accepted destination crates/integrations/humanymous-platform exposes ONLY
humanymous_platform::{windows_object,testing} for this slice. No moves, manifests,
public API edits or new compatibility/private helper package are implemented.
Root freezes the exact slice and owns common graph. native-security remains a
separate safe context until later Runtime absorption. No SDK dependency belongs
in platform; particularly do not create core -> platform -> core cycles.

# Exact proposed feature/dependency settings

default=[]; windows-object=[dep:windows-sys,dep:windows-permissions];
testing=[dep:serde,dep:serde_json]. Declare serde/serde_json optional normal deps;
windows-sys=0.61.2 (Foundation, Storage_FileSystem, Security) and
windows-permissions=0.2.4 optional ONLY under target cfg(windows) dependencies.
Declare windows_object with cfg(all(windows,feature="windows-object")); declare
testing with cfg(feature="testing"), NOT cfg(test) alone. testing::process keeps
its existing non-wasm cfg; no process capability on wasm. No feature implies
another; enabling windows-object on non-Windows exposes no bridge API/dependency.

Package rust unsafe lint must be deny rather than inherited workspace forbid,
with windows_object module deny and EXACT existing six function-local allow
scopes (five query, one create). testing module must explicitly forbid unsafe.
No package/root allow, extra unsafe scope, wrapper or new ffi. No root pub use
flattens either context. Missing-doc/Clippy rules remain equivalent.

native-security target cfg(windows) normal dependency:
humanymous-platform, default-features=false, features=["windows-object"].
Core dev dependency uses platform default-features=false, features=["testing"].
Each other test/tool consumer explicitly enables testing in its existing normal
or dev dependency category. No normal core dependency on platform/testing.
Do not alias the old package name to hide required API changes.

# Exact source/consumer inventory and semantic edits required

platform-slice-consumers.json in the owned evidence bundle hashes90 current
text-reference files across Cargo/Rust/Markdown/workflows at its recorded HEAD.
It is a static scan, not executed Cargo closure; historical spec receipts are
excluded. Root must rerun at freeze because other owners are actively editing.

| Consumer class | Required changes and owner hazard |
| --- | --- |
| Core, network, audit, CLI, .NET, Python tests | Existing assertions/process APIs become testing::assert_json_equivalent/testing::process and testing::json; preserve dev versus normal category. |
| attacker, lab runner/runtime, delivery, CEF pilot | Some use strict JSON/process helpers as NORMAL tool deps, contrary to old lib prose saying dev-only; explicitly opt testing in, no permission inference. |
| browser bootstrap/desktop | Normal or dev imports and standalone path depths need their owning browser review; browser qualification remains last. |
| native-security | Manifest currently depends on bridge on Windows; static Rust scan finds no current bridge import. Do not invent a consumer call or native proof. |
| windows-native-runner | Standalone manifest path must point to platform with testing; actor/prebuilt, harness, pilot, recovery, service imports change atomically, foreign dirty receipt.rs needs owner custody. |

Internal testing parent_context.rs and parent_context/binding.rs use crate::json:
change to crate::testing::json. capture_attempt/worker.rs has pub(in crate::process)
on Worker and methods: change EXACT visibility to crate::testing::process, keeping
the worker private to that shared process owner. Do not broaden to pub(crate) or
duplicate Worker/OwnedDirectChild. Sibling super paths remain relative to the same
testing subtree and need review, not blanket replacement. testing lib numeric
comparison helpers remain private to testing.

Bridge lib's cfg(windows)/deny move to windows_object module; create/query retain
super relations. Private observation fields and no raw-handle/serde/Debug factory
remain. Update both doctest imports to humanymous_platform::windows_object and
retain compile_fail E0451 plus successful public import test under the explicit
Windows feature. Private data is inaccessible to sibling testing despite same
crate; do not add pub(crate) observation constructors/fields to bridge that gap.

Move linux_tree_inventory example with required-features=["testing"] and update
its import. Copy/update current INTEGRATION docs, including legacy blocking/wait
caveat; source-specific historical evidence and source inventories stay immutable.
Runner evidence.rs old source roots must change only with the actual move.

# Execution identity and negative-proof hazards

Runner controlled.rs currently admits executable prefix humanymous_test_support-
and selectors process::capture_attempt::actual_tests::* and
process::controlled_barrier::actual_tests::*. New lib harness prefix is
humanymous_platform- and actual selectors are testing::process::... . Update BOTH
hardcoded exact allowlist/negative tests AND the fixture's own grant-selector
strings; otherwise admission can fail or mismatch. No wildcard widening.
harness/helpers.rs package_id and target checks must change from test-support
to platform; only the testing-enabled platform harness may claim these helpers.
Cross-check other harness/footer/accounting consumers before changing counts.

Nonoptional Root decision: prevent workspace feature unification from disguising
default platform authority. Verify isolated default, testing, windows-object,
and joint closures independently; workspace --all-features is not default proof.
Current core package attempt failed101 offline because unpublished test-support
could not resolve in registry, even with multi-package command. Preserve dev
semantics and versioned dependencies: separate core normal closure from testing
dev closure; local clean packaged-consumer proof does not prove public namespace
availability. No publication or blind dev-dependency removal is authorized.

# Required gates and blockers before implementation

Root must accept exact module lint boundary, feature/dependency definitions,
standalone tool ownership and selector/fixture remapping, plus resolve foreign
receipt.rs modifications. Need new API positive import and private-field
compile_fail tests, no-feature API absence and non-Windows bridge absence proof.
Run locked check/test/strict Clippy separately for closed feature selections after
allocation; preserve existing pure counts and reconcile new test-name prefixes.
Actual outer faults and process fixtures require new source-bound native grants;
pure/default closure does not qualify SCM/Host/Authority/interactive workers.

Priority source-custody regression fix is authored in
[[plan:root/20261107203500]]: nested manifests, required production JSON and
supplied snapshot missing-key refusal. Three regressions NOT_RUN; global
CA-NATIVE-SOURCE-01 stays OPEN until Root-allocated proof. No new compiler/runtime
lease, unsafe/dependencies, structural moves or historical receipt rewrite here.

## Root freeze handoff

Root accepted the exact map: default[], independent testing/windows-object,
optional Windows deps only on Windows, root deny/testing forbid/exact six local
allows, no flattened exports/old dependency aliases. This is NOT an independent
move grant to the Native owner. Root owns the35-to34 package integration.
platform-two-context-freeze.json records ALL28 tracked files of test-support and
windows-object-bridge at its exact HEAD, SHA256+length, dirty0/untracked0.
Native guard session43027 is actual terminal1 (three regressions/check/Clippy pass,
first final dependency fmt failure retained); no active owned leases/marker.
Both source contexts are frozen: Native owner released/narrowed native-windows
lane to exclude those two roots, preserving native-security/runner/spec ownership.
No writes to frozen old or new sources until Root returns an exact slice.
The session board handover records this narrow release; no ninety-file rescan.

## Authorized runner consumer source checkpoint

Root authorized exact namespace and fixed helper identity mapping only, while
retaining sole ownership of all moves/manifests/locks. Ten runner Rust files
changed: actor/prebuilt, controlled, evidence, evidence/recovery, harness,
harness/helpers, pilot, service/execution, service/receipt, service/summary.
platform-runner-consumer-source.json records before/after SHA256 per path.
Imports now use humanymous_platform::testing; controlled executable prefixes
and capture/barrier selectors use humanymous_platform- and testing::process;
harness helper package/target and six exact parent strings use platform names.
Evidence's collected and own-Git-byte roots use the actual platform src/root.
No wildcard, authority, behavior, unsafe or required-core-three-key policy change.
source_inputs.rs remains SHA72190755582633f5289f1d3af541970b96cbbfb513c1fbbfd28403ee6e130f3c.
native-security contains no bridge Rust imports to edit. No compiler/fmt/run.

Receipt custody correction: saved pre-edit receipt bytes SHAa24d1ca965d724589a3a5613af38747d1d4295448173fb65e1629f640ebb44df
hash to Git blob b5f2a57d13331fd5c22dc50e23c267c0a5260eea, equal BOTH current
HEAD and historical native-guard HEAD1e8c5af6844915090840df834e262eec51cc9e3c.
The earlier status-M observation did not establish foreign CONTENT; historical
raw receipts remain unchanged, with this additive correction only. Root's
superseding authorization permits staging the one mechanical import replacement.
Actual after SHAce276fc46c5c134ba056b3e87e820731dd8307ee8e2c909d0544bfd5c8aabbc1.
No restore/stash/index repair was used. Full graph and new source remain untested.

Root then authorized formatter-only standalone runner correction after metadata
resolution. cargo fmt --manifest-path scripts/ci/windows-native-runner/Cargo.toml
-p humanymous-native-runner exited0, changing ONLY controlled.rs qualified grant
encoder call wrapping (6 added/5 removed lines, no token change). Result SHA256
61ad7eba41026752665ab6050e6dc06396f98b2638ad84bf7b8a5f760ae18733.
Required-input guard source hash72190755582633f5289f1d3af541970b96cbbfb513c1fbbfd28403ee6e130f3c
remains unchanged. No --all mutation, test/check/Clippy or native process pilot.

## Opaque Cargo package ID defect and source correction

Root observed actual platform ID
path+file:///C:/workspaces/humanymous/crates/integrations/humanymous-platform#0.1.0.
The previous contains("#humanymous-platform@") helper guard therefore refused
the actual moved package. An ID is opaque, not a package-name/suffix grammar.
This source correction is authorized; no consumer qualification is implied.

harness/packages.rs loads bounded strict Cargo metadata --locked --offline
--no-deps --format-version1 using the existing shared capture implementation.
It retains metadata stdout/stderr privately beside the report; verifies source
snapshot before/after; requires exact current logical workspace_root, unique
workspace_members-to-packages ID join, local null source, nonempty name/version,
absolute workspace-contained manifest and target inventory. Five role manifests
(platform/audit/authority/runtime/CLI) must hash-match source custody; unrelated
workspace tools are not claimed to be in the narrower native snapshot. No
registry/path counterfeit can assign one of these roles from its name alone.
The loader uses the existing legacy capture limits, not a new bounded-settlement
or lifecycle guarantee. Windows logical root spelling avoids canonicalize's
verbatim-prefix mismatch with Cargo metadata; no namespace rule is relaxed.

Helper/custody/required-service/native-process classification now joins exact
artifact package_id against that map and compares resolved name, exact expected
manifest and exact metadata target name/kind. No substring/rsplit interpretation
or old-name alias remains in harness, helpers or service catalog. Every role's
expected manifest is CURRENT source path; future Runtime/CLI moves require Root
to update that explicit table, not add suffix heuristics. Delivery's separate
native_archive logic is outside this owner and was routed by Root.

Workspace/pilot/service callers obtain the same-source map before classification;
supplied artifact manifests cannot supply their own metadata role map. Audit,
Authority and CLI retain their same exact tests/kind/target/ignored semantics;
Platform keeps exactly the same helper plus six parent strings, no broad waiver.

Five pure tests authored across package-map2, helper2 and catalog1 selectors:
actual short platform ID succeeds via metadata, unknown/other-ID/name/manifest/
kind/target and nonmember deny; duplicate/absent members, registry source,
empty version and relative manifest refuse; existing Audit/Authority/product
and CLI service negatives remain. All tests and loader execution NOT_RUN.
Bare source rustfmt/whitespace/spec checks only; no Cargo/compiler/runtime lease,
manifest/dependency/unsafe/feature change. Root must allocate scoped tests plus
runner all-target check/strict Clippy and actual metadata/source join proof.
Required core source_inputs guard remains unchanged.

## Allocated opaque-ID pure validation

New original monotonic390/450 budget, fresh CreateNew own marker on explicitly
reallocated target/native-source-guard-v0; jobs1/incremental0/locked/offline.
Actual tool66287 terminal0 at15.20s, all owned processes terminal and own marker
removed within original cleanup. Observed source HEAD057befe36a15242a619e286b6ee6aa1f7ae5358c,
no overlays. All prefix inventories verified2+2+1 before any selected execution.
Actual package-map2/helper2/catalog1 passed, zero failed/ignored/measured;
filtered58/58/59 separately. Runner all-target check/strict Clippy/scoped package
fmt all0. No previous source-guard fmt receipt was rewritten or hidden.

Selected local/root source+manifest/lock/config/tool105 files, selected registry
2079 files and nine explicit environment keys were identical before/after.
The selected test binary was c605bbcb6a8c777c, explicitly joined from all three
Cargo test stderr receipts; another cached executable is NOT qualified.
platform-package-id-validation-summary.json registers50 private raw attachments.
Production Packages::load and actual repository metadata/source association were
NOT_RUN: these are fixture-based role-classification tests, not native/full graph
proof or a new settlement guarantee for the existing capture implementation.
No active own resource lease remains; target returned as cache only to Root.
