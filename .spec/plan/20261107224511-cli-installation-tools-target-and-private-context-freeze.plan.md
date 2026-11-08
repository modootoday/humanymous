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

Implementation/composition identity reconciliation is explicit in
bin-composition-identity-map.json: ALL15roles name old implementation/hash,
new actual implementation file, private crate-root namespace and new composition
root. The232Rust src-file rows join Root's279file map; all187LIB and259BINcase
source paths/selectors now point at actual implementation, never at a thin root.
Old source SHA256 applies to implementation bytes only. The14nonprimary
src/bin/<role>/main.rs roots are NEWnarrow Rust composition source, not oldmain
files moved there. Primary humanymous src/main.rs stays the existing implementation.

| Bin | OLDimplementation | NEWimplementation | Private command namespace | NEWcomposition root |
| --- | --- | --- | --- | --- |
| humanymous | crates/cli/src/main.rs | src/main.rs | existing root | src/main.rs, unchanged |
| humanymous-native | crates/native-cli/src/main.rs | src/qualification/native/command.rs | native_command | src/bin/native/main.rs |
| humanymous-service-fixture | crates/cli/src/bin/service_fixture.rs | src/qualification/service_fixture/mod.rs | service_fixture | src/bin/service_fixture/main.rs |
| humanymous-attacker | crates/attacker/src/main.rs | src/qualification/attacker_command/mod.rs | attacker_command | src/bin/attacker/main.rs |
| humanymous-report | crates/attacker/src/bin/humanymous-report.rs | src/qualification/report_command/mod.rs | report_command | src/bin/report/main.rs |
| humanymous-wargame-verify | crates/attacker/src/bin/humanymous-wargame-verify.rs | src/qualification/wargame_verify_command/mod.rs | wargame_verify_command | src/bin/wargame_verify/main.rs |
| humanymous-agent-maintenance | skills/agent-maintenance/src/main.rs | src/qualification/maintenance/command.rs | maintenance_command | src/bin/maintenance/main.rs |
| humanymous-delivery | scripts/ci/delivery/src/main.rs | src/qualification/delivery/mod.rs | delivery | src/bin/delivery/main.rs |
| humanymous-pages | docs/tooling/src/main.rs | src/qualification/pages/command.rs | pages_command | src/bin/pages/main.rs |
| humanymous-lab-runner | test/lab/runner/src/main.rs | src/qualification/lab_runner/mod.rs | lab_runner | src/bin/lab_runner/main.rs |
| humanymous-runtime-pilot | test/lab/runtime/src/main.rs | src/qualification/runtime_pilot/mod.rs | runtime_pilot | src/bin/runtime_pilot/main.rs |
| humanymous-cef-pilot | scripts/ci/cef-pilot/src/main.rs | src/qualification/cef_pilot/mod.rs | cef_pilot | src/bin/cef_pilot/main.rs |
| humanymous-native-runner | scripts/ci/windows-native-runner/src/main.rs | src/qualification/native_runner/mod.rs | native_runner | src/bin/native_runner/main.rs |
| humanymous-docs-review | scripts/ci/docs-review/src/main.rs | src/qualification/docs_review/mod.rs | docs_review | src/bin/docs_review/main.rs |
| humanymous-local-parity | scripts/ci/local-parity/src/main.rs | src/qualification/local_parity/mod.rs | local_parity | src/bin/local_parity/main.rs |

For native/pages/maintenance, the original LIBlib.rs moves to that context's
mod.rs ONCEand stays under native/pages/maintenance namespace. Its original MAIN
moves to command.rs separately. Composition roots path-link the lib context and
the separate *_command module directly at their binary crate root. Main-command
imports point to crate::<lib-context>, while helpers defined by that command
remain in crate::<command-context>. This avoids overwriting mod.rs, merging
unrelated test namespaces or adding forwarding exports to the production library.

