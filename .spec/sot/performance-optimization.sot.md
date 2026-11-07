---
id: sot:root/performance-optimization
title: Reproducible build and runtime performance optimization
status: draft
created: 2026-11-07T13:56:43Z
updated: 2026-11-07T15:10:19Z
rel:
  depends_on: [sot:root/rust-migration, sot:root/program-roadmap, sot:root/delivery, sot:root/structural-development]
  governs: [crates/integrity/src/request/canonical.rs, crates/integrity/src/request/canonical/tests.rs, crates/integrity/examples/canonical_performance.rs]
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
initially held only specification/plan/evidence paths; root later delegated the
three exact Integrity source paths governed above, with no wider crate grant.

Detection weights, thresholds, predicates and verdict bands remain frozen. Keep
fail-closed decisions, audit durability, cryptographic/canonical bytes, dual
control and motor/speed-independent accessibility. Preserve the T4 ceiling.

## Bottom-up qualification and experiment admission

Follow structural-development and plan:root/20261107151000. X performance work
measures the layer actually exercised. B1 is native compiler/build observation;
R1 is an L1 canonical-function comparison. Neither qualifies L2 durable I/O,
L3 authenticated services, L4 supported hosts or L5 Docker product behavior.
Performance receipts compose upward only with the owning boundary's actual
functional and lifecycle evidence. Preserve failures and unsupported cases.

The next common preparation implementation is a tooling-owner handoff, not a
new source grant here. Its L0 contract must identify selected inputs, finite
file/byte/root/time budgets, partial-result states, content-hash freshness and
unknown tool/SDK closure. L1 tests must cover exact limits, overflow, changed
content and incomplete capture. L2 qualification requires real files, read
failures, link/reparse policy and observed termination on supported hosts.
Measure preparation separately from compilation and validation. Any faster
preparation result must retain the same declared input coverage; compare
coverage changes explicitly rather than reporting them as implementation speedup.

An Audit L1 candidate is under owner triage: record_hash_bounded currently wraps
its SHA256 sink in an8192-byte BufWriter. NFC emission writes individual UTF-8
characters, so buffering cannot be removed on allocation intuition alone. A
fixed-buffer alternative would trade heap allocation for stack use and code
complexity. Before implementation, the owner must accept exact canonical bytes,
count/replay and byte/item errors, sorting/NFC, explicit final flush and previous
hash text as unchanged contracts. Compare all92 frozen records and boundary
inputs; keep fixture construction outside timing and report corpus byte weights.
No measured allocation reduction or speedup exists. This proposal grants no
Audit source ownership and does not qualify WAL, recovery or service throughput.

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

### Proposed bounded verification preflight

Root requested this proposal after broad registry/toolchain censuses delayed
small unit batches. These limits need root acceptance; they do not supersede
program-roadmap admission, owner allocation or mandatory tests. No reusable
implementation or new execution permission follows from this specification.

The proposed default budget per admitted pure-Rust batch is5000 selected files,
2GiB aggregate file bytes,512MiB maximum individual file size,64 explicitly
selected roots and30seconds of cooperative observation. Check counts/lengths
before hashing each file and time between files. A blocked filesystem/hash call
can exceed that threshold; do not advertise a hard OS return guarantee. Root may
assign a larger explicit finite profile when necessary, or move the batch to a
qualified venue. No unbounded traversal or automatic budget increase.

Select inputs by the exact command profile:

- Frozen owning package sources, fixtures, path dependency packages, required
  root manifests/locks/toolchain and applicable Cargo ancestor/home config.
- Actual resolved external package source directories and corresponding archives
  and relevant registry index/config records. Include required build inputs;
  unrelated cache packages and other projects are outside this closure.
- Selected Cargo/compiler executables, their required runtime libraries and
  compiler target libraries for the declared host/target. Include the actual
  linker/SDK inputs when the profile needs them. Unknown tool/library inputs
  remain explicit; do not assert a complete compiler/SDK identity from rustc.exe
  alone. Unused toolchain docs/examples/other target libraries are excluded.
- Hashed effective flag/environment dimensions and declared feature/profile
  selection. No raw credentials or unrelated user configuration are collected.

