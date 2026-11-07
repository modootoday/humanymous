---
id: plan:root/20261107214557
title: Four standalone qualification tools into CLI handoff
status: active
created: 2026-11-07T21:45:57Z
updated: 2026-11-07T21:45:57Z
rel:
  depends_on: [sot:root/structural-development, sot:root/delivery, plan:root/20261107205157]
---

# Scope and census

Root requested read-only preparation for absorbing all four maintained standalone
tools into CLI qualification contexts. Root owns the accepted design and
every move, manifest and lock edit. No permanent private helper package is added.
The accepted final target remains eight maintained packages, including these tools.

At the initial Platform handoff, the Git census was39 Cargo manifests:34 main-workspace packages, four standalone
packages and one virtual root. That maintained package total was38, not34.
The original main-workspace41 plus four standalone packages was45 maintained
packages. Earlier34-member metadata receipts remain valid for their stated main
workspace scope; they do not become complete maintained-package inventories.

Root subsequently consolidated six Runtime contexts at64f74c7a. The declared
workspace now has29 members plus the same four standalone packages:33 maintained
packages and one virtual root. The original standalone-tools-handoff.json is the
initial source/hash census, not a refreshed post-Runtime source receipt. Its old
dependency names and source hashes must not be represented as current tool bytes.
The four-tool absorption remains pending; Root's maintained-package progression
is45 initial,39 Core,38 Platform,33 Runtime,21 Tools,10 Adapters and8 Browser.

The source/hash list is attached under [[evidence:root/20261107052001]] as
standalone-tools-handoff.json. Source test-attribute counts below are static
observations, not actual executed inventories or qualification results.

# Tool contracts at initial handoff

| Tool/package | Sources and entrypoint | Dependencies and unsafe policy | Current qualification surface |
| --- | --- | --- | --- |
| cef-pilot / humanymous-cef-pilot | Eight Rust files; src/main.rs; OUTPUT_DIRECTORY or --configure-toolchain | Platform/testing, serde_json, sha2; forbid unsafe | Windows x64 real GitHub-hosted runner only, ACT refused; protocol preflights, bootstrap security-override rejection, two desktop process cases; safe diagnostic projection and private report |
| windows-native-runner / humanymous-native-runner |28 Rust files; src/main.rs; native/workspace/service/summarize modes plus Windows actor/control modes | Platform/testing, native-security, serde_json, sha2, getrandom, zeroize; Windows-only windows-permissions/windows-sys; deny unsafe with exactly two explicit module allowances | Restricted Windows actors, Job Objects, account/token/SCM work; cross-OS workspace discovery/accounting, service summaries and source custody; actual process/provider requirements remain separate |
| docs-review / humanymous-docs-review | One Rust file; src/main.rs; optional repository-root argument defaulting to dot | serde derive, serde_json, sha2; forbid unsafe | Read-only docs/reviews.json verification against governed/tracked source and12 declared reader surfaces; changed/missing source or evidence refuses release |
| local-parity / humanymous-local-parity | One Rust file; src/main.rs; environment-bound controller | Standard library only; forbid unsafe | Exact current control source, exact separate historical workload, one/two measured passes, explicit tool/runtime/resource/source observations; not product qualification |

All four are Rust1.99.1/edition2024, version0.1.0, Apache-2.0, publish=false and
declare their own empty workspace. Their independent lockfiles currently belong
to the five-lock advisory closure together with the main lock.

# Unsafe and authority isolation: accepted design, freeze still required

The native runner's only explicit unsafe allowance roots are src/actor.rs and
src/actor/scm.rs, both beneath Windows-only actor wiring. They already perform
real account/token, process/Job Object and SCM operations. The other three tools
forbid unsafe. No actual operation was executed during this survey.

Current CLI inherits the workspace unsafe_code=forbid policy. Copying the native
controller below that forbid cannot compile. Moving it into Platform/testing
would violate that context's forbid rule and capability purpose. Broadening the
workspace lint or exporting raw controller authority is not a migration solution.