Pages browser_command.rs remains the single physical
src/qualification/pages/browser_command.rs implementation and is declared ONLYby
pages_command with an explicit file-relative path from command.rs. Its ONEbin unit
selector changes from the prior proposal pages::browser_command::tests::<name>
to pages_command::browser_command::tests::<name>. All37Pages LIBselectors remain
pages::<old-selector>. Native and Maintenance own MAINunit counts are0; their
LIBselectors remain native::* and maintenance::*. The accepted10relocated
Maintenance cases remain maintenance::qualification_tests::*, not maintenance_command.

Attacker/verifier composition roots link exact canonical shared attacker/defender
files plus their distinct *_command implementation. Report links ONLYthe
canonical legacy leaf plus report_command. Runtime-pilot links its own command
context and only legitimate-cfg actual shared files. Actor stays ONLYunder
native_runner; crate::Options/evidence/harness rewrites to that exact context.
No old-package alias, implementation copy, SDK helper or capability-field visibility
change is proposed. Each narrow command entry is parent-visible inside its
owning binary and preserves original arguments, return/error/exit and flush.
All safe composition roots forbid unsafe; native runner denies with only the
existing two actor-module allows. No source/Cargo/move/compile has occurred.

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

P2 pre-freeze reconciliation CLOSED AS DESIGN: Root accepted Delivery
daf541b871c4c5255498e0b7e94fb0f976706ad4 relocation after human confirmation.
The accepted result is10integration roots/27compiled roots; all33maintenance
cases remain23integration +10actual owned-filesystem/provider cases in the
maintenance binary. No pure reclassification or implemented move is implied.

X originally found that Delivery's
maintenance-identity-proposal.json under evidence:root/20261107052001 relocates
session_storage and selected_inputs cases into the maintenance bin. This plan's
baseline mapped those as two retained integration targets. The static34old-input
root census remains factual; the accepted27compiled-root design replaces the
baseline29root proposal below. Whole source/resource closure is still not frozen.

| Root choice | Integration roots | Compiled roots | Maintenance qualification case identities |
| --- | --- | --- | --- |
| Superseded baseline retaining two integration targets | 12 | 29 | Not selected; no empty targets may preserve this count |
| ROOT ACCEPTED relocation into the owning bin | 10 | 27 | Profile0.2 mapping retains33unique old-to-new cases:23integration and10bin cases |

The10relocated cases perform real owned filesystem/provider work; they are not
pure/no-I/O. X verified33old target/test keys against profile0.1,33unique proposed
identities and eight source hashes/sizes/function lines. This prose and
tools-linkage-map.json now adopt Root's choice and exact33case selectors. Root
must freeze final source placement, compiled target aliases/counts, qualified-name
inventory and the final profile together before implementation.
Do not retain empty integration targets, duplicate cases to preserve old counts,
or alter workload/authority classification. No code/profile/target relocation
has occurred. No Cargo/source/path change is authorized before Root freezes the
complete reconciled input map. Delivery proposal SHA256 observed:
fb43e083664731eaff32891c8ef587352837be506b550fbbe6f165cba8ad8e54.

tools-linkage-map.json maps EVERYoriginal root: old package/kind/name/source/hash,
new target alias/source/required feature/lint/disposition. The five retired public
lib roots map to canonical private contexts with all unit cases retained. The two
maintenance test-source rows map to bin-owned modules, not independent targets.
Accepted compiled roots are27: one lib,15bins,10integration tests and one example.
This count reduction is not test loss.
All Rust roots live under src, including src/qualification/tests and examples;
disable automatic discovery and list those explicit targets in Root's manifest.

Exact integration aliases are cli-watchdog-boundary, cli-service-process,
cli-runtime-service-process, native-command, maintenance-process,
maintenance-git-coord, maintenance-windows-doctor, maintenance-product-package,
maintenance-evidence-layout-cli and maintenance-selected-inputs-cli. The example is
cli-service-evidence. Minimal existing feature selectors: service-process uses
service-pilot; runtime-service-process uses runtime-service-pilot; watchdog uses
both; native-command uses native-planner; maintenance tests use qualification-
maintenance. The existing provenance example needs no service feature. These
exact aliases/selectors are Root-review proposals, not executed Cargo metadata.