A workspace-only metadata inventory with --no-deps is not a resolved dependency
graph: resolve is null and dependency source fetching is excluded. Use actual
artifact/package evidence or qualified scoped resolution for the dependency
closure. Preserve a failed broader metadata attempt rather than rewriting it as
a passing scoped observation. Package identity strings must be interpreted under
their observed Cargo version; reject unsupported forms rather than guessing.

An observation manifest records source/lock/toolchain/host/target/features/profile,
the finite selected path set and its selection evidence, per-file content hashes,
counts and bytes, budget/elapsed scope, missing inputs, completeness and terminal
command identities. Refuse traversal escapes and unresolved reparse/symlink
identities. A selected closure cannot prove unrelated shared cache unchanged.

Root-owned manifest reuse has two separate meanings:

1. Reuse a reviewed path inventory when source/lock, package set, toolchain,
   target/features/profile and config scope match. Rehash selected current bytes
   before and after the admitted batch; path inventory reuse is not content reuse.
2. Within one root-declared frozen batch, reuse its before manifest across the
   scheduled commands, then compare the same complete selected set after all
   commands. Record the original manifest digest, command IDs and root's no-write
   coordination window. This avoids repeated identical per-command censuses.

Across separate batches, do not reuse stale content hashes solely because of
mtime/size, a claim file, a version string or an owner's assertion. Root may
reuse a content-addressed immutable source/image manifest only when actual
materialization identity and the declared immutability boundary are evidenced;
otherwise selected current content hashes are required. Repository coordination
is not OS tamper resistance, and no hostile same-user guarantee is implied.
Any changed source/package/config/toolchain/SDK or unresolved path invalidates
the applicable reuse decision. No manifest cache authorizes skipping actual tests.

On budget exhaustion or missing required evidence, retain the partial census
with explicit completeness=false and its original failure/handle. The owner may
request stopping its own I/O under its existing supervisor; never stop another
owner's process or infer a terminal result from an observation timeout. Root must
choose a corrected finite scope, explicit larger allocation, qualified venue or
honestly limited observation before further work. Required tests/checks and
security gates remain required; partial provenance cannot become artifact reuse
authority or full qualification. No global cache deletion/prune is involved.

The measured R1 precedent is a broad55842file before census followed by an
actual30-package selected closure of1041 files, with before/after0 differences.
This establishes the selected comparison's coverage, not a timed A/B speedup for
the proposed policy. Required parser/selection/budget tests and actual tool pilot
belong to the owning Rust tooling context under a separate implementation grant.

### B3 command-observation coverage proposal

Source-only review at6aa87db1 traced components.rs::invoke through
command::json_log to metrics.rs. The component helper invokes GNU timeout with
the actual Cargo command nested in its fixed argument vector. Metrics recognize
only literal cargo/cargo.exe as the executed program; this wrapper route therefore
receives neither a Cargo observation nor optional --timings insertion. Capture,
header and cbindgen installation use that route. The runtime component's explicit
direct-Cargo helper is a separate observed route. No actual coverage pilot has
run for this finding and no validation outcome is being reclassified.

The proposed delivery-owner change supplies a closed explicit inner-Cargo
descriptor when constructing the existing wrapper invocation. Do not infer an
arbitrary nested command from untrusted strings. Preserve timeout budget, TERM,
kill-after and failure behavior exactly; inject optional timings into the inner
Cargo argument segment before its forwarding boundary. Distinguish wrapper
elapsed time, including startup/wait, from pure Cargo-child time. Prevent double
counting and keep raw arguments, paths and timing HTML private. Observations must
not change required tests, cancellation semantics, success interpretation or
artifact reuse flags. Tool-owner design/implementation and allocated real pilot
remain required; this is not a new path grant or compiler allocation.

Requested-directory identity also needs a separate correction before cache
comparisons. Current metrics hash CARGO_TARGET_DIR or literal target, omitting
the CLI --target-dir request. A real wrapped cbindgen install supplies that flag,
although its metrics are currently absent through the wrapper gap above. Cargo's
configuration reference states CLI override and a separate build.build-dir for
intermediate artifacts. A unique target directory alone is not proof of distinct
intermediate storage when that setting is overridden.

Record requested CLI target directory and inherited target/build-directory
candidates with presence/source labels and hashes. When effective config is not
resolved, keep effective directory identity unknown rather than hashing a default
as an observed location. Preserve both unset and explicitly empty flag states.
Unknown config/wrapper/SDK dimensions must keep reuse_key_complete and
linked_artifact_reuse_qualified false. No compiled-cache enablement follows.