Root's accepted qualification-target Rust-crate/lint boundary must
keep production CLI library/application roots forbidding unsafe, preserve the
two existing controller allowances without expansion and exclude the controller
from default/public release targets. Any necessary package-local lint exception
needs an explicit accepted decision and actual negative compile/release tests;
only the bounded Root decision authorizes the exception. Distinguish a Cargo
package from each bin/lib Rust crate when evaluating that boundary. Do not use cap-lints or feature-wide
allow settings as a hidden substitute.

## Accepted target-isolation contract

Root accepted this design in d45e600d9930c2707885ec28ceef97614d55057a and
[[sot:root/structural-development]]. The workspace forbid remains unchanged.
CLI's explicitly local baseline is deny, missing_docs warn and Clippy all warn.
Production lib/bin and every safe qualification bin root forbid unsafe. Only the
separate native-runner bin uses deny and the two existing Windows actor allowance
roots. The four existing executable names, autobins=false, default-off required
features, exact production release exclusions and target-based role admission
are accepted controls. Implementation and verification remain pending the Root
atomic freeze; this acceptance is not proof of compiled isolation.

Keep the four current executable names as explicit internal CLI-package bin
targets, not packages or new default subcommands. Give each its own required
qualification feature/profile under the accepted Root design.
Keep autobins=false and defaults free of qualification features. In particular,
the native runner stays a separate bin Rust crate, so current_exe self-reexec can
retain its own exact mode parser rather than entering production CLI dispatch.

Cargo lints are package-level, not a per-bin table. A viable explicit exception
therefore needs CLI's local package baseline deny plus crate-root forbid on the
production library, production executable and every safe qualification bin.
The workspace forbid setting and other packages remain unchanged. Only the
native qualification bin has a deny root; every safe sibling module inside it
must itself forbid unsafe, while the existing two Windows actor allowance roots
remain its only exceptions. No controller module is included in the public
library or production main. This package-local exception is an accepted
design delta, not an assertion that current CLI already implements that scheme.

Make native controller dependencies optional/profile-gated and Windows-targeted.
Check absence of the new qualification-only capabilities in default dependency
features, accounting for legitimate pre-existing Windows service dependencies.
Reject package-wide allow, cap-lints, shared public unsafe helpers, or an all-feature
production release. Explicit release target/feature allowlists must exclude all
four qualification bins even when their source ships in the public CLI package.

Required negatives: unsafe in each production/safe crate or safe native sibling
must fail compilation; no default installed qualification executable; an unknown
CLI target sharing the package ID cannot obtain a native helper role; forbidden
feature/binary inventory cannot pass release admission. Actual controller probes
remain separately authorized, source-bound and Windows-native. Root must freeze
this lint/profile/release implementation atomically before any source movement.

# Fixtures, profiles and dispatch that must move together

- CEF's compiled include points to the desktop pilot-contract.json. Provisioning
  checks browser/bootstrap/desktop/shared, Platform and nested crate inputs;
  preserve the new nested Core census and artifact hashes. Keep source-before/
  after, actual host admission, toolchain configuration, bounded compiler and
  bootstrap diagnostic parsing, four security-override rejection process cases
  and private/public output separation. Do not promote protocol or desktop
  component success to browser_ready, GUI/IME/AX or full runtime qualification.
- Native's current_exe self-reexec and mode parser are a protocol. A shared CLI
  entrypoint must explicitly route every parent/actor mode and preserve absolute
  report/binary paths, source/SID checks and unknown-option refusal. Keep guarded
  user/account/SCM cleanup, Job Object and cancellation outcomes intact. A generic
  CLI dispatch fallback must never select a privileged qualification mode.
- Native service catalog selects the service_process and runtime_service_process
  tests with CLI/service-pilot and CLI/runtime-service-pilot features; the latter
  case is Windows-only. Preserve fixture target identities and expected evidence
  flags. Ignored real product cases must not be reclassified as subprocess helpers.
