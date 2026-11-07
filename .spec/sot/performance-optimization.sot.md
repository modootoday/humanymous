---
id: sot:root/performance-optimization
title: Reproducible build and runtime performance optimization
status: draft
created: 2026-11-07T13:56:43Z
updated: 2026-11-07T13:56:43Z
rel:
  depends_on: [sot:root/rust-migration, sot:root/program-roadmap, sot:root/delivery]
---

# Reproducible build and runtime performance optimization

Document version: 0.1.0. This is a measurement and acceptance contract, not a
claim of measured improvement or permission to execute a workload.

## Problem and scope

Reduce Rust build/verification turnaround and product runtime resource costs
while preserving exact functional, security and accessibility behavior. Study
Cargo/rustc profiles, code generation/linking, incremental compilation/compiler
caches, feature/target/package duplication, invalidation and CI trust; Windows/WSL
storage and workflow critical paths; and Runtime, Authority, Audit and collector
allocations, RSS, throughput, latency, cancellation, bounded I/O and cleanup.
Root owns shared Cargo/lock/toolchain changes and execution admission. Product and
delivery owners implement proposed patches in their claimed contexts. This owner
has only explicitly claimed performance specification/plan/evidence paths.

Detection weights, thresholds, predicates and verdict bands remain frozen. Keep
fail-closed decisions, audit durability, cryptographic/canonical bytes, dual
control and motor/speed-independent accessibility. Preserve the T4 ceiling.

## Measurement vocabulary and identity

An observation records a real terminal command and its declared scope. A
comparison requires matched conditions and retained before/after observations.
A source inspection, plan, successful metadata query or existing artifact is
neither an execution result nor a compiler-cache hit.

Record source SHA plus dirty-state/immutable materialization provenance, complete
source and fixture closure hashes, lockfiles, Rust/Cargo/LLVM and native tool/SDK
versions, binary hashes, OS/architecture/ABI, effective Cargo configuration and
ancestor configuration, environment/flags, package/target/features/profile,
CPU/memory limits, target/build directory identities, timing provider, concurrency,
repetitions, warmup and background workload. Hash sensitive configuration values;
do not publish raw arguments, headers, local paths, credentials or timing HTML.
Missing dimensions remain explicitly unknown; a partial identity cannot authorize
linked-artifact reuse. Record explicit host builds separately from --target builds.

Keep these dimensions separate:

| Dimension | Required distinction |
|---|---|
| Work | Build/check/test discovery/test execution/product runtime |
| Cache | Existing target, observed fresh/rebuilt/unknown events, compiler cache hit/miss, archive transport hit/miss |
| Cold state | New target is not proof of cold registry, OS page cache or compiler cache |
| Venue | Native Windows, WSL, Desktop act, genuine GitHub, Docker product topology |
| Time | Preparation, queue, controller overhead, Cargo child, actual executable/request |
| Memory | Process peak, simultaneous process-tree sample, cgroup peak and host commit |
| Statistics | Individual operations, batch aggregates, independent repetitions and their sample counts |

Cargo timing HTML is a private compilation/critical-path diagnostic. Preserve
Cargo compiler-artifact freshness events and terminal build-finished status.
Do not infer aggregate memory from one child's peak. Never publish p95/p99 from
five aggregate batch times as if they were request-level samples. Preserve first
failures and interrupted, unsupported, ignored, filtered and not-run outcomes.

## Execution and cache rules

Each workload requires root's explicit source, slot, target, command and time
budget. Four jobs1/incremental0 local pipelines are a ceiling. Require host
commit below85% and physical free memory above20GiB before admission; defer new
compilation at commit at least88% or free memory below16GiB. Serialize cold
compilation against heavy native Docker/CEF/SDK work. Keep root's source graph
window and exact target ownership; a persistent .cargo-lock file is not a live
holder proof. Tool version queries are recorded as identity observations only.

No global cache deletion/pruning, WSL shutdown, foreign process termination,
host security/frequency/ASLR changes, new paid infrastructure, publication,
tags or OIDC. Preserve caches and decline unsafe admission. An allocated deadline
must name the existing qualified controller and owned-process cleanup behavior;
an observation timeout is not proof that a child stopped.

The current delivery archive pilot permits checksum-verified public .crate
archives only, with trusted-push saves and bounded quota/retention. Compiled
targets, index/source trees, Git dependencies, secrets and test-result caches
remain outside that grant. Any compiler cache experiment needs a separate owner
review, trust namespace, executable version and allocation. Never skip actual
source-specific checks because a transport cache reports a hit.

## Experiment sequence and ownership

| ID | Experiment and current status | Acceptance boundary |
|---|---|---|
| B1 | Proposed native signals build plus three identical repeats in an existing target; not allocated/executed | Root chooses immutable post-graph SHA, existing target and quiet window; report actual freshness, never assume warm hits |
| R1 | Root accepted review of streaming signed-header whitespace emission; implementation/measurement pending | Preserve Unicode split_whitespace, signed-header order/newlines, cookie/body hash and all token bytes; owner patch plus matched comparison |
| B2 | Archive fetch/preparation measurement and eligible-job restore proposal; research only | Current policy-job cache does not populate other runner filesystems; owner approves any exact policy expansion |
| B3 | Package/version/feature/target duplicate compilation and invalidation; research only | Prove equivalent gates and actual dependency features before eliminating any build |
| B4 | Profile/debug/codegen/linker/incremental/sccache; research only | Separate developer and CI profiles; matched compile and actual executable regressions; no inferred universal speedup |
| R2 | Runtime/Authority/Audit/collector resource and tail-latency pilots; not allocated | Owner-approved real boundary, bounded concurrency/I/O, request-level distributions and security/cleanup checks |

