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
contexts, lint isolation and10operational resource moves. The listed binary/feature
spellings were subsequently accepted by Root; detailed linkage and
test alias mapping remain pre-freeze proposals, not manifest edits.
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

Paths below follow Root's revised standard-src rule within eventual
crates/apps/humanymous-cli. Root accepted all15existing bin names and listed
qualification-* spellings/default-off plus native-planner. No source/Cargo move
is performed by this plan. Resources stay at package-root qualification/.

| Existing target | Proposed root/context | Proposed required feature | Role |
| --- | --- | --- | --- |
| humanymous_cli lib | src/lib.rs with current production modules | existing optional runtime-service | production library, no qualification exports |
| humanymous | src/main.rs | none; explicit release features | production CLI |
| humanymous-native | src/bin/native/main.rs + private native planner | native-planner | user-facing native planner, no protection claim |
| humanymous-service-fixture | src/bin/service_fixture/main.rs | existing service-pilot | opt-in fixture, release-excluded |
| humanymous-attacker | src/bin/attacker/main.rs | qualification-attacker | owned deployment lab workload |
| humanymous-report | src/bin/report/main.rs | qualification-report | legacy report role |
| humanymous-wargame-verify | src/bin/wargame_verify/main.rs | qualification-wargame-verify | source-bound verification |
| humanymous-agent-maintenance | src/bin/maintenance/main.rs | qualification-maintenance | private maintenance context |
| humanymous-delivery | src/bin/delivery/main.rs | qualification-delivery | required CI/delivery role |
| humanymous-pages | src/bin/pages/main.rs | qualification-pages | Pages/docs tooling |
| humanymous-lab-runner | src/bin/lab_runner/main.rs | qualification-lab-runner | isolated Docker lab runner |
| humanymous-runtime-pilot | src/bin/runtime_pilot/main.rs | qualification-runtime-pilot | runtime/custody pilot |
| humanymous-cef-pilot | src/bin/cef_pilot/main.rs | qualification-cef-pilot | explicit safe qualification binary |
| humanymous-native-runner | src/bin/native_runner/main.rs | qualification-native-runner | isolated native actor runner |
| humanymous-docs-review | src/bin/docs_review/main.rs | qualification-docs-review | explicit safe qualification binary |
| humanymous-local-parity | src/bin/local_parity/main.rs | qualification-local-parity | original local/hosted workload |

One production library and15retained binaries survive; the other five library
roots become private implementation contexts, not extra public libraries. Shared
attacker/defender support for lab/runtime/report must keep one canonical source
implementation and crate-private wiring. Root assigns ONEcanonical tree at
src/qualification/lab_support/{attacker,defender}/mod.rs. Bins link those actual
same files by in-package #[path]; rewrite old crate/import roots, not package aliases.
No implementation copy,
public production re-export, package alias or helper crate solves this linkage.
The precise minimal linkage and dead-code review below remain freeze prerequisites.

Disable automatic bin discovery. Integration/example discovery must also become
explicit where necessary to preserve target aliases/features. Required-features
does not prevent an intentional all-features qualification build. Release uses
an explicit production-bin allowlist/features: Root accepted default artifact ONLY
humanymous, plus humanymous-native only as an explicitly selected native-planner
artifact. All qualification/service fixtures are excluded. No developer/lab executable
ships through a package-wide install/all-features accident.

# Test, lint and unsafe matrix

## Precise34-root mapping and minimal bin linkage

P2 pre-freeze reconciliation: X found that Delivery's
maintenance-identity-proposal.json under evidence:root/20261107052001 relocates
session_storage and selected_inputs cases into the maintenance bin. This plan's
baseline maps those as two retained integration targets. Neither topology is
accepted merely by this draft. The static34input-root census remains factual;
postmerge counts below are conditional, not fixed freeze acceptance counts.

| Root choice | Integration roots | Compiled roots | Maintenance qualification case identities |
| --- | --- | --- | --- |
| Keep the two current integration targets | 12 | 29 | Root must supply matching unchanged-target profile identities |
| Accept Delivery's relocation into the owning bin | 10 | 27 | Proposed profile0.2 retains33unique old-to-new cases:23integration and10bin cases |

