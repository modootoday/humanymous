---
id: plan:root/20261107224511
title: CLI installation tools target and private context freeze matrix
status: draft
created: 2026-11-07T22:45:11Z
updated: 2026-11-07T22:45:11Z
rel:
  depends_on: [sot:root/cli-installation, sot:root/structural-development]
---

# Scope and source census

Pre-freeze v0, no source/path/Cargo moves or compiler execution. Root's accepted
structural-development contract controls13Tools responsibilities, private bin
contexts, lint isolation and10operational resource moves. Final feature/path
spellings below are explicit PROPOSALS for Root to canonize, not manifest edits.
No new helper crate or public tool-library API is proposed.

[[evidence:root/20261107200932]] contains tools-freeze-census.json:13manifest
bodies/dependency categories,34target roots,279tracked files with Git modes/blob/
SHA256,10resource contracts and the two actor files' unsafe lexical inventory.
tools-source-readers.json records123include/manifest-root/CARGO_BIN_EXE/current_exe
locations. Current roots:6libs,15bins,12integration tests,1example; no discovered
bench/build root. Explicit plus conventional roots were enumerated without Cargo;
Root must reconcile actual metadata before accepting the freeze. These observed
hashes do not qualify later edits or an immutable whole-graph source closure.

# Production and private context matrix

Keep production lib's current Plan/release/service and optional runtime_host APIs.
Do not add pub maintenance/pages/attacker/defender/delivery/lab modules merely to
preserve old crate imports. Native CLI is user-facing manifest planning, not
native protection. Prefer its existing arguments/manifest/storage as a private
humanymous-native binary context, not new public CLI SDK exports. Any genuine
public API addition requires a separate supported-purpose decision.

Paths below are relative to eventual crates/apps/humanymous-cli. Each feature is
a proposed Root spelling; qualification features default off. Existing production
features remain explicit. No path or feature is adopted by this draft.

| Existing target | Proposed root/context | Proposed required feature | Role |
| --- | --- | --- | --- |
| humanymous_cli lib | src/lib.rs with current production modules | existing optional runtime-service | production library, no qualification exports |
| humanymous | src/main.rs | none; explicit release features | production CLI |
| humanymous-native | src/bin/native/main.rs + private native planner | native-planner | user-facing native planner, no protection claim |
| humanymous-service-fixture | qualification/service/fixture.rs | existing service-pilot | opt-in fixture, release-excluded |
| humanymous-attacker | qualification/attacker/main.rs | qualification-attacker | owned deployment lab workload |
| humanymous-report | qualification/report/main.rs | qualification-report | legacy report role |
| humanymous-wargame-verify | qualification/wargame_verify/main.rs | qualification-wargame-verify | source-bound verification |
| humanymous-agent-maintenance | qualification/maintenance/main.rs | qualification-maintenance | private maintenance context |
| humanymous-delivery | qualification/delivery/main.rs | qualification-delivery | required CI/delivery role |
| humanymous-pages | qualification/pages/main.rs | qualification-pages | Pages/docs tooling |
| humanymous-lab-runner | qualification/lab_runner/main.rs | qualification-lab-runner | isolated Docker lab runner |
| humanymous-runtime-pilot | qualification/runtime_pilot/main.rs | qualification-runtime-pilot | runtime/custody pilot |
| humanymous-cef-pilot | qualification/cef_pilot/main.rs | qualification-cef-pilot | explicit safe qualification binary |
| humanymous-native-runner | qualification/native_runner/main.rs | qualification-native-runner | isolated native actor runner |
| humanymous-docs-review | qualification/docs_review/main.rs | qualification-docs-review | explicit safe qualification binary |
| humanymous-local-parity | qualification/local_parity/main.rs | qualification-local-parity | original local/hosted workload |

One production library and15retained binaries survive; the other five library
roots become private implementation contexts, not extra public libraries. Shared
attacker/defender support for lab/runtime/report must keep one canonical source
implementation and crate-private wiring. Root must assign exact common private
module paths and rewrite crate:: imports at owning roots. No implementation copy,
public production re-export, package alias or helper crate solves this linkage.
This unresolved private sharing is a concrete freeze blocker.

Disable automatic bin discovery. Integration/example discovery must also become
explicit where necessary to preserve target aliases/features. Required-features
does not prevent an intentional all-features qualification build. Release uses
an explicit production-bin allowlist/features; native planner distribution needs
Root's inclusion decision. No actor, service fixture or developer/lab executable
ships through a package-wide install/all-features accident.

# Test, lint and unsafe matrix