Required regression cases include wrapped/direct Cargo, non-Cargo wrapper,
forwarded -- separator, unchanged wrapper stop flags, CLI versus inherited
directory requests, empty/unset environment, build-directory override, redacted
payloads and single metric accounting. Then an allocated Rust-authored actual
bounded Cargo pilot must prove the observation arrives while original required
commands still execute. No shell-script test implementation is authorized.

| ID | Experiment and current status | Acceptance boundary |
|---|---|---|
| B1 | Four native signals builds completed on immutable9b064dbd; initial15fresh/8rebuilt, repeats23fresh/0rebuilt | Project-quiet/external-load-observed; no controlled A/B/cold-cache claim; [[evidence:root/20261107135643]] |
| R1 | Native c4786b67 tests27/check/strict Clippy/fmt; one release fixed-corpus comparison completed |5hot paired batches observed13.75-18.69% shorter time, paired median15.68%; unchanged source/binary/input closure; no product/tail/allocation or independent-process claim |
| B2 | Archive fetch/preparation measurement and eligible-job restore proposal; research only | Current policy-job cache does not populate other runner filesystems; owner approves any exact policy expansion |
| B3 | Wrapped Cargo observation gap and directory provenance proposal routed to delivery owner; source only | Preserve wrapper stop/failure semantics and privacy; actual pilot pending; prove equivalent gates/features before eliminating builds |
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
Root's temporary narrow handoff permitted this owner to change only canonical.rs,
canonical/tests.rs and examples/canonical_performance.rs. The example uses the
fixed81 independent request fixtures plus6 synthetic cases, prevalidates exact
canonical/token bytes, then warms both implementations16 corpus iterations and
runs5 alternating AB/BA pairs of256 corpus iterations per variant. Preparation
is outside timing. A120second deadline covers warmup and paired work, with a
check per corpus iteration; fixed bounded fixtures constrain each iteration.
Output includes per-batch time, operation/byte counts and fixture/output digests,
never raw header or key data. This source contract is not executed evidence.
After measured receipt fc74b9be and independent raw review5ae869a6, these three
source paths were returned to root. The actual claim now contains only the five
exact performance spec/plan/public evidence/private bundle scopes. Any further
implementation requires a new explicit owner handoff; governing this contract
does not imply current source-write authority.
Independent read-only review of c4786b67 found no semantic blocker in the three
delegated paths. Its acceptance is source-only. The120second check is explicitly
a cooperative timed-work budget after preparation, not a hard whole-process
deadline. Root subsequently clarified that this fixed trusted function example,
with no child spawning, network or external input, may use an ordinary tool-owned
execution session and actual observed terminal status. No new native supervisor
or hard OS wall-clock/cleanup guarantee is required or claimed for this narrow
function measurement. Exact root allocation remains required. Real product/native
process-tree lifecycle gates remain unchanged. Five hot in-process pairs are
not independent process repetitions or cold runs. Keep these distinctions in
any result schema interpretation and conclusion.
Before measurement, a case-sensitive read-only inventory found that all81 oracle
canonical outputs total15174 UTF-8 bytes, with individual lengths181 through192.
One synthetic long-header case alone contains49152 header payload bytes before
framing. The aggregate benchmark therefore weights a synthetic large-input case
heavily by byte work. This is a corpus property, not observed timing dominance.
Preserve the predeclared corpus; do not generalize its aggregate ratio to typical
HTTP requests or alter case weights after seeing timings.

Runtime's current single accept loop owns mutable Sessions and durable audit.
Parallel workers would change ownership/ordering and need an accepted architecture
contract before implementation. A function benchmark cannot qualify throughput,
network cancellation, audit persistence or complete process-tree cleanup.

## Runtime measurement profiles awaiting owner allocation

These are proposed contracts, not executable tests or new deployment claims.
Each owner must select exact supported fixture, topology, capacity, payload bounds,
request count, timeout, cleanup budget and source before root admits execution.

