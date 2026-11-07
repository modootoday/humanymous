---
id: evidence:root/20261107200932
title: CLI installation package transition handoff
status: active
created: 2026-11-07T20:09:31Z
updated: 2026-11-07T22:59:05Z
rel:
  depends_on: [sot:root/cli-installation]
files:
  - {path: owned-paths.json, sha256: a439efb3738bd73836ab2b056a9fba09b972e544941f4f5ab2d7ee1776dea109, bytes: 8375, store: repo}
  - {path: tools-role-map.json, sha256: 342f08f560b0b401d4cedd185232e75eaf5e5cc2c99701ba34b96b7a4bd4f1ce, bytes: 8719, store: repo}
  - {path: runtime-slice02-inputs.json, sha256: bf3657232be606d662433c31ae0c86acaeb8abed740910fd77d2071dcda35396, bytes: 4715, store: repo}
  - {path: runtime-slice02-formatted.json, sha256: 0583ceb99f97c8d2f2ffa2449220df0ab73cca3371cc44a32cc88bc05ab7a48e, bytes: 4763, store: repo}
  - {path: tools-freeze-census.json, sha256: 7e7765d28b232f5760235c715fd7dfe29dc2ca93efed12d12d0f702502e23cce, bytes: 111913, store: repo}
  - {path: tools-source-readers.json, sha256: d6f6e022d7991cad3d4a65da7c87e6ee861c1cca9a2400728ec392db69cc1f12, bytes: 22007, store: repo}
  - {path: tools-linkage-map.json, sha256: 790cef2d9f3a8e8bfcd34a20e86afb65c3afd770884184606c97e270514671b9, bytes: 52633, store: repo}
---

## Mission and current ownership