All12existing integration roots and service_evidence example are named/hashed in
the census. Proposed aliases are cli-watchdog-boundary, cli-service-process,
cli-runtime-service-process, native-command and maintenance-<original-name>.
Root freezes exact names and CARGO_BIN_EXE rewires. Do not drop or silently merge
targets to avoid collisions. Unit tests from retired tool libs move with their
owning private bin contexts; preserve their inventory with crate-private rewiring.

Every compiled safe root explicitly forbids unsafe: production lib/main, native
planner, safe qualification bins, integration tests, example and any discovered
generated/build/bench root. Native runner alone uses deny and retains actor.rs:8
and actor/scm.rs:5 allows. Workspace forbid remains unchanged; Root's package-local
deny/warn exception never permits unsafe in sibling safe roots.

Freeze existing actor operations/linkage as well as attributes. The census has
26unsafe lexical operation sites in actor.rs and7in actor/scm.rs, exact file
hashes/line text retained. These are lexical counts, not a soundness proof. Only
reviewed mechanical crate/path rewiring may differ; semantic unsafe/new allow/
unreviewed path/include/macro linkage requires a separate decision. Actor source
must not move to platform testing or production/safe sibling contexts.

Required controls, all NOT_RUN: isolated unsafe-negative compile probes for each
safe root, rejection of unapproved actor linkage, positive Windows-only actor
compilation, and exact target/bin/kind/source/features/profile/artifact native-role
joins. Shared package identity must reject wrong sibling executables. Source
census must inspect unsafe operations/module reachability, not just count allows.

# Resource, Cargo category, mode and source-path matrix

Move exactly the10original operational contracts to Root's qualification paths
recorded in tools-freeze-census.json. Preserve bytes, Git objects, hashes and
all readers/source pins. Header R3 remains compile-time embedded from its owned
package path with unchanged compiler/job/proof contract; no ambient filesystem
profile, self-reported replacement, fixture copy or weaker digest check.

Additional freeze inputs: CEF's browser/native/desktop/pilot-contract.json include;
local parity's CARGO_MANIFEST_DIR control root; Delivery/maintenance repository
and archive roots; native runner current_exe self-reexecution; every recorded
CARGO_BIN_EXE. Root must preserve original parity workloads, admission/deadlines,
immutable compiler profiles and advisory scope of four retired standalone locks.
Unified-lock selectors must explicitly replace that coverage, not omit it.

Each of13manifest snapshots retains normal/dev/build/Windows-target dependency
tables plus features/lints. Root maps these edges to explicit role features and
keeps service/credential-pilot, CEF/SDK, audit/runtime and platform-testing context
selection. CLI normal Runtime edge must remain native-security without Gateway.
Production must not activate network/archive/CEF/actor/qualification dependencies
merely because they were former tool-package normal dependencies. Optionalization
requires actual tree/compile controls; no source/feature/alias workaround.

Git file modes/blobs and current hashes for279tracked files are recorded. This is
not deployed Unix owner/executable-mode evidence. Root preserves or explicitly
reviews destination modes and refreshes dirty/untracked/source census at freeze.
The123lexical reader locations are a review queue, not every possible ambient
input or semantic reference. Actual production/qualification graph proof remains
mandatory after the coherent move.

# Ordered freeze and verification

1. Root canonizes or revises all proposed feature/target paths, native-planner
   distribution, five retired-lib private contexts and canonical shared lab
   linkage. No public tool API or implementation copying to preserve imports.
2. Owners reconcile all34roots, unit inventories,10resources/123reader locations,
   Cargo categories and actor operations. Freeze exact hashes/modes/current dirty
   paths only after the matrix and unresolved fixture/role edges are closed.
3. Root performs atomic13-to1 source/Cargo/lock/consumer handoff. Owners edit only
   explicitly delegated paths, preserving bin names, role exclusions and old
   source-specific evidence. No independent move, ref switch or process action.
4. Root allocates fmt-first dependency/feature trees, pure/compile/Clippy and
   unsafe-negative matrices, extracted-package controls and actual release
   inventory exclusions. OS/CEF/process/SCM and Docker product gates stay separate.
   Local services/accounts remain unchanged. Native proof/Ready/score gates stand.

# Review and limits

Single-author blue/red/SRE/DPO/evaluator and adversarial review: needs-work pending
the explicit Root decisions. P0 actor leakage requires all safe-root forbids and
role-specific identity. P1 fixture substitution requires original embedded bytes/
pins/readers. P1 disappearing tests/advisories requires preserved inventories.
P1 public API creep requires private contexts. These are future controls, not
already passing implementation. No Cargo/compiler/runtime/SDK/CI/release action
was run. Existing84828470 qualifies the earlier8e51consumer slice, not Tools13-to1.