| Bin | Minimal source modules and namespace rewiring |
| --- | --- |
| humanymous | existing production library only; no qualification module declared |
| humanymous-native | native LIBmod.rs + native_command command.rs; command imports crate::native::{arguments,manifest,storage}; LIBhelpers remain crate::native::* |
| humanymous-service-fixture | src/qualification/service_fixture/mod.rs and existing explicitly feature-gated CLI fixture APIs; no new lib qualification re-export |
| humanymous-attacker | actual canonical attacker+defender mod.rs files; old humanymous_attacker -> crate::attacker, humanymous_defender -> crate::defender; former attacker crate:: references become crate::attacker::* |
| humanymous-report | only canonical attacker/report/legacy/mod.rs leaf and its aggregate/html children; use crate::legacy, no full network/defender tree merely for type visibility |
| humanymous-wargame-verify | canonical attacker+defender roots: verifier actually references replay run, NetworkReport/report types and defender LabError; full reachable closure reviewed, not a copied verifier shim |
| humanymous-agent-maintenance | maintenance LIBmod.rs plus maintenance_command command.rs; LIBhelpers crate::maintenance::*, MAINhelpers crate::maintenance_command::* |
| humanymous-delivery | src/qualification/delivery/mod.rs; its existing dispatcher/modules and Result/Options/helper references become crate::delivery::*; no other tool trees included |
| humanymous-pages | pages LIBmod.rs + pages_command command.rs; browser_command belongs only to pages_command using one actual pages/browser_command.rs file |
| humanymous-lab-runner | src/qualification/lab_runner/mod.rs plus only canonical shared lab support files it actually reads; no attacker/defender by default |
| humanymous-runtime-pilot | src/qualification/runtime_pilot/mod.rs plus canonical attacker+defender and its actual shared lab inputs; preserve Unix/test cfg branches |
| humanymous-cef-pilot | src/qualification/cef_pilot/mod.rs with existing desktop/diagnostics/provisioning; no actor module |
| humanymous-native-runner | src/qualification/native_runner/mod.rs with existing actor/controlled/evidence/fixture/harness/pilot/service/workspace and cfg(test) prebuilt source; actor references become crate::native_runner::* ONLYhere |
| humanymous-docs-review | src/qualification/docs_review/mod.rs only, current single-file parser/command behavior retained |
| humanymous-local-parity | src/qualification/local_parity/mod.rs only, unchanged workload and root/control provenance |

Each bin root is a narrow composition entry and links actual in-package files with
#[path]. Private mod.rs or split command.rs owns implementation; expose only a parent-
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

All12old integration roots and service_evidence example are named/hashed in
the census; two old roots now map to bin-owned modules. The remaining10aliases are cli-watchdog-boundary, cli-service-process,
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
filesystem-resolved. Root a070983e and Delivery9cb6e435 now ACCEPTall35row
ownership categories/destinations (34distinct paths). tools-input-dispositions-
accepted.json mirrors the exact decisions/pins/readers. There is no remaining
generic requirement for Root to decide every external input. Actual reader/caller/
source-pin/role/test closure is still NOT_FROZEN; the table is not a compiler read
census or source/resource move grant.

| Accepted rows | Final ownership direction and remaining exact freeze check |
| --- | --- |
| L01,L02,L18,C01-C10 | Keep original agent/skill/script canon; actual repository checks use explicit finite source-root manifests and bounded readers; synthetic parser units remain distinct |
| L03-L06 | Move original adapter contracts to qualification/adapters with exact bytes and atomic callers |
| L07 | One browser-owned CEF DATA replica at qualification/browser/cef/pilot-contract.json; preserve producer9251376f/blob4a134d4f/3098bytes/SHAeac4fab4, provenance/license and equality; no fixtures/cef-desktop substitute |
| L08,L10-L15 | One pinned Collector DATA replica qualification/fixtures/collector/pilot.json and six original-filename Core scoring datasets qualification/fixtures/core/scoring; producer hash/license/equality before freeze |
| L09 | qualification/pins/collector-runtime-pilot-source.json DATA pin and accepted d49c19bc actual selected-source binding, no Collector implementation copy |
| L16,L17 | Keep Docker recipe/standards corpus canon; bounded explicit readers retain real recipe verification and25corpus rows/5accept/20reject |
| L19-L21,L23,L25 | Move each original shared Lab Rust file ONCEto src/qualification/lab_support/<basename>, not a new shared subdirectory; all roles link actual same files |
| L22,L24 | Move browser-wire-contract.json/go-configuration.json to qualification/lab unchanged with atomic reader/config/source-pin updates |

