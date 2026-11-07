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
tools into CLI qualification contexts. Root decides target/profile names and owns
every move, manifest and lock edit. No permanent private helper package is added.
The accepted final target remains eight maintained packages, including these tools.

Current Git census is39 Cargo manifests:34 main-workspace packages, four standalone
packages and one virtual root. Thus the maintained package total is38, not34.
The original main-workspace41 plus four standalone packages was45 maintained
packages. Earlier34-member metadata receipts remain valid for their stated main
workspace scope; they do not become complete maintained-package inventories.

The source/hash list is attached under [[evidence:root/20261107052001]] as
standalone-tools-handoff.json. Source test-attribute counts below are static
observations, not actual executed inventories or qualification results.

# Exact current tool contracts

| Tool/package | Sources and entrypoint | Dependencies and unsafe policy | Current qualification surface |
| --- | --- | --- | --- |
| cef-pilot / humanymous-cef-pilot | Eight Rust files; src/main.rs; OUTPUT_DIRECTORY or --configure-toolchain | Platform/testing, serde_json, sha2; forbid unsafe | Windows x64 real GitHub-hosted runner only, ACT refused; protocol preflights, bootstrap security-override rejection, two desktop process cases; safe diagnostic projection and private report |
| windows-native-runner / humanymous-native-runner |28 Rust files; src/main.rs; native/workspace/service/summarize modes plus Windows actor/control modes | Platform/testing, native-security, serde_json, sha2, getrandom, zeroize; Windows-only windows-permissions/windows-sys; deny unsafe with exactly two explicit module allowances | Restricted Windows actors, Job Objects, account/token/SCM work; cross-OS workspace discovery/accounting, service summaries and source custody; actual process/provider requirements remain separate |
| docs-review / humanymous-docs-review | One Rust file; src/main.rs; optional repository-root argument defaulting to dot | serde derive, serde_json, sha2; forbid unsafe | Read-only docs/reviews.json verification against governed/tracked source and12 declared reader surfaces; changed/missing source or evidence refuses release |
| local-parity / humanymous-local-parity | One Rust file; src/main.rs; environment-bound controller | Standard library only; forbid unsafe | Exact current control source, exact separate historical workload, one/two measured passes, explicit tool/runtime/resource/source observations; not product qualification |

All four are Rust1.99.1/edition2024, version0.1.0, Apache-2.0, publish=false and
declare their own empty workspace. Their independent lockfiles currently belong
to the five-lock advisory closure together with the main lock.

# Unsafe and authority isolation: decision required before movement

The native runner's only explicit unsafe allowance roots are src/actor.rs and
src/actor/scm.rs, both beneath Windows-only actor wiring. They already perform
real account/token, process/Job Object and SCM operations. The other three tools
forbid unsafe. No actual operation was executed during this survey.

Current CLI inherits the workspace unsafe_code=forbid policy. Copying the native
controller below that forbid cannot compile. Moving it into Platform/testing
would violate that context's forbid rule and capability purpose. Broadening the
workspace lint or exporting raw controller authority is not a migration solution.

Root must choose an explicit qualification-target Rust-crate/lint boundary that
keeps production CLI library/application roots forbidding unsafe, preserves the
two existing controller allowances without expansion and excludes the controller
from default/public release targets. Any necessary package-local lint exception
needs an explicit accepted decision and actual negative compile/release tests;
this handoff does not authorize one. Distinguish a Cargo package from each bin/lib
Rust crate when evaluating that boundary. Do not use cap-lints or feature-wide
allow settings as a hidden substitute.

## Concrete target-isolation proposal for Root acceptance

Keep the four current executable names as explicit internal CLI-package bin
targets, not packages or new default subcommands. Give each its own required
qualification feature/profile; names are proposals until Root canonicalizes them.
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
library or production main. This package-local exception is a proposed accepted
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
remain separately authorized, source-bound and Windows-native. Root must approve
this lint/profile/release exception atomically before any source movement.

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

# Critique and readiness

Verdict: needs-work until Root chooses names/profiles and resolves unsafe isolation.
P0 blockers are lost controller isolation, ambiguous self-reexec/role identity and
forgetting four maintained packages. P1 risks are incomplete feature coverage,
changed cwd/source custody, incorrect lock meaning, unreviewed docs and promoting
historical benchmark output. The requirements above reflect these findings.
This is one read-only author review, not an independent panel or migration proof.
No source, manifest, lock, workflow, profile, target, actor or compiler was changed
or executed by this handoff. Sole full-CI watch96123 remains independent.
