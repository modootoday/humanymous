---
id: sot:root/crate-architecture
title: Rust crate naming and architecture review
status: active
created: 2026-11-06T23:16:00Z
updated: 2026-11-06T23:16:00Z
verify: manual
rel:
  depends_on: [sot:root/rust-migration, sot:root/delivery, sot:root/adapter-ecosystem]
---

# Crate architecture review

Version 0.1.0, official sources checked 2026-11-07 KST. Research-only review by
three agents, synthesized against the actual 13-package metadata snapshot.
This does not authorize an automatic package rename, merge or publish. Ongoing
key-module workspace integration is a separate implementation checkpoint.

## Findings and recommendation

Current humanymous-* package names, virtual workspace and crates/context roots
are valid Cargo organization. No naming defect justifies rewriting the entire
core. Preserve the current acyclic responsibility graph while implementing the
missing shared application/session state and real runtime consumers. Fix typed
validation and secret ownership gaps before creating new abstraction packages.

There is no Rust requirement to name a project package core, engine or policy.
Core architecture means responsibility and dependency ownership; adding a
forwarding-only crate cannot implement missing capture/session/browser/admin.

## Official requirements versus project decisions

| Topic | Official behavior/convention | Project decision |
| --- | --- | --- |
| Package name | Cargo/registry name restrictions allow hyphens/underscores | Use consistent humanymous-<context> kebab-case |
| Rust identifier | Default library replaces package hyphens with underscores | humanymous-audit is imported as humanymous_audit |
| Binary target | May have a different explicit name from its package | humanymous-cli exposes operator command humanymous |
| Package layout | Cargo.toml, src/lib.rs/main.rs, tests/examples/benches | Context modules inside each package, no unrelated handler dump |
| Workspace | Shared lock, explicit resolver, inherited metadata/lints | Root virtual workspace, edition 2024, resolver 3, user-approved MSRV 1.99.1 (initial survey was 1.88) |
| Crate count/SRP | Cargo specifies no count or file-size budget | Boundaries require responsibility, target/dependency or consumer reason |
| Core naming | Rust core is a standard library, not a domain-layer template | Do not alias a project library to core or imply no_std support |