Collector d49c19bc is modest trusted-checkout drift detection: explicit frozen
root and Compose context association, private selected record from same freeze
bytes,16KiB bounded source read, exact digest/UTF8/line checks and revalidation
before refinement. Missing/drifted required source refuses checkout qualification.
Keep source_prefix_authenticated=false and unchanged Origin/policy/assertion/
diagnostic fields; no hostile-race/nofollow/compiler-read/native-custody claim.
Snapshot public paths or retained JSON are not authenticated authority.

Remaining checks are concrete implementation/source joins: exact bounded reader
calls/caps/absence/race rules, actual producer bytes/license/equality, unchanged
profiles and canonical source links, final per-role tests and artifact admission.
Accepted ownership does not waive any gate or authorize independent moves.

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

## Legacy LIB unit inventory handoff

OLDWindows semantic discovery is now observed under Root allocation on immutable
5d68cd39, [[evidence:root/20261107235307]]: NativeCLI11, Attacker27, Defender9,
Maintenance76, Pages37, CLI26registered names,186total, exact name differences0.
All187case-source hashes match actual B's canonical source; the one absent lexical
case is the expected Unix-only CLI permission test. Test functions executed0;
ignored native names are not passes. OldLinux/nonWindows lists, all15BINsemantic
lists and new private-bin selector/role collection remain unobserved.
Post-analysis also matches all259BINcandidate source pins against the retained
3128-source-before snapshot with0hash mismatches; this is not BINsemantic discovery.

The completed machine artifacts are legacy-lib-unit-map.json and the separate
legacy-bin-unit-map.json. All15MAIN/BINroots are now included rather than omitted:
259source-target candidates from their declared module closures, no unresolved
lexical module/function locations. Combined with187LIBcandidates this is446old
source-target collections but440physical source/function candidates; six shared
source collections are not new coverage. These are not semantic or pass counts.

The exact Wargame5bcce859 source reconciliation is recorded per case. Attacker
LIB27does not include its separate3main/bin argument tests:25nonignored units
(5seed/catalog/report +5network +7verification/JSON +8legacy parity), one portable
ignored native-publication case at report/legacy/tests.rs:235 and one Windows-only
ignored sharing-denial case at:276. Thus static registered candidates are Linux26
and Windows27, while25is the nonignored source category, never a measured pass
count here. Defender's ninth source case is lab/isolation.rs:116,
host_cannot_enable_network_mode, cfg(not(target_os="linux")); Linux8/nonLinux9.
Ignore flags, platform cfg and actual native effects remain unchanged.

| Existing bin root | Lexical own-bin source collections |
| --- | --- |
| humanymous-native-runner | 61 |
| humanymous-cef-pilot | 14 |
| humanymous-runtime-pilot | 9 |
| humanymous-lab-runner | 40 |
| humanymous-pages | 1 |
| humanymous-delivery | 121 |
| humanymous-agent-maintenance | 0; former LIB76and relocated10are separate required inputs |
| humanymous-service-fixture | 0; actual fixture role remains required |
| humanymous | 1 |
| humanymous-docs-review | 8 |
| humanymous-native | 0; former LIB11remains required |
| humanymous-report | 1 |
| humanymous-attacker | 1 |
| humanymous-wargame-verify | 1 |
| humanymous-local-parity | 1 |

The bin map follows real mod/path declarations, only immediately adjacent path
attributes, and retained inherited cfg. External library dependency units are NOT
treated as old-bin units; actual shared source included directly by an old bin is
recorded under that bin. Zero lexical cases does not waive executable/role proof.