- Native Cargo IDs already join exact same-source metadata. After absorption many
  targets share CLI's package ID: identity must also bind the exact target/source,
  kind, role and selected profile. CLI-name or ID-prefix matching cannot admit all
  qualification binaries. Update the Delivery native archive's explicit approved
  path anchors in the same reviewed move; its two release roles stay unchanged.
- Docs review owns its12 exact surface/check mappings and read-only behavior.
  Update paths only through a concrete review of moved source and caller commands;
  do not regenerate hashes to declare unreviewed documentation current.
- Source contains14/60/8/1 test attributes respectively. Conditional compilation,
  ignored/native cases and generated child fixtures prevent treating this as an
  executed83-test inventory. Fresh profile-specific listing/execution is required.

# CWD, targets and historical source boundaries

CEF provisioning, Native metadata/source discovery and many spawned Cargo commands
assume repository-root CWD. Native records its independent runner lock and exact
source inputs. Docs review accepts a root explicitly. Local parity derives control
root from CARGO_MANIFEST_DIR ancestors and checks literal controller/workflow/catalog
paths. Root must replace these assumptions deliberately when moving the source;
an unchanged ancestor depth is not a reviewed root-discovery contract.

Preserve explicit target directories and profile input keys. Do not blend old
standalone target artifacts into new CLI role proof. Replace the runner-lock field
and advisory-lock inventory with a reviewed single-root-lock meaning rather than
silently filling an old independent-lock field with a different value.

Local parity's current push caller requests workload8132117e7bf1445939d7dee9066fcf45ccf0915b
and two passes. Its current controller and detached historical workload are two
different source identities. The workload runs collector-lib26, workspace-all-target
build and runtime native_process1 selections under the pinned Linux container.
Keep the original source, command counts, cold/repeat distinction, timing/resource
scope and before/after checks. Moving the controller does not qualify current CLI
from that old workload, authorize rerunning it or permit editing its checkout.
The separate981d Go oracle remains Git-only; do not confuse those two baselines.

# Current callsites and required migration gates

Primary current callers are cef-native-component.yml, cross-os-probe.yml,
rust-validation.yml and local-parity.yml; Delivery pilot.rs, workspace.rs and
service_component.rs also select the standalone Native manifest. The source census
records those selectors. Update current workflow commands, path filters, catalog,
source readers, fixture grants, Cargo metadata target matching and CLI packaging
as one Root-owned slice. Historical receipts/catalogs and their source paths stay
immutable; new candidate readers need their own bindings.

Root must define exact internal qualification targets/features before writing:
default CLI and installed release targets exclude them; selected safe tool
profiles compile without native controller authority; Windows-only control
profiles require both the explicit feature and real OS/provider admission.
Cargo all-targets alone will skip targets with unselected required features, while
all-features can overenable roles. Neither replaces explicit required profile gates.

Required verification includes source relocation byte binding; default dependency
and production unsafe-negative checks; each selected profile's inventory/unit/
strict Clippy/fmt gates; legitimate target/metadata joins and foreign-role negatives;
actual approved Native/CEF process profiles; unchanged read-only docs behavior;
complete advisory lock coverage and the full workflow policy/catalog. Product
claims still require Docker-only owning e2e and their exact source/image receipts.

Root's accepted structural contract additionally requires an exact unsafe-site
inventory and coverage of every crate root, including build scripts, examples and
integration tests. Verify transitive feature isolation, final release executable
inventory and exact target-role metadata joins. The two existing Windows actor
module allowances do not authorize other roots, targets or transitive features.
These are required migration gates, not evidence that migration has occurred.