| Context and source boundary | Measurements | Required companion correctness evidence |
|---|---|---|
| Runtime application.rs serialized accept/decide/dispatch | End-to-end request latency including queue time; separate service time, status counts, completed/offered throughput and sampled RSS | Frozen verdicts/canonical bytes, durable audit before origin, slow-client deadline, shutdown drain and no second response |
| Authority transport/mod.rs and transport/service.rs | Capacity occupancy, admission rejection, live connection/worker count, timeout-to-terminal interval and permit recovery | Real authenticated native peer, no permit release while detached work survives, no false graceful acknowledgment |
| Audit record/canonical/encoder.rs and durable sink/storage | Canonical encode/hash work by bytes/items; durable append/commit latency separated from normalization; RSS by bounded record set | NFC/sort/frame/signature parity, count-and-replay limits, partial-write failure, durable persistence/reopen and retained independent pin |
| Collector browser/behavior.rs and session/mod.rs | Capture/reduction/serialization time, bounded buffer occupancy, listener count before/after stop and browser memory observations | Actual isolated browser, bounded samples/key pairing, reentrant stop cleanup, cleared secrets/raw buffers and no motor/speed gate |

Authority's LocalContext owns an Arc permit expressly to retain admission while
detached work survives. Started Tokio1.54.1 spawn_blocking work cannot be stopped
by aborting its JoinHandle; a shutdown timeout only stops waiting. No performance
patch may count a timed-out future as a reclaimed worker. Reserve capacity before
allocating large queued work; a bounded message count alone does not bound total
bytes held by waiting producers. Report held permits separately from queue size.

Audit's encoder intentionally counts then replays each bounded field, including
normalization. Do not remove either pass or reuse unverified lengths to gain
speed. Characterize CPU and bounded-memory cost separately from filesystem
durability; an in-memory sink or mocked fsync is not a durable-append benchmark.
Collector memory is browser/host specific; a native unit test's RSS cannot prove
WASM heap or whole-browser usage. Raw samples remain private and are cleared.

For each future latency profile, report offered, admitted, completed, rejected,
cancelled and timed-out denominators plus unfinished work. Declare whether
arrivals are open-loop or completion-paced: completion pacing can conceal queue
delay under overload. Record latency from the scheduled/offered request boundary
where applicable, without silently excluding dropped/unfinished requests.
Use a declared quantile estimator and request sample count; retain repetitions
separately, and publish tail estimates only with sufficient samples and explicit
uncertainty. A pilot with insufficient samples reports its observations without
asserting a stable p99. Do not increase workload merely to obtain a percentile
outside the allocated budget. Owner-authored reusable harnesses remain Rust-only.

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
- Additional2026-11-08 review: [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html)
  and [Cargo build options](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
  for target/build-directory requests and command observation scope.
- [Rust1.99.1 String source](https://raw.githubusercontent.com/rust-lang/rust/1.99.1/library/alloc/src/string.rs)
  and [str::split_whitespace](https://doc.rust-lang.org/std/primitive.str.html#method.split_whitespace).
- [LLVM23.1.1 benchmarking](https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-23.1.1/llvm/docs/Benchmarking.rst)
  and [LLD Windows support](https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-23.1.1/lld/docs/windows_support.rst).
- [GitHub dependency caching](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching),
  [upstream sccache Rust constraints](https://github.com/mozilla/sccache/blob/main/docs/Rust.md),
  [Microsoft WSL filesystem guidance](https://learn.microsoft.com/en-gb/windows/wsl/filesystems).
- [Tokio1.54.1 blocking-task source](https://raw.githubusercontent.com/tokio-rs/tokio/tokio-1.54.1/tokio/src/task/blocking.rs)
  and [bounded channel source](https://raw.githubusercontent.com/tokio-rs/tokio/tokio-1.54.1/tokio/src/sync/mpsc/bounded.rs).
  Versioned docs.rs retrieval failed; tagged source supplied the contract.
- [Rust1.99.1 black_box](https://doc.rust-lang.org/std/hint/fn.black_box.html):
  benchmark optimization barrier is best effort and provides no cryptographic guarantee.

Actual native identity observed: Cargo1.99.1(5f94df478),
rustc1.99.1(58cf8f9c35b355199727627d07ae34a4e2846d67),
x86_64-pc-windows-msvc, bundled LLVM23.1.1. This does not identify the installed
external linker/SDK. Four allocated library builds have now completed; function
A/B also completed once under the declared fixed-corpus limits. Product-runtime
experiments remain not run. See the exact B1/R1 evidence and raw-row digests.