New command-module prefixes for the attacker/report/verifier entry roots are
proposed attacker_command/report_command/wargame_verify_command, avoiding collision
with canonical shared attacker/defender or legacy namespaces. Native, Pages and
Maintenance command prefixes are native_command/pages_command/maintenance_command;
their LIBnamespaces remain native/pages/maintenance. Other single-context bins
retain their role prefix; primary humanymous retains commands:: names.
Root must approve these precise command namespaces alongside semantic lists; no
source, target or visibility change is performed by this proposal.

legacy-lib-unit-map.json in [[evidence:root/20261107200932]] is the source-bound
case/collection map, separate from the accepted33Maintenance cases. It records
187unique source/function candidates:160across the five retiring tool libraries
and27in the retained production CLI library. Counts are lexical, not semantic
Cargo inventory or executed test denominators.

| Original LIB | Source cases | Primary new collector |
| --- | --- | --- |
| native-cli | 11 | humanymous-native, native::<old-selector> |
| attacker | 27 | humanymous-attacker, attacker::<old-selector> |
| defender | 9 | humanymous-attacker, defender::<old-selector> |
| agent-maintenance | 76 | humanymous-agent-maintenance, maintenance::<old-selector> |
| Pages | 37 | humanymous-pages, pages::<old-selector> |
| retained CLI | 27 | existing CLI lib selector unchanged |

Each row pins old package/target/source SHA256/attribute and function line,
source-derived old selector and exact proposed new target-qualified collections.
There are0candidate selector collisions and0unresolved attribute/function pairs.
126module declarations retain parent cfg/path observations for semantic review.
Source layout/indentation is not a Rust parser; module resolution/platform cfg
and actual test registration remain explicitly unresolved pending --list.

Canonical attacker/defender cases may also collect from their actual shared source
in the verifier and legitimate Unix runtime-pilot role. The10legacy report leaf
cases additionally map to legacy::<suffix> in humanymous-report. These rows are
repeat collections of the SAMEsource case, never new unique tests or substitutes
for its primary owner. No source/test implementation is copied or deleted.
Root must freeze the real bin/module cfg and exact per-target list before execution.

The76Maintenance LIBcases do not include the33accepted integration/bin-relocation
cases. Its complete owning binary inventory must retain all applicable library
cases plus10actual filesystem/provider cases and any preexisting main-bin unit
cases, while the23integration cases keep their exact executable boundary. No
category is automatically pure. Only the historical CLI pure11 has prior effect
review; other candidates need their actual filesystem/provider/runtime classification.

Concrete unresolved items in the machine map:

- Lexical CLI27includes conditional-platform source; historical Windows actual
  inventory26is not a reason to drop a candidate. Resolve inherited cfg by --list.
- Repeated shared-lab collection and preserved local test modules need exact
  target-qualified inventory and unique-source accounting, not inflated totals.
- Existing standalone/main-bin unit tests are now fully inventoried separately,
  with all259lexical source-target cases retained; their cfg/semantic names and
  exact private command prefixes still require owning --list reconciliation.
- Proposed module prefixes are not semantic discovery; external include/input
  reader/caller/source-pin joins must close before the new private-bin graph can compile.

The machine map includes exact nonexecuting discovery commands: six OLDlibrary
selectors use cargo test --locked --offline -p <old-package> --lib --all-features
-- --list; NEWselectors use the CLI lib plus exact --bin for native, attacker,
report, verifier, maintenance, Pages and legitimate runtime-pilot. A later
Root allocation must pin old/new immutable graphs separately, enforce one context
90s work/30s cleanup within a selected390s/450s batch, jobs1/inc0, fresh guards,
exclusive lease and before/after source/tool/config/env hashes. Stop first failure
or inventory drift, no retry/source fix/test execution. --list may compile and
spawn the harness but must not call a test function or native/service/SDK pilot.
The original source-only task ran none. The subsequent separately allocated
OLDWindows six-LIB --list run above completed; no new-bin command is authorized.

1. Accepted15bin names/features/default artifact policy stay fixed. Root reviews
   remaining test aliases/minimal feature selectors, per-bin shared module
   linkage, preserved bin-only public namespaces and dead-code handling. Resolve
   exact accepted input readers/callers/source pins and private role joins before
   freeze. The35ownership decisions are already accepted; do not reopen them.
   No public production tool API or implementation copying.
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