The existing local-edit profile already sets line-tables-only and incremental
compilation. Do not duplicate it or silently override CI incremental0. Compiler
cache and incremental experiments are separate configurations. A faster build
does not establish a faster runtime or vice versa.

R1 currently replaces only split_whitespace().collect::<Vec<_>>().join(" ")
emission with direct appends from the same iterator. Keep the baseline Rust
implementation only in test/benchmark code, never a second production canonical
protocol. Preserve81 independent request canonical/token vectors and27 verifier
precedence vectors. Add exact-byte comparisons for empty/all-whitespace values,
all Unicode whitespace scalars, non-whitespace U+200B/U+FEFF, long tokens,
combining UTF-8, case collisions and raw body bytes. Decode fixtures/build maps
outside timed batches; consume outputs through black_box. Removal of explicit
temporary constructions is source evidence, not an observed allocation count.
Do not introduce unsafe global allocator instrumentation to manufacture counts.

Runtime's current single accept loop owns mutable Sessions and durable audit.
Parallel workers would change ownership/ordering and need an accepted architecture
contract before implementation. A function benchmark cannot qualify throughput,
network cancellation, audit persistence or complete process-tree cleanup.

## Verification and completion

Every accepted patch needs scoped locked tests, fmt and strict Clippy plus actual
owning boundary evidence; collector changes additionally require wasm32 check and
an isolated real browser pilot. Product e2e remains Docker-only and requires its
skill, pinned source/image/topology and actual runner. Windows Desktop Docker
operations follow rule96 engine identity and child helper preflight. Local native
or component successes never substitute for full product or genuine hosted gates.

An improvement is accepted only after matched before/after measurement and
functional/security regression checks both pass, with uncertainty and residuals
retained. No result is accepted merely for being faster. Preserve rollback and
owner review. Whole-goal closure additionally requires coverage of every research
area above, measured justified changes, canonical evidence and exact completion
accounting; R1 or B1 alone cannot close this goal.

## Critique reflected

Reviewed inline using blue, red, SRE, compliance and evaluator perspectives;
no independent agents or unexecuted panel results are claimed.

- P0: cache trust or audit batching could weaken enforcement. Keep the archive
  grant narrow, actual gates mandatory and durable-before-dispatch ordering.
- P1: source relocation, ancestor configuration and feature unification can
  invalidate a warm-cache assumption. Retain full identity and observed freshness.
- P1: fewer temporary allocations do not prove faster requests. Require R1 A/B
  and a separately allocated product-boundary profile before broader claims.
- P1: broad keyword lane globs overlap existing owner globs. Use the exact
  temporary claim's concrete plan/evidence roots; registration is not ownership.
- P1: benchmark overhead, background jobs and aggregate quantiles can bias the
  result. Record sampling and timing scopes and repeat matched measurements.
- P2: shared desktop noise and CPU topology limit generalization. Keep observed
  variance and workload-specific conclusions; do not change host controls.

## Evidence and research provenance

Existing comparison [[evidence:root/20261107092034]] used control d610ab4d and
workload8132117: six matched passing records; repeated targets already exercised.
Initial collector was slower locally, while workspace build/native process were
faster. It is neither fully cold-cache certification nor universal venue parity.
The current four public files were hash-checked against their index unchanged.

Research reviewed2026-11-07; live documentation is identified separately from
tagged upstream source and installed tools:

- [Cargo timings](https://doc.rust-lang.org/cargo/reference/timings.html),
  [profiles](https://doc.rust-lang.org/cargo/reference/profiles.html),
  [build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html),
  [feature resolution](https://doc.rust-lang.org/cargo/reference/resolver.html),
  [build-script change detection](https://doc.rust-lang.org/cargo/reference/build-scripts.html#change-detection).
- [Rust1.99.1 String source](https://raw.githubusercontent.com/rust-lang/rust/1.99.1/library/alloc/src/string.rs)
  and [str::split_whitespace](https://doc.rust-lang.org/std/primitive.str.html#method.split_whitespace).
- [LLVM23.1.1 benchmarking](https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-23.1.1/llvm/docs/Benchmarking.rst)
  and [LLD Windows support](https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-23.1.1/lld/docs/windows_support.rst).
- [GitHub dependency caching](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching),
  [upstream sccache Rust constraints](https://github.com/mozilla/sccache/blob/main/docs/Rust.md),
  [Microsoft WSL filesystem guidance](https://learn.microsoft.com/en-gb/windows/wsl/filesystems).

Actual native identity observed: Cargo1.99.1(5f94df478),
rustc1.99.1(58cf8f9c35b355199727627d07ae34a4e2846d67),
x86_64-pc-windows-msvc, bundled LLVM23.1.1. This does not identify the installed
external linker/SDK or prove a build. No new benchmark has run for this contract.