Root requested this handoff after the human accepted the eight-public-package
baseline. Root alone sequences path/Cargo/ref integration. This chat retains
native-cli ownership of crates/cli/**, CLI SoT/plans and CLI evidence until a
specific freeze/transfer. No directory/package/import move, structural worktree,
branch operation, manifest change or publication is in progress. Detection policy
is unchanged. Local services/accounts must not be changed.

owned-paths.json pins32tracked files at observed HEAD8affb45a with SHA256 and Git
blob identity, including all CLI source/Cargo/INTEGRATION and the CLI SoT/two
owning plans. Every recorded worktree blob matched Git. Owned source paths were
clean before authoring this handoff; only this handoff and its plan link are new.
No compiler/runtime process handle, target or artifact-write lease remains held.
The approved baseline does not authorize this chat to edit the common graph.

## Done and source-specific verification

RUNTIME-SLICE-02 mechanical CLI consumer change is recorded separately in
runtime-slice02-inputs.json:17Rust src/test files, exact before/after SHA256.
Existing runtime roots map to humanymous_runtime::gateway before other old
package names map to humanymous_runtime::{authority,audit,native_security}.
No behavior/role/authority/unsafe change, aliases, Cargo edit or source move.
The new graph's compile/test/Clippy/fmt gates remain NOT_RUN. Root owns context
features and all common/dependency manifests. Historical checks below retain
their old source-specific scopes.

CLI DATA sourcef20db16e consumes shared Authority checked DATA and Runtime
AuthorizationData without recreating cryptography, copying ownership or issuing
native authority. Corrected receiptcf64468c/evidence:root/20261107192208 records
Windows fmt-first/all-feature/all-target check/strict Clippy and exact11pure
passed/15filtered, with complete507local/root Git joins and7239stable input rows.
Explicit cache reuse is retained. Independent auditdb60bfe8 closed the scoped
CA-CLI-INPUT-01 correction. Old6674-row proof remains HOLD; no backdating.

Actual watchdog component91536cce plusa7cab103 records the cooperative four-case
Windows process boundary and outer-Job cleanup. It is not real audit flush,
service admission or product qualification. Runtime's separate DATA receipt
7bdda110/six signed-fixture tests remains its owner's proof. All these results
belong to their historical source/path/feature closures and cannot be inherited
by renamed packages or a changed dependency graph.

## API, visibility and dependency hazards

The final pre-freeze target/private-context proposal is
[[plan:root/20261107224511]]. tools-freeze-census.json records13manifest dependency
category bodies,34compiled-root candidates (6libs/15bins/12integration/1example),
279tracked files with Git mode/blob/hash,10resource source/destination pins and
the existing actor unsafe lexical sites (26+7). tools-source-readers.json retains
123include/manifest-root/executable selector locations. Root's accepted private
qualification/resource ownership rules apply; final feature spellings/paths and
the five retired-lib private shared contexts remain explicit Root decisions.
No new public tool APIs, helper crates, source copies, moves or compile proof are
introduced. This census is read-only, pre-freeze and not Cargo metadata authority.

tools-linkage-map.json now maps all34old roots to standard-src targets or private
contexts, exact proposed test aliases/minimal selectors and required lints. Root
accepted15bin names, qualification-* and native-planner spellings/default-off,
default release ONLYhumanymous and explicit native-planner artifact policy.
Canonical shared attacker/defender source is src/qualification/lab_support; bins
link the same files with reviewed in-package path modules, not copies or lib
exports. The plan specifies each bin's minimal modules and per-warning dead-code
review; existing public tool namespaces may remain binary-only without exposing
new private capability fields. No blanket lint suppressions are proposed.

The expanded source reader review found25literal external inputs plus10resolved
manifest-relative includes. Collector implementation path, shared lab files,
mutable registry embeds, frozen Core/Collector replicas and remaining dynamic
repository readers need Root/owner dispositions. Whole34root+resource closure is
NOT_FROZEN. Root must route this detailed mapping for X review before any move.

Root-approved lint-isolation amendment d45e600d is now part of this handoff.
tools-role-map.json records all13absorbed responsibilities: the9workspace roles
plus4standalone qualification tools. The old owned-paths.json remains the original
32-file observation, not a refreshed Tools freeze/build closure. Root's common
manifest integration is in flight; the new role map's hashes are observations.

| Preserved standalone binary | Current source context | Accepted isolation |
| --- | --- | --- |
| humanymous-cef-pilot | scripts/ci/cef-pilot | own default-off required feature; safe root forbids unsafe |
| humanymous-native-runner | scripts/ci/windows-native-runner | own default-off required feature; root denies unsafe, only actor.rs and actor/scm.rs existing allows |
| humanymous-docs-review | scripts/ci/docs-review | own default-off required feature; safe root forbids unsafe |
| humanymous-local-parity | scripts/ci/local-parity | own default-off required feature; safe root forbids unsafe |

All four retain names as explicit binary targets with autobins=false. Root alone
will assign feature spellings/manifest paths. Workspace unsafe_code=forbid stays
unchanged. Only the combined CLI package may use local deny/warn lint settings;
production lib/bin roots and every other safe qualification root explicitly
forbid unsafe. Native runner's isolated root uses deny and retains exactly the
two existing Windows actor module allows. No actor module is promoted to platform
testing or linked from production. Package/feature identity grants no helper role.

Release commands select production binary names/features explicitly; default
build/release inventory excludes the four qualification executables and all
fixture/helper artifacts. Required controls are per-target production/qualification
compile matrices, forbidden-unsafe negative probes for safe roots, source census
of exactly two approved allows, and actual produced/released artifact inventories.
Compile unsafe-negative probes as isolated failing fixtures, not product unsafe
source or suppressed workspace lints. These are future required controls, NOT_RUN.

Unresolved before the Root Tools freeze:

1. Safe context API/visibility and exact paths for native CLI, attacker/defender,
   maintenance, Delivery, Pages, lab runner/runtime and the four qualification
   roles; defender is currently a library, not a new fabricated binary. Resolve
   shared crate-root references without making role dispatch globally available.
2. Explicit default-off feature names, production install/release target allowlist
   and dependency cfg/categories for Windows runner, CEF, Docker/SDK and audit
   contexts. No all-features release or propagated actor capability to siblings.
3. Fixture packaging: service-fixture retains service-pilot opt-in; watchdog uses
   its exact CARGO_BIN_EXE artifact. CEF embeds browser/native/desktop/
   pilot-contract.json by relative include; assign the new root-owned fixture
   path and release exclusion without copying an outdated contract.
4. Unique test/example target names and all CARGO_BIN_EXE rewires, including native
   CLI/maintenance/service and watchdog tests. Native runner's current_exe
   self-reexecution must still dispatch only its own verified actor role.
5. Root-directory discovery: local-parity CARGO_MANIFEST_DIR ancestor calculation,
   Delivery archive/image context roots and provenance helpers cannot retain old
   package-depth assumptions. Preserve original parity workload and standalone
   lock advisory coverage when their lock files cease to be independent packages.
6. Requalify helper binary path/hash, source/fixture inclusion, bounded controller
   grants, original watchdog deadlines and held-reader controls after any move.
   No native/service/account pilot is authorized by the structural decision.

- Current package humanymous-cli has public lib modules release/service and an
  optional runtime_host, private installation with re-exported Plan. Its binaries
  are humanymous and feature-gated humanymous-service-fixture; autobins=false and
  publish=false. Preserve command/artifact names unless Root explicitly changes
  their external contract. Browser-owned tooling is a separate owner.
- runtime_host exposes HostPlan/HostMode/ObserverPlan/StopPlan, SecretSource,
  NativeSecrets and HostAuthorizationData. The last type's fields remain private;
  into_parts/into_runtime_data move the same Configuration and checked DATA.
  They grant no NativeProof, Ready, secret, listener or ServiceHost activation.
  Runtime's static AuthorizationDataError is intentionally propagated. Feature
  and public re-export identity must survive the root-sequenced transition.
- Existing shared edges are native-security, Authority, Runtime and optional
  Audit; test-support is a dev dependency. Preserve workspace ed25519-dalek,
  hex/serde/serde_json/sha2, optional tokio/zeroize and pinned Windows
  windows-service0.8.1; Windows dev dependencies are windows-permissions0.2.4 and
  junction2.1.0. Root owns Cargo.lock/common graph. No new helper crate or
  compatibility wrapper is proposed.
- service-pilot and runtime-service-pilot are opt-in fixture surfaces; default
  release excludes fixture writes. runtime-service admits shared Runtime only.
  Keep platform cfg boundaries and existing shared native-security implementations;
  do not copy ACL/token/secret/unsafe FFI into the CLI. There is no requested unsafe
  expansion. Status credentials never acquire stop/kill/erasure authority.
- Exact raw BASE bytes, executable/configuration SHA256, service name, scope and
  cap remain signed content bindings. HostPlan observer v1 selectors cannot be
  treated as v2 leases. Root must coordinate serialized API/path changes with the
  Authority/Runtime/native owners, not insert speculative shims.
- Integration tests bake CARGO_BIN_EXE paths and fixture artifacts. A moved source
  path or feature graph needs fresh build/provenance and actual process proof;
  copied old binaries or an idle target lease cannot authorize the new structure.

## Blockers and ordered next actions

1. Root freezes this specific CLI slice and hands over exact source/path hashes
   after earlier engine/core and runtime/platform slices. No independent move is
   authorized here. Refresh owned-paths hashes at that actual freeze.
2. Root supplies the accepted owning API/dependency paths. Change only the assigned
   CLI imports/manifests in serial integration; preserve authority/visibility and
   fixture gates. Current native preparation remains Unsupported with no accepted
   issuer/delegation/P-S-O retained-instance/atomic-claim/W/transport producer.
3. Requalify the new actual graph and source closure under a fresh root resource
   allocation. Signed CLI-to-Runtime end-to-end composition remains a distinct
   future test; present caller proof is type-check plus pure boundary regressions.
4. Genuine-hosted service lifecycle must use the dedicated delivery/Windows runner
   component, with exact source/run, both cases, all13facts and actual cleanup.
   No local SCM/account fallback. Production observed ServiceHost cannot be
   activated by DATA-only checks or standalone component success.

## Required verification and limits

On the root-approved new graph: package fmt first; locked/offline CLI all-features
all-targets check; strict Clippy with -D warnings; --lib runtime_host:: -- --list
then the reviewed exact11pure cases (baseline15filtered). Reject inventory drift
until reclassified. Full CLI library/service tests touch files/ACLs/ports and are
not a pure profile. Unit fixtures/mocks/ignored cases are not actual native proof.
After artifact/path changes, actual watchdog needs the allocated controller and
fresh binary/path hashes. Service/account pilots are genuine-hosted only under
this user's restriction. Full product e2e remains Docker-only and separately
allocated; no product pass follows from these component checks.

Read AGENTS.md, .agents/lessons/HARD-WON.md, crates/cli/INTEGRATION.md, CLI SoT,
CLI installation/observer-lifecycle plans, structural-development SoT and the
corrected receipt before editing. Codex retains this chat's residual state; the
durable files and session board are cross-provider authority. The lane is still
active for non-overlapping work, not released as an abandoned session. Root must
coordinate its explicit transfer/freeze and corresponding board release.