Cargo recommends sibling workspace packages, commonly under crates/. That is
different from flattening unrelated implementation files within a context. The
existing migration canon explicitly permits lib.rs/mod.rs namespace facades and
requires context modules. Keep that interpretation; do not invent crates/core/*
physical nesting merely to make Cargo membership look less flat. Kebab-case
packages are our convention, not a mandatory API-Guidelines crate-casing rule.
[R1-R5]

Each registry package needs separate owner/name/trusted-publisher verification.
A humanymous package does not reserve every humanymous-* name. Hyphen/underscore
or casing variants are not independent registry namespace claims. This review
did not reserve names or inspect live registry ownership. [R6]

## Current responsibility and dependency map

Arrows below mean consumer -> dependency, normal workspace dependencies only.
Dev edges must not be confused with runtime cycles.

```text
authority -> audit, integrity, network, scoring, signals
audit/integrity/network -> signals
scoring -> behavior, model, signals
model -> behavior, signals
behavior -> signals
attacker -> defender, signals
defender -> scoring, signals
challenge and cli -> no normal internal package dependencies
product parity tests -> test-support (dev only)
```

| Context | Current owner | Judgment |
| --- | --- | --- |
| Shared observations/wire/report/registry | signals | Valid independent contract; no server state or native I/O |
| Feature extraction | behavior | Cohesive schema/transform ownership |
| Inference and model runtime | model | Keep training I/O/dependencies separate when implemented |
| Frozen verdict policy/Engine/trace | scoring | Sole verdict implementation reused by authority/defender |
| TLS/H2/header/TCP transforms | network | Own observation conversion; separate actual transport lifecycle |
| RIT/resource/adapter byte binding | integrity | One strict representation and crypto owner |
| PoW/attestation/Pass mechanics | challenge | Session/counter ownership belongs to application state |
| Records/proofs/WAL/keys/lifecycle | audit | Cohesive audit integrity/custody context; retain contextual modules |
| IPC/auth/scope/replay/admission | authority | Native composition, not completed Core/Gate/browser |
| Provisioning/doctor/rollback | cli | Operator boundary; services/runtime still incomplete |
| Owned-lab scenarios/reporting | attacker/defender | Explicit user-required lab packages; never runtime/npm image content |
| Parity assertions | test-support | Real reuse and dev-only; packaging policy still needs a decision |

No runtime cycle was found. Similar observer names in model telemetry and audit
liveness are different responsibilities and must not become one generic observer
crate. Audit file count or small defender wrapper size alone proves neither SRP
violation nor unnecessary fragmentation.

## Core-level design alternatives

| Alternative | Assessment |
| --- | --- |
| Keep current contexts and complete one shared application/session owner | Recommended immediate path; preserve valid contracts and frozen parity |
| Add humanymous-core as a curated public kernel | Conditional on real public consumer/API and owned orchestration responsibility |
| Consolidate behavior/model/scoring implementations into core modules | Possible pre-publication design, not required by Rust; measure packaging/API benefit against churn |
| Move all observations and scoring into core | Risks forcing parsers/audit onto the whole scoring dependency tree; retain an independent shared-contract boundary when needed |
| Optional humanymous Rust SDK facade | Valid for a curated user API with limited re-exports; does not make private dependencies publishable |
| New crate for every handler/config/observer/helper | Reject artificial fragmentation and forwarding-function duplication |

If a consolidation is selected later, move actual implementation ownership and
remove obsolete parallel packages; do not copy algorithms. Keep frozen scoring,
float32 behavior, wire/schema IDs, audit bytes/signatures and inference vectors
identical. Public item/package moves can be SemVer breaking changes. No rename
was performed by this review. [R7]

## Required application/runtime boundaries

One shared application owner must eventually compose authenticated observations,
session/correlation/abuse state, RIT counters/collector/challenge lifecycle,
validated revisioned configuration and audit-before-effect. IPC authority, Core
HTTP collection and Gate forwarding consume that owner instead of each cloning
the engine or maintaining divergent security state.

Current actual binaries: humanymous, humanymous-authority, humanymous-attacker.
Delivery still requires real humanymous-server, humanymous-gate and browser
humanymous-wasm. These names are proposed/fixed delivery contracts, not existing
implemented functions. Decide their package/bin/subcommand mapping explicitly
before adding shells. The operator CLI command may remain humanymous regardless
of internal package names. Keep admin auth/RBAC/dual-control local and separate
from public browser/adapter routes.

Pure Rust computation is not synonymous with no_std/no allocation. Native IPC,
Tokio, ACL/UID/PID and audit key/WAL dependencies stay outside browser exports.
Audit verifier WASM compilation is not a collector/loader/browser qualification.
Features must be additive capability/dependency selections, never no-audit,
allow-missing-signals or replay bypass release switches. Cross-language IPC is
the current boundary; any C/WASM ABI additionally needs version, ownership/free,
layout, bounds, errors/panics and concurrency contracts. cdylib/staticlib alone
does not supply ABI stability. [R8-R10]

## Concrete improvements, ranked

- P1: Separate startup secrets from authority runtime configuration. Existing
  audit hex input strings survive after decoding because whole configuration is
  moved into Authority. Consume/drop and zeroize startup buffers and give decoded
  key custody one owner, reusing audit::keys instead of another keystore crate.
- P1: Implement the missing shared session/application contract and actual
  Core/Gate/browser runtimes; do not market local IPC as full product protection.
- P1: Fix public package/release closure before publication, as detailed below.
- P2: Expose typed HeaderMode/provenance from integrity::ValidatedRequest so
  authority does not deserialize the already strict-validated JSON again.
- P2: Keep header observation construction in network when multiple consumers
  need it; avoid a catch-all common/utils crate.
- P2: Replace policy only through validated revisioned snapshots; do not scatter
  management mutations across public Engine maps or duplicate hard-rule tables.
- P2: Extend retirement inventory to scripts/gen-demo-keys/main.go and
  scripts/gen-ml-demo-bundle/main.go, which are outside the original source list.

These are implementation/review follow-ups, not a request to weaken scoring or
to expose operational administration before its authorization design is ready.

## Publication and validation closure

All 13 packages are currently publish=false. This is appropriate protection
during migration, not a Cargo violation. Description/public readme/package-content
and notice metadata need completion for chosen public packages. Authority has
four path-only runtime dependencies; registry publication requires versioned
resolvable dependencies. A facade depending on private path-only packages cannot
be published alone as a workaround. Private attacker/defender remain excluded.
[R11, R12]

Cargo permits some path-only dev-dependency exceptions, while current delivery.py
intentionally checks every required path dependency, including versioned
test-support, in the publication closure. That gate is a project policy and is
stricter than Cargo. Do not weaken it just to make CI green. Choose and validate
either a legitimate public test-support contract or a private conformance harness
owning shared differential assertions/corpora, while keeping per-package unit
tests and full repository qualification. Never copy helpers into each package.

Node experimental version and Rust workspace version differ, while release CI
requires one exact release version. Owner/bootstrap/OIDC trust records remain
unverified. Resolve these with the public package allowlist, registry ownership,
metadata, packaged-test closure and exact archive checks before publication.

Any structural change must update imports/versions/lock, package targets, CLI
artifact names, Docker entrypoints, CI required-binary inventory, Node supervision,
docs freshness and source mappings together. Keep wire field names, fixture
expectations and historical identifiers unchanged unless separately versioned.
Require dependency DAG/target checks, frozen parity, startup-secret lifecycle,
same-session shared decision, native OS/WASM, actual Docker product acceptance
and archive tests. No directory-only rewrite counts as completion.

## Critique and sources

Blue preserves one Engine/contract owner. Red requires real runtime trust/state
boundaries instead of naming claims. SRE requires target/feature/package evidence.
DPO requires short-lived startup plaintext and one key owner. Evaluator rejects
core/no_std/browser/publication claims inferred merely from a crate name.
Recommendation: retain the valid 13-context DAG now; implement the concrete
boundary corrections and missing application services; revisit consolidation
only with an accepted consumer/target/publication reason.

- R1 https://doc.rust-lang.org/cargo/guide/project-layout.html
- R2 https://doc.rust-lang.org/cargo/reference/workspaces.html
- R3 https://doc.rust-lang.org/cargo/reference/cargo-targets.html
- R4 https://rust-lang.github.io/api-guidelines/naming.html
- R5 https://doc.rust-lang.org/core/
- R6 https://doc.rust-lang.org/cargo/reference/registry-index.html#name-restrictions ; https://doc.rust-lang.org/cargo/commands/cargo-owner.html
- R7 https://doc.rust-lang.org/cargo/reference/semver.html
- R8 https://doc.rust-lang.org/cargo/reference/features.html
- R9 https://doc.rust-lang.org/cargo/reference/rust-version.html
- R10 https://doc.rust-lang.org/reference/linkage.html ; https://doc.rust-lang.org/reference/items/external-blocks.html#abi
- R11 https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#local-paths-in-published-crates
- R12 https://doc.rust-lang.org/cargo/reference/publishing.html ; https://doc.rust-lang.org/cargo/reference/manifest.html