The10relocated cases perform real owned filesystem/provider work; they are not
pure/no-I/O. X verified33old target/test keys against profile0.1,33unique proposed
identities and eight source hashes/sizes/function lines. Root must explicitly
choose, then reconcile this prose, tools-linkage-map.json, target aliases/counts,
source placement, qualified-name inventory and the final profile in one freeze.
Do not retain empty integration targets, duplicate cases to preserve old counts,
or alter workload/authority classification. No code/profile/target relocation
has occurred. Delivery proposal SHA256 observed:
fb43e083664731eaff32891c8ef587352837be506b550fbbe6f165cba8ad8e54.

tools-linkage-map.json maps EVERYoriginal root: old package/kind/name/source/hash,
new target alias/source/required feature/lint/disposition. The five retired public
lib roots map to canonical private contexts with all unit cases retained. New
baseline compiled roots would be29before the unresolved relocation: one lib,
15bins,12integration tests and one example. This count reduction is not test loss.
All Rust roots live under src, including src/qualification/tests and examples;
disable automatic discovery and list those explicit targets in Root's manifest.

Exact integration aliases are cli-watchdog-boundary, cli-service-process,
cli-runtime-service-process, native-command, maintenance-process,
maintenance-git-coord, maintenance-windows-doctor, maintenance-product-package,
maintenance-evidence-layout-cli, maintenance-session-storage,
maintenance-selected-inputs-cli and maintenance-selected-inputs. The example is
cli-service-evidence. Minimal existing feature selectors: service-process uses
service-pilot; runtime-service-process uses runtime-service-pilot; watchdog uses
both; native-command uses native-planner; maintenance tests use qualification-
maintenance. The existing provenance example needs no service feature. These
exact aliases/selectors are Root-review proposals, not executed Cargo metadata.

| Bin | Minimal source modules and namespace rewiring |
| --- | --- |
| humanymous | existing production library only; no qualification module declared |
| humanymous-native | src/qualification/native/mod.rs; old native lib+main handlers bind native::{arguments,manifest,storage}, old crate:: helpers become crate::native::* |
| humanymous-service-fixture | src/qualification/service_fixture/mod.rs and existing explicitly feature-gated CLI fixture APIs; no new lib qualification re-export |
| humanymous-attacker | actual canonical attacker+defender mod.rs files; old humanymous_attacker -> crate::attacker, humanymous_defender -> crate::defender; former attacker crate:: references become crate::attacker::* |
| humanymous-report | only canonical attacker/report/legacy/mod.rs leaf and its aggregate/html children; use crate::legacy, no full network/defender tree merely for type visibility |
| humanymous-wargame-verify | canonical attacker+defender roots: verifier actually references replay run, NetworkReport/report types and defender LabError; full reachable closure reviewed, not a copied verifier shim |
| humanymous-agent-maintenance | src/qualification/maintenance/mod.rs plus its existing module declarations; main/imports become crate::maintenance::*; no production lib exports |
| humanymous-delivery | src/qualification/delivery/mod.rs; its existing dispatcher/modules and Result/Options/helper references become crate::delivery::*; no other tool trees included |
| humanymous-pages | src/qualification/pages/mod.rs plus existing browser command; old Pages APIs/helper refs become crate::pages::* within this bin |
| humanymous-lab-runner | src/qualification/lab_runner/mod.rs plus only canonical shared lab support files it actually reads; no attacker/defender by default |
| humanymous-runtime-pilot | src/qualification/runtime_pilot/mod.rs plus canonical attacker+defender and its actual shared lab inputs; preserve Unix/test cfg branches |
| humanymous-cef-pilot | src/qualification/cef_pilot/mod.rs with existing desktop/diagnostics/provisioning; no actor module |
| humanymous-native-runner | src/qualification/native_runner/mod.rs with existing actor/controlled/evidence/fixture/harness/pilot/service/workspace and cfg(test) prebuilt source; actor references become crate::native_runner::* ONLYhere |
| humanymous-docs-review | src/qualification/docs_review/mod.rs only, current single-file parser/command behavior retained |
| humanymous-local-parity | src/qualification/local_parity/mod.rs only, unchanged workload and root/control provenance |