The later literal compile-include audit found15 Delivery callsites reading10
operational JSON contracts outside the Delivery package, including test sites.
cross-package-contract-census.json records their reader and input hashes. This
is not a complete compiler input closure. Gateway and historical Audit includes
were removed separately, but that does not make the remaining package independent.
Before Tools freeze, Root must resolve ownership of these operational contracts
and bind the selected extracted tool's compile/test inputs. The Header canonical
profile is a fixed producer/proof input: replacing its compiled pin with an
ambient filesystem reader would need explicit contract review, not a mechanical
path edit. Do not copy the full Audit corpus, add a public fixture API or hide the
boundary behind a permanent helper package. Historical profiles remain immutable.

## Root operational ownership decision and all13 map

Root02d8aedf accepted the ten-original operational resource table in structural
SoT. Move those originals into CLI qualification resources at the atomic stage;
preserve their frozen original byte/Git/hash bindings. In particular, Header R3
stays same-byte compile embedded from the owning CLI package. Each mapped reader
can use the exact package-relative compile expression recorded in
tools-operational-ownership-handoff.json independently of its final private module
layout. This is an accepted ownership direction, not an executed relocation.

The expanded source map observes all13 responsibilities,246 Rust source files,
34 root candidates and61 compile includes:51 literal and10 manifest-concat,
covering35 distinct inputs. It records manifests/features/current target roots,
the ten originals' Git objects and hashes, observed callers/source-pin references
and remaining include classifications. These are static source observations,
not Cargo execution inventories or complete generated/transitive closure.

Remaining ownership decisions include CEF desktop's pilot contract, Lab browser
wire and Go configuration data, and the public Docs normalized-profile corpus.
Core's six frozen cases and Collector's pilot fixture need the accepted data-only
replica provenance/equality process and explicit destinations. Collector's Rust
implementation currently embedded by Lab panic-site classification must not be
copied: the replacement needs a reviewed owned digest and actual same-source
binding. Mutable .agents records/rules, agent scripts, skills documentation and
the product Docker recipe stay at their original owners; parser units use small
synthetic fixtures and live checks use bounded explicit repository readers.

Maintenance qualification0.1 hard-codes its former package, empty feature vector
and eight integration targets/33 cases. CLI target/feature consolidation therefore
requires an explicit reviewed owner-contract delta; do not retain a misleading
package alias or simply lower the inventory. More generally, current integration
tests import former tool libraries. Private binary contexts cannot be exposed
through the production CLI library just to preserve those imports. Root must fix
the new test ownership/target matrix before source movement: including private
implementation modules directly in integration roots can duplicate their cfg(test)
unit inventories, while regrouping tests changes harness identity. Require exact
discovery and all semantic/process obligations under the reviewed new profile,
without silently filtering unknown cases or transferring old test counts.

tools-callers-profiles.json complements the resource map with all13 manifests'
dependency/feature declarations,15 existing binary source candidates,379 lexical
package/crate/manifest caller references, source-root and self-executable anchors,
and the exact two current unsafe allow locations. Attacker has both report and
wargame-verify binaries in addition to its main actor:13 responsibilities does not
mean13 executables. These are source candidates/references, not observed Cargo
targets or semantic unsafe-operation review. Root must bind the final explicit
binary/feature/test matrix, production transitive graph and release inventory.

# Critique and readiness

The design is accepted; implementation remains needs-work until the atomic freeze,
source wiring and executable/unsafe-negative/release gates are actually verified.
P0 blockers are lost controller isolation, ambiguous self-reexec/role identity and
forgetting four maintained packages. P1 risks are incomplete feature coverage,
changed cwd/source custody, incorrect lock meaning, unreviewed docs and promoting
historical benchmark output. The requirements above reflect these findings.
This is one read-only author review, not an independent panel or migration proof.
No source, manifest, lock, workflow, profile, target, actor or compiler was changed
or executed by this handoff. The independent sole full-CI watch96123 later ended1:
source1f87 run42415763189 completed with5 success,13 failure and1 cancelled jobs.
hosted-1f87-terminal.json binds all19 original job members and the sealed private
archive index. No live hosted watch or local compiler lease remains at that
checkpoint; this terminal evidence does not freeze or qualify the Tools move.