Each bin root is a narrow composition entry and links actual in-package files with
#[path]. Private mod.rs owns the command implementation; expose only a parent-
visible command entry such as pub(super), not a generalized capability constructor.
Root-private helpers formerly in main stay inside that owning context. Namespace
rewiring includes nested crate:: references and unit tests; it does not require
aliases, public production tool modules or copies of an implementation file.

## Dead-code and visibility handling proposal

Compile the minimal role closure first. Legacy report links its leaf, not the
whole attacker library. Preserve existing public item/field visibility inside
former library contexts, and preserve existing private opaque authority/owner
types exactly. A reviewed pub mod at an owning BINARYroot may retain an existing
tool library's public namespace/lint reachability inside that executable; it is
not a production-library export. This is permitted only for exact former public
interfaces already in that role's original dependency closure, never to expose
private secrets, native grants, actor routines or opaque fields to silence a lint.
Keep other modules private. Any candidate newly-public item beyond that preserved
interface is an explicit review blocker, not automatic dead-code cleanup.

No blanket allow(dead_code), expect-based suppression, pub-everything pass or
unsafe weakening. For each actual warning, record owner/item/source, whether it
is an existing externally reachable tool API, a missing true caller, a module
unnecessarily linked by this role, or an obsolete item needing owning retirement.
Repair the true linkage/minimal module selection; any semantic deletion or
visibility expansion goes back to Root/owner. Do not fabricate callers/tests.

Moving the former lib #[cfg(test)] cases into canonical contexts can recollect
them in attacker/verifier/runtime-pilot or report bins. Preserve every original
case and record per-target source-qualified inventories; repeated collection is
not new unique coverage. Root must approve unique-case mapping and workload
accounting before broad test execution. Do not delete cfg(test) cases or infer a
full native pilot from a duplicate pure/unit result to force old totals.

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

tools-linkage-map.json adds a recursive literal path/include source review to the
34root map:25referenced inputs outside the13old package roots, plus10manifest-
relative concat includes resolved to actual original hashes. The latter are
filesystem-resolved, but their destination/reader policy is not accepted merely
because a file exists. This expanded closure is NOT_FROZEN. The original10resource
table alone is insufficient; no blanket closed-input claim is made.

Concrete closure blockers for Root/owning review:

- Shared test/lab browser_expectations/browser_outcome/browser_wire/dns_wire/
  mount_policy Rust sources and their browser-wire/go-configuration JSON inputs
  need one canonical src/qualification/lab_support/shared tree, exact bytes and
  all readers assigned. This is an extension proposal, not an autonomous move.
- test/lab/runtime currently links Collector implementation source at
  crates/collector/src/browser/transport/runtime_pilot.rs. Do not copy it into CLI,
  silently depend on an unpublished path or invent a fixture public API. Root
  must settle the actual producer/ownership boundary with the Collector owner.
- Existing Core frozen scoring JSON replicas require exact producer/path/blob/
  hash/license binding and coordinated equality under the accepted replica rule;
  collector fixture and desktop CEF contract need accepted producer-owned pins.
- Mutable .agents registries/rules and scripts/agents verification inputs remain
  original-owner resources. Replace snapshot-dependent repository checks with
  bounded explicit repository readers and small synthetic parser fixtures as
  accepted by Root; do not embed mutable THREADS/lanes/script snapshots in a
  published tool or discard real repository verification. Product SKILL/reference
  packaging likewise needs a genuine input contract, not an incidental embedded
  copy just to compile under a new CARGO_MANIFEST_DIR.
- docs normalized-profile fixtures, operational language-preservation inputs and
  the Docker product image recipe need explicit owner/source-bound readers or
  approved resource ownership. No implementation/source/template copying rule is
  waived by this inventory. Root must assign all remaining dynamic reader paths.

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

1. Accepted15bin names/features/default artifact policy stay fixed. Root reviews
   remaining test aliases/minimal feature selectors, per-bin shared module
   linkage, preserved bin-only public namespaces and dead-code handling. Resolve
   every additional external input/Collector/mutable-registry boundary before
   freeze. No public production tool API or implementation copying.
2. Owners reconcile all34roots, unit inventories,10resources/123reader locations,
   Cargo categories, expanded input closure and actor operations. Freeze exact hashes/modes/current dirty
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
