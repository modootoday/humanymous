---
id: sot:root/performance-optimization
title: Reproducible build and runtime performance optimization
status: draft
created: 2026-11-07T13:56:43Z
updated: 2026-11-07T15:10:19Z
rel:
  depends_on: [sot:root/rust-migration, sot:root/program-roadmap, sot:root/delivery, sot:root/structural-development]
  governs: [crates/sdk/humanymous-core/src/integrity/request/canonical.rs, crates/sdk/humanymous-core/src/integrity/request/canonical/tests.rs, crates/sdk/humanymous-core/examples/integrity_canonical_performance.rs]
---

# Reproducible build and runtime performance optimization

Document version: 0.1.0. This is a measurement and acceptance contract, not a
claim of measured improvement or permission to execute a workload.

Current governs paths follow root's Core relocation c67d07c0. Historical R1 source
paths, Cargo package names, command argv, hashes and evidence below identify their
original revisions and remain unchanged. This reference correction grants no
production-source ownership and does not qualify the relocated implementation.

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
budget. Six admitted lightweight owners are the current ceiling under structural-
development; heavy-workload admission is separate. Existing grants keep jobs1/
incremental0; jobs2 requires its own experiment allocation. Require host
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
small unit batches. Root subsequently authorized M11 to implement the separate
selected-input primitive in existing owned Rust tooling/dependencies and accepted
the defaults below. This does not supersede program-roadmap execution admission,
owner allocation or mandatory tests. No implementation or execution pass follows
from that authorization.

The accepted default budget per admitted pure-Rust batch is5000 selected files,
2GiB aggregate actual read bytes,512MiB maximum individual file size,64 explicitly
selected roots and30seconds of cooperative observation. Check counts/lengths
before hashing each file and time between files. A blocked filesystem/hash call
can exceed that threshold; do not advertise a hard OS return guarantee. Root may
assign a larger explicit finite profile when necessary, or move the batch to a
qualified venue. No unbounded traversal or automatic budget increase.

The first implementation is trusted-repository observation, not an immutable
security snapshot or execution/cache authorization. Keep content-read completion,
input-selection completion and provider-identity qualification separate. Unknown
identity cannot become qualified completeness. Generic reader/budget/state tests
precede actual-file cases; hostile filesystem race guarantees require a qualified
native provider rather than metadata comparisons. Existing canonical-LF inventory
and Delivery Git/worktree provenance schemas remain unchanged.

Count all actual read bytes, including oversize probes. A limited reader reaching
its cap may present synthetic EOF; it cannot alone establish a full-file digest.
Reserve probe capacity within the aggregate ceiling or report completion unknown
when no probe fits. Preserve partial failures and never publish a truncated hash
as a complete-file commitment. Blocking read calls still have no hard deadline.

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

## Audit preparation input and explicit pilot profile

Audit's154715 raw manifests contain177 source files/7434998bytes,3181 selected
registry files/42368010bytes and73 selected tools/607737208bytes. Their combined
3431files/657540216bytes and maximum single file208167760bytes fit the default
count/byte caps. Actual prior registry-before hashing took26.8293511seconds and
tool-before hashing18.911908seconds:45.7412591seconds combined, excluding source,
config, after hashing and1.3099437seconds materialization. These are recorded
prior phase durations, not selected-input CLI measurements or a speedup trial.
Raw files are in the private20261107154715-audit-key-custody-l1-transition-validation-raw
evidence directory; Audit retains ownership of those artifacts.

Root accepted an explicit120second cooperative observation profile ONLY for the
later equal-coverage CLI pilot over this same finite selection. All file/root/
byte caps remain unchanged and one clock spans the entire observation. Default
profiles remain30seconds. The current CLI must first implement and qualify this
explicit profile selection; no automatic increase or per-root clock reset is
allowed. At an exhausted budget return partial rather than a complete digest.
No pilot workload is authorized until CLI qualification and root's exclusive
execution allocation; the current local CEF window excludes benchmark admission.
Preserve prior timings and unknown remaining costs. Do not infer antivirus or
filesystem causation, comparative speedup or immutable/cache authority.

## Source-only CPU distribution experiment

Root observed unrestricted32-bit process affinity masks and uneven per-logical-
processor utilization on the24-core/32-logical-processor i9-14900K. This is not
evidence of an affinity restriction, P/E-core numbering, scheduler failure or a
throughput opportunity. The reported86% commit snapshot fails fresh admission;
no workload is authorized by this design. Retain scheduler default unless actual
matched observations justify a task-local alternative.

First compare Cargo jobs1 versus2, with test workers fixed at1 and affinity
unchanged. Cargo job admission, libtest workers and compiler/codegen parallelism
are distinct controls; jobs1 is not a one-core guarantee. Freeze the same source,
lock, target/features/profile, codegen units, incremental0, toolchain, command and
input inventory. Root assigns two exclusive existing target directories and a
finite changed-source batch with comparable pre-build artifact state. Do not clean
caches or invent cold-cache equivalence. Initial builds are state observations;
unchanged all-fresh replays cannot measure compilation scaling. A genuine owner
edit with equivalent rebuilt units in both targets is required for that claim.

Predeclare three matched changed-source pairs, alternating arm order, only if
root supplies that finite source sequence. No post-result repetition or selection.
Record each pair's source/command, Cargo JSON fresh/rebuilt unit identities,
compiler timing, total compile wall time, process CPU time when observed, peak
commit/RSS sampling scope, CPU distribution and competing workload observations.
Unequal rebuilt units or missing CPU/memory observations limit the conclusion;
report them rather than merging unmatched rows. Unit/test/lint regressions remain
mandatory and separate from compilation measurement. Capture process start/exit
and actual target return; abort further admission on guard failure without foreign
termination. Jobs2 inside one owner and six concurrent owners are separate limits;
root must budget their aggregate load rather than applying six-times-two blindly.

Optional affinity/CPU-set comparison follows only if this first experiment leaves
a measured placement question. Obtain actual group, logical processor, core/SMT,
cache/NUMA and EfficiencyClass mappings from a qualified native topology provider;
numeric processor indexes and utilization alone do not establish P/E topology.
CPU-set IDs are not affinity-mask bit positions. Microsoft documents CPU sets as
soft affinity and restrictive masks as taking precedence; reserved/parked sets
and actual effective assignments must be recorded. No new native helper/unsafe
boundary is authorized here. Without a qualified provider, defer this arm.

Change only an owned experimental process through an already qualified launcher,
before its measured work; retain and verify original/effective policy and child
inheritance. Never change Codex, another owner's process, global scheduler policy,
priority, power plan or firmware. Keep jobs/profile/test workers identical and
compare default versus one predeclared topology-derived placement. A spreading
visualization alone is not a performance win: require shorter matched completion
time without admission, correctness, memory or cleanup regression. Missing or
failed placement observation remains unknown, never successful CPU distribution.

Official sources reviewed2026-11-08: [Cargo jobs configuration](https://doc.rust-lang.org/cargo/reference/config.html#buildjobs),
[Cargo test execution](https://doc.rust-lang.org/cargo/commands/cargo-test.html),
[rustc codegen units](https://doc.rust-lang.org/rustc/codegen-options/index.html#codegen-units),
[Windows CPU sets](https://learn.microsoft.com/en-us/windows/win32/procthread/cpu-sets),
[CPU topology fields](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-system_cpu_set_information)
and [process affinity](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setprocessaffinitymask).
These document mechanisms, not a measured speedup or the current host topology.

## Priority: Meari shell responsiveness and true GPU presentation

Root assigned this core UI defect ahead of cosmetic work. The human report of
150-200ms or more during rapid resize is an estimate, not an instrumented sample.
Browser identifies manual candidate ce24fe52, dev/unoptimized+debuginfo, Meari
1482b746ebc0d867f25d17d01c48c12df888ca6c. Its manual receipt records25 resize
readbacks at96DPI, one tab and exit0; it does not establish latency, full content
visibility, automated input coverage or descendant cleanup. The CEF area was
reportedly responsive; that does not isolate the cause of shell latency.

### Inspected source and measurement boundaries

Read the actual pinned Cargo checkout, not an assumed current upstream API:

1. Browser desktop host.rs handles resize/DPI by synchronizing owned CEF child
   geometry before driver dispatch, then dispatches controls. tab_sessions.rs
   reprojects tabs when views or viewport width change. This retained-source path
   is a separate unmeasured cost candidate, including the64 lifetime tab slots.
2. Meari window/handler/event_loop.rs forwards resize/DPI to meari-shell, refreshes
   the host environment, then updates it. RedrawRequested enters checked_render.
   Native meari-shell/window.rs changes viewport and requests redraw on resize;
   its WASM debounce is not the native Windows implementation.
3. verification/checked_window.rs resolves assets/style/layout, checks context and
   viewport generations, configures changed nonzero target size, prepares the
   retained scene, then submits it while the source/context owner is exclusive.
4. driver.rs, window/mod.rs, preparation and startup owner/context, and checked
   window signatures fix VelloCpuWindowRenderer. anyrender_vello_cpu0.18.0 selects
   SoftbufferWindowRenderer with the enabled feature. Its0.9.0 adapter acquires a
   CPU surface buffer, rasterizes the target, swizzles RGBA into RGB words and
   calls softbuffer present. This is CPU rendering even if desktop composition
   later uses a GPU. The separate optional GL path uses synchronous readback;
   enabling gl is not a true GPU window-presentation fix.

No measured stage durations currently establish which step causes the report.
The existing log_frame_times feature supplies some softbuffer stage labels, but
its stderr timings are not a correlated full-path or compositor acknowledgment.

Propose an opt-in fixed-capacity numeric observation sink: at most512 records
and512KiB, with explicit dropped-record counters, no per-event file/stdout I/O.
Use process-relative monotonic timestamps, frame/context/viewport generation,
finite stage/status/backend enums, physical extent, DPI, event/coalesced counts
and queue depth. Exclude URLs, titles, keys, text, document/source/pixel contents,
raw handles, paths and cross-process identifiers. Flush only after a bounded pilot;
instrumentation overflow or missing stages remains explicit, never zero latency.

Measure receive-to-redraw age separately from host/child placement, tab projection,
retained source preparation, resolve/style/layout, scene encoding, target resize,
CPU raster, swizzle, surface acquire, submit and present-call time. Start the event
age clock on actual event receipt; it excludes prior OS queue/input delay unless
separately observed. Correlate every frame to its latest accepted resize generation.
Submission/present return is not GPU completion or visible scanout. GPU timestamp
queries or OS presentation evidence require separately qualified support and
calibration; unknown visibility is not an invented input-to-photon measurement.

### Minimal reviewable upstream delta

The first actionable predecessor is Browser's viewport-width/full-source tab
reprojection, not a backend toggle. Add the bounded phase observations there and
compare the optimized CPU control before interpreting GPU benefit. Separate tab
model/action-metadata dirtiness from viewport layout. Do not simply remove the
width guard: tab widths, close visibility, compact class and hit/AX bounds depend
on viewport width. Any narrow update must preserve retained source ownership,
generation checks, action paths, selected state, labels and disabled migration.
Use existing1/multiple/overflow-tab and narrow-width oracles, plus actual DPI,
tab clicking and accessibility boundaries in the later allocated Windows pilot.
This predecessor does not replace the required true GPU investigation.

Keep the existing native CEF child HWND and the Meari-owned parent surface. Do not
texture-copy the CEF page, reparent it into a new renderer, enable offscreen CEF,
weaken the sandbox or add a CPU readback/upload loop. A minimal candidate keeps
AnyRender0.14 scene encoding and adds an optional true-window GPU adapter upstream:
Vello compute rendering into a GPU texture followed by GPU blit/presentation to
the parent's wgpu surface. The source path must contain no normal-frame CPU pixel
mapping/readback/swizzle. Software adapters must be reported and excluded from a
hardware GPU claim. Capability/init failure is explicit; no silent fallback.

Prefer an upstream compile-time renderer alias shared by Application, driver,
preparation/startup and checked_window over changing only the constructor. A new
window-gpu feature can retain window compatibility and choose that alias, leaving
CPU as the explicit control. Existing CPU dependencies may remain linked but must
not be mistaken for the selected active backend. Generic checked-scene submission
already accepts impl WindowRenderer; preserve its source/context/viewport checks.
The new upstream commit must cover all typed ownership paths and diagnostics,
not mutate Cargo's cached checkout. Root alone selects the new immutable Meari rev
for both meari and meari-native, changes desktop features and resolves Cargo.lock.
Browser owns integration, CEF geometry/input and actual GUI qualification.

AnyRender's published compatibility matrix and observed workspace suggest the
candidate release line anyrender0.14/anyrender_vello0.15/wgpu30/Vello0.11. This is
a proposal, not a resolved dependency tuple: live crate/source pages returned
inconsistent0.13/0.14/0.15 versions. Inspect exact release manifests/MSRV/checksums,
freeze the selected upstream source and qualify Rust1.99.1 before a pin delta.
Do not copy a latest-version API into the current lock or add an unrelated winit.

The observed upstream main GPU render method performs an indefinite device poll
after presentation. That is a concrete responsiveness risk, not evidence about
the current CPU build. A usable candidate must replace that per-frame UI wait
with reviewed bounded in-flight ownership/completion, retaining GPU resources
until completion. Merely deleting the wait is insufficient lifetime proof. Start
with one accepted frame in flight, one latest pending resize and one pending redraw;
skip superseded work before encoding. A renderer's void render call cannot prove
success: expose finite acquire/submit/present-requested/skipped/error observations
bound to the same generation, without relabeling submission as actual display.

### Scheduling, native invariants and acceptance pilot

Coalesce repeated resize requests to the latest physical extent/DPI before the
next frame and configure only a changed, nonzero target. Preserve close, input,
focus, IME, accessibility and lifecycle ordering; do not debounce away user input
or delay native child hit-test geometry behind an obsolete paint. Do not render
all intermediate sizes or queue unbounded scene/frame copies. The latest scene,
child rectangle, accessibility bounds and submitted generation must agree.

On minimize/occlusion/zero size suspend acquisition without configuring a zero
surface; on restore rebuild the target and invalidate stale frame tickets. On
DPI/multi-monitor changes synchronize physical dimensions, logical layout, CEF
child placement and AX bounds exactly once per accepted generation. Preserve
parent/child clipping/z-order, keyboard focus, pointer coordinates, selection,
IME candidate placement and screen-reader tree semantics. Surface lost/outdated,
device loss, unsupported adapters and shutdown need explicit finite state/error
paths; drop the surface before its HWND owner and retain pending GPU resources
through observed completion. No hard cleanup guarantee from an event-loop timeout.

Before execution root freezes source/binary/profile/backend/driver/display/extent/
DPI/tab-state and one finite owned resize/input sequence. Compare current CPU dev,
optimized CPU with the same behavior/settings, then true GPU at the same optimized
profile. Optimized CPU is a control, not permission to abandon the true GPU path.
Keep backend and optimization effects separate. Predeclare a bounded120second
pilot per arm, one warm-up then three alternating CPU/GPU pairs if allocated;
no opportunistic repetitions or post-result input selection. Separate cold device/
shader initialization from steady resize and report all failure/dropped events.
No new GUI/GPU workload, install, network navigation or heavy slot is granted here.

Acceptance requires exact stage counts and per-frame distributions, queue bounds,
latest-generation final geometry, no retained obsolete resize backlog, and no
regression in CEF child behavior, DPI/minimize/restore, input/IME/AX, sandbox or
shutdown. Propose a predeclared p95 receive-to-submit target of two measured
display-refresh periods, with p99/max and unfinished/skipped work reported; this
is a proposed engineering target requiring owner acceptance, not a pass result or
a human accessibility gate. Visible latency remains unknown without independent
presentation evidence. A GPU checkbox, smoother human impression or faster raster
alone cannot qualify end-to-end responsiveness or close this performance goal.

Primary research: [AnyRender compatibility](https://github.com/DioxusLabs/anyrender),
[observed GPU adapter](https://raw.githubusercontent.com/DioxusLabs/anyrender/main/crates/anyrender_vello/src/window_renderer.rs),
[Vello rendering model](https://docs.rs/vello/latest/vello/),
[wgpu surface lifetime/configuration](https://docs.rs/wgpu/latest/wgpu/struct.Surface.html).
Mutable upstream observations require exact source freezing before implementation.

## Project-wide GPU eligibility and crossover research

This is a performance proposal for root's structural policy, not a new backend,
dependency, model activation, shader or execution grant. Preserve Rust1.99.1,
detection freeze and all existing CPU experiments unchanged. Installing a driver
does not accelerate ordinary rustc, linking, Jekyll, hashing preparation or I/O.
Experimental compiler offload is a separate unsupported qualification track.

Initial hardware evidence supplied by root identifies RTX4070Ti and Windows
DriverVersion32.0.16.1088 only. It proves neither a usable hardware adapter/context,
API feature support nor an allocation budget. Do not infer usable memory from
Win32 AdapterRAM or marketing capacity. No GPU context, benchmark or install was
performed by this performance owner.

The later M11 runtime research checkpoint
e4b822fd479b857e994cb0b2b77e2708a56b1a06 was read from its exact Git blobs:
local-rust-gpu-compatibility.md and local-rust-paired-results.md. Its public
[[evidence:root/20261107073348]] GPU summary has SHA256
33d3e6bd45db05b93aad5902fcd158aae84f445736963b92ad89bc0cfbdfc636.
Private184338 NVML records report CC8.9/NVML-KMD610.88 and a12282MiB total,
10836MiB free,1159MiB used snapshot. CUDA UMD13.3 is driver capability, not an
installed SDK, context/model execution, allocation reservation or process budget.
These observations were read, not rerun, by this owner.

M11 resolved mistral.rs0.9.5 to upstream commit
465191d0b0a7e2432d327b0de06204ab3f94d3a2. Its official Windows CPU asset is the
actual studied executable; Windows CUDA is a source-build candidate. A Linux
CUDA13.0/sm89 artifact and macOS ARM64 Metal artifact are different OS/runtime
tuples, not alternatives executable under the existing Windows CPU grant. The
tagged runtime establishes no Vulkan backend. New supported venue/build/context
and finite host/device admission are prerequisites; WSL remains unallocated.

The unchanged CPU study supplies semantic/task evidence, not a clean GPU timing
baseline:64 attempts produced60 final answers and4 incomplete outputs across
two32-attempt studies, with132 local calls/144084 observed tokens. Both studies
completed30/32, with different incomplete cases. The local behavior judge missed
clear violations; no aggregate quality improvement is established. Matched task
inputs or byte-identical baseline answers do not prove statistical independence
or cross-backend determinism. Preserve exposure modes, failures, model/tokenizer/
template and the fixed output/context limits in any later fresh CPU/GPU study.
All original model processes/listener/target were reported settled by M11; that
does not reserve GPU resources. CPU correctness/exposure, inference throughput,
complete task latency and answer quality remain separate acceptance dimensions.

### Ranked candidates and CPU controls

| Priority | Workload and interface boundary | Eligibility, control and missing evidence |
|---|---|---|
| P0 | Meari parent-window presentation | Direct GPU target is relevant; first qualify UI38 retained-source resize/layout and optimized CPU control. Preserve native CEF child, AX/input and bounded frame ownership. Existing1482CPU path and proposed GPU wait issue are detailed above; no measured crossover yet. |
| P1 | Offline LLM prefill/decode in the isolated maintenance evaluation | Large tensor work is a candidate after the existing CPU study finishes unchanged. Keep exact weights/tokenizer/template/dtype/context/batch and local-only semantics; compare TTFT, steady decode and complete task quality separately. CPU binary is not GPU-capable merely because the host has NVIDIA hardware. |
| P2 | Offline batched model standardization/training/evaluation | First define supported tensor shapes and non-serving acceptance. Keep optimized scalar/CPU SIMD controls, finite input/cohort caps and RNG/numeric contracts. Transfer and queue costs can dominate small batches; no batch threshold is measured. |
| P3 | Product MLP single-request inference | Current source uses27 f32 features, ordered f32 accumulation and f64 sigmoid, and defaults to abstention without a model. A GPU change needs exact semantic/threshold parity and actual deadline benefit, not tensor-library availability. CPU remains the control; no serving/model/scoring change is authorized. |
| P3 | Image/watermark carrier batches or large offline transforms | Only a demonstrated dense, bounded owner-defined kernel is eligible. Preserve exact carrier/codec bytes and standards oracles; no generic GPU claim for PNG/WASM/HTTP. Owner contract and hotspot evidence are missing. |
| Exclude now | Integrity, Audit canonical bytes/MAC/WAL, Gate decisions, IPC, collector events, filesystem preparation | Small/control/ordered work, secrets, durability and canonical parity favor existing CPU/OS paths until a measured hotspot and explicit contract justify change. A faster kernel cannot replace fsync, peer identity, cancellation or enforcement. |
| Separate research | Compiler/kernel offload | Rust-GPU and documented NVPTX workflows require separate compiler/tooling qualification; no nightly, generated foreign kernels, new shader authoring or unsafe exception is permitted by this study. |

CPU SIMD is not automatic permission to use target-cpu=native in distributable
binaries. Distinguish safe runtime-detected ISA dispatch and portable fallback
from an ISA-specific artifact. Preserve arithmetic/byte contracts and run the same
regressions; optimized builds, vectorization and multithreading are independent
experimental factors. Do not use an unoptimized CPU baseline to justify GPU default.

### Candidate compatibility matrix, not product support

Each row requires exact OS/architecture, crate/features/driver/runtime/device and
actual workload qualification. Backend compilation is not runtime availability.

| Platform / vendor family | Candidate route | Required qualification / exclusions |
|---|---|---|
| Windows x86_64, NVIDIA/AMD/Intel | wgpu DX12 or Vulkan for graphics/compute | Query real adapter class, device/surface capabilities, dtype/storage limits and bounded memory; no vendor-name whitelist as proof. Current RTX inventory has no runtime proof. |
| Windows x86_64, NVIDIA tensor inference | Reviewed mistral.rs/Candle CUDA build | CUDA-feature executable and its precise runtime/driver/SM/kernel tuple required. Existing official CPU executable and --cpu study stay unchanged; no AMD/Intel CUDA assumption. |
| Windows arm64 | Candidate wgpu Windows backend | Feature/driver/toolchain and actual native binary/provider proof missing; do not inherit x86_64 qualification. |
| Native Linux x86_64/aarch64, NVIDIA/AMD/Intel | Vulkan wgpu; vendor-qualified tensor backend separately | Device/render-node permissions, real driver and exact kernel/dtype required. CUDA is NVIDIA-specific; other runtimes are independent candidates, not interchangeable flags. |
| macOS arm64 Apple Silicon | Metal wgpu; separately qualified Candle/mistral Metal | Real adapter and compute support, shared-memory residency and exact feature/model tuple required. Shared memory is not unlimited or automatically zero-copy. |
| macOS x86_64 | Candidate Metal on supported hardware/OS | Device and dtype limits must be measured; Apple Silicon qualification does not transfer. No automatic CUDA or portability-layer assumption. |
| Android/iOS | Upstream Vulkan/Metal possibilities | Product host, toolchain, lifecycle/thermal and device matrix are unqualified; not a distribution promise. |
| Browser WASM | WebGPU if browser/device capabilities permit | Separate permission/secure-context/browser/backend qualification. WebGL/GLES availability does not establish Vello compute or tensor compatibility. No collector GPU requirement or accessibility gate. |
| Headless, CI, Docker, VM, remote desktop or WSL | Explicit supported compute device or CPU | Surface availability, software adapters and passthrough differ. WARP/lavapipe/SwiftShader/NOOP are not hardware speed evidence. WSL remains stopped/not allocated; container GPU product qualification needs separately admitted passthrough/topology. |

Tagged wgpu30.0.1 enables DX12/Metal/Vulkan/GLES/WebGPU through explicit features.
Its package declares Rust1.87 while the workspace declares1.93; neither proves
the selected dependency closure under1.99.1. Its NOOP backend performs no compute.
Adapter power preference is a request, not measured residency or performance;
request and observed device class/backend must be distinct. Cpu is software;
VirtualGpu/Other remain unknown hardware backing without explicit provenance.
Require-hardware must refuse unsupported, software or unknown qualification.

Tensor candidates must not silently unify incompatible release lines. Inspected
Candle0.9.1 exposes CPU/CUDA/Metal; mistral.rs0.9.5 instead pins Candle0.11 at
66a8cf184a5a519671454066b1b9efd446ec9f5c and declares Rust1.94. Its CUDA/Metal and
CPU acceleration features are distinct build choices. Inspected Burn0.18 uses
wgpu25 and optional fusion/autotuning, not the proposed Meari wgpu30 context.
These are comparison candidates, not selected dependencies. Latest hardware docs
cannot prove a historical binary's features or minimum supported GPU. Record
unretrieved tagged files rather than substituting a mutable current API.

### Proposed interface and measured selection rule

Keep domain algorithms/records separate from device discovery, backend resource
ownership and scheduling. An owner-reviewed adapter proposal would accept a finite
workload descriptor (shape/count/dtype/input bytes/output budget/deadline/parity
class), a requested CpuOnly/Auto/RequireHardware policy and an opaque resource
lease. Return bounded result/status, requested versus actual backend/device class,
effective limits, failure/fallback reason and source/model identity. Do not expose
raw GPU handles, driver context, shaders or device-specific buffers across domains.
Surface rendering and tensor execution have separate adapters and lifecycles.

CpuOnly must avoid GPU initialization. Auto may prefer qualified hardware only
for a workload/device/profile with measured end-to-end benefit and budget/parity
headroom; otherwise use CPU with a finite reason. RequireHardware refuses rather
than silently running CPU/software. Do not change an active experiment's backend
mid-pair. After device loss, fallback is a fresh explicit state transition with
owner validation; never replay external effects, use a stale frame/prediction or
turn missing GPU evidence into an accepted security decision. Root owns final
cross-cutting policy names/defaults and activation.

Compare total completed work, not kernel duration alone:

T_gpu = amortized initialization + packing + upload + queue wait + kernel +
download + synchronization + validation; T_cpu includes the same useful work and
its CPU scheduling. On UMA the transfer term may change, but mapping/copies/cache
coherency and shared bandwidth remain measurable costs. Benchmark cold start and
steady residency separately. Rendering should stay on GPU through presentation;
tensor results requiring CPU enforcement include their actual readback cost.

A crossover threshold is keyed by exact device/backend/driver/model/shape/dtype/
residency/concurrency/profile, with uncertainty and expiry on changed inputs. No
universal batch size or GPU-first default is established. A future finite sweep
may propose batches1/8/64/512/4096 only within the owning input limits, matched
fixed data and randomized/predeclared arm order. Report latency distributions,
throughput, bytes transferred, queue occupancy and memory peaks; include waiting
to form batches so throughput cannot hide a request-deadline regression. Serving
admission may choose CPU for an urgent request even above an offline throughput
crossover. Actual energy/thermal observations are optional evidence, never inferred.

### Memory, parity and bounded lifecycle

Budget weights, compiled pipelines, activations, KV/cache state, staging buffers,
surface textures, copies and in-flight work separately. Query current qualified
local/nonlocal process budgets where supported; unavailable telemetry remains
unknown. UMA uses host memory pressure too: avoid treating CPU and GPU budgets
as independent extra capacity or double-counting the same physical allocation.
No guessed VRAM budget, automatic spill, unified-memory migration or unbounded
device-layer remapping. Keep host guards and explicit per-task memory/queue caps.
Meari/CEF/model allocations may compete on the same physical device.

Numeric acceptance must precede acceleration. Parallel reduction order, fused
operations, dtype conversion, quantization and exp/sqrt behavior can change NaN,
Inf, rounding and threshold-adjacent results. Keep frozen bytes exact for crypto,
canonical/RNG/codecs; define authorized tolerances only for a non-serving numerical
contract, including all failures/abstentions and unchanged decision boundaries.
GPU ML serving requires separate proof/approval even if offline error is small.
LLM equal weights do not imply identical tokens across backends; hold tokenizer,
chat template, sampling/greedy settings and denominator fixed and evaluate quality
separately from speed. Do not relabel unreliable local judging as a benefit result.

Use a finite admitted queue and explicit buffer/resource owner through actual GPU
completion. Cancellation before submission can discard pending work; a cancelled
future or timeout after submission does not prove device work stopped. Retain
permits/buffers until observed completion or a qualified device-loss/teardown path;
do not reuse allocations while work may survive. Bound frame/batch counts, startup,
polling/retry and shutdown policy; no UI-thread indefinite wait. Device lost/OOM,
unsupported kernels, stale generations and missing output are explicit terminal
or retained-uncertainty states, with no automatic unsafe result promotion.
Clearing host buffers is not proof that driver/device copies of secrets are erased.

### Qualification roadmap and policy critique criteria

Browser's UI39 research is frozen at
aa22cd0abcdaabb35eb27d06b356952a81cf23cf, including GPU-BOUNDARY.md and
evidence:root/20261107073932. Its corrections match this synthesis: package versus
workspace MSRV, UI38 viewport-source-cache elision versus native event coalescing,
independent CEF GPU pressure, trusted-host-only selection and UMA alias accounting.
This is a reviewed research checkpoint, not an executed GPU backend or latency fix.

The initial one-in-flight control above and Browser's proposed maximum two frames
are experimental alternatives, not two competing shipped limits. Root must select
the actual finite owner budget for an admitted backend/profile; measure queue age,
memory and responsiveness with that limit. Surface latency hints never establish
an OS display-queue bound. Preserve unknown compositor/display depth and distinguish
owner completion from actual display. The exact Meari GPU API/pin, compatibility
tuple, bounded lifetime implementation, hardware pilots and product gates remain
unqualified. No research consolidation grants execution or changes CPU studies.

1. Finish unchanged CPU studies and UI38 pure/retained-document checks; preserve
   all first failures. Root structural policy defines selection/fallback names.
2. Qualify exact discovery and backend lifetimes on a separately allocated native
   venue. Pure policy tests cover missing/software/unknown adapters, limits, overflow,
   stale input, device loss and CpuOnly/require refusal; mocks stay separate.
3. Meari optimized CPU and true GPU Windows pilots follow the earlier fixed-input
   matrix, including native CEF child/DPI/minimize/IME/AX/input and actual cleanup.
4. After CPU LLM evaluation completes, propose an isolated exact-feature GPU tuple
   and then finite cold/warm quality/TTFT/decode/end-to-end comparisons. Batch
   compute experiments follow owner shape/parity contracts; no serving activation.
5. Promote only source-bound real hardware tuples with measured benefit and all
   regression gates. Product integration still requires Docker-only actual e2e,
   with GPU topology if claimed, plus required native host/UI qualification.

Reviewed root's actual structural GPU policy draft v0 after it became available.
Its workload priorities, separate presentation/compute/venue interfaces, CPU
reference, qualified-auto and require refusal, no indefinite UI wait, stable
toolchain and bottom-up real-pilot requirements align with this research. No
implemented policy or GPU qualification follows from that source review.

Concrete clarifications proposed to root, before implementation:

- P1: distinguish request cancellation/terminal reporting from actual submitted
  device-work settlement. A timeout may finish the caller while the device owner
  remains RetainedUncertain with permits/buffers held; a backend without a qualified
  completion/teardown path cannot claim cancellation reclaimed its capacity.
- P1: define hardware classification for Require-GPU explicitly. Cpu/software and
  NOOP refuse; VirtualGpu/Other require verified hardware backing rather than
  vendor string or requested power preference. Unknown capability/memory inputs
  cannot be converted into a qualified device tuple by successful enumeration.
- P1: separate logical memory ledgers from physical admission. Recording CPU RAM,
  VRAM and UMA separately must not add UMA bytes to host free memory or double-count
  shared pages. Use a finite joint host/device budget and fresh actual residency
  observations, with unknown telemetry and concurrent consumers explicit.
- P2: state that crossover eligibility expires on driver/backend/model/shape/dtype/
  residency/concurrency changes. A cached threshold or offline throughput gain
  cannot override a current request deadline, device loss or admission guard.
- P2: attach source/version/feature evidence per exact backend tuple. A compatible
  package MSRV is not a complete transitive/native SDK proof, and the experimental
  offload guide is not a stable-workspace build acceleration implementation.

These are policy-design clarifications, not observed runtime defects or an extra
approval gate. Root retains structural policy ownership. No GPU runtime,
crossover, compatibility expansion or selection-speedup is proven.

Primary source ledger reviewed2026-11-08:

- [wgpu30.0.1 package/features](https://raw.githubusercontent.com/gfx-rs/wgpu/v30.0.1/wgpu/Cargo.toml),
  [workspace](https://raw.githubusercontent.com/gfx-rs/wgpu/v30.0.1/Cargo.toml),
  [adapter requests/types](https://raw.githubusercontent.com/gfx-rs/wgpu/v30.0.1/wgpu-types/src/adapter.rs).
- [Candle0.9.1 devices](https://raw.githubusercontent.com/huggingface/candle/0.9.1/candle-core/src/device.rs);
  the requested exact66a8cf Candle device source was not retrieved.
- [mistral.rs0.9.5 workspace](https://raw.githubusercontent.com/EricLBuehler/mistral.rs/v0.9.5/Cargo.toml)
  and [feature definitions](https://raw.githubusercontent.com/EricLBuehler/mistral.rs/v0.9.5/mistralrs-core/Cargo.toml).
- [Burn0.18 workspace](https://raw.githubusercontent.com/tracel-ai/burn/v0.18.0/Cargo.toml)
  and [wgpu backend](https://raw.githubusercontent.com/tracel-ai/burn/v0.18.0/crates/burn-wgpu/Cargo.toml).
- [DXGI current process memory budget](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_4/nf-dxgi1_4-idxgiadapter3-queryvideomemoryinfo),
  [D3D12 UMA properties](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/ns-d3d12-d3d12_feature_data_architecture1)
  and [NVIDIA transfer guidance](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html#data-transfer-between-host-and-device).
- [Rust-GPU mutable toolchain](https://raw.githubusercontent.com/Rust-GPU/rust-gpu/main/rust-toolchain.toml)
  requests nightly2026-08-15; [documented NVPTX workflow](https://doc.rust-lang.org/rustc/platform-support/nvptx64-nvidia-cuda.html)
  also uses nightly components. Neither is approved for this stable workspace.

## Runtime64 optional normal-dependency proposal

Root requested this source/graph review after consolidating six Runtime contexts.
It authorizes no Cargo/source edit, resolution, compiler or workload. Inspected
Runtime Cargo.toml/lib.rs, CLI manifest, workspace inheritance and actual imports/
parent cfg gates. No package-name/version/pin/new-dependency change is proposed.
Observation HEAD was95e4af0edda50474c1770dd364a86a0020188d4f;
Runtime manifest SHA2569c3353fdf4ead93eb7213a34c4cbf0b679b2b2de714066243b39c38f793fc556,
CLI manifest SHA256d89248233341f580b50f2a8f82b1673548fc439f13f685c2fe7a707f1f90aca8
and Cargo.lock SHA2565d3e1453ab8c3cdba1d22443effc045740e629153784d0c3b4ac62b7231cbf00.
These are source observations, not a frozen execution grant. Root may need a
workspace-edge lock refresh after accepted optional wiring; it must preserve every
existing external version/source/checksum and retain the first locked failure.

Historical CLI8e51e55881c5e3d6389ab2bb853d6236748805ca normal-tree raw output in
private223854 shows Runtime/native-security only, Gateway absent, yet HPACK,
platform/windows-object, Tokio, md-5 and rustls remain normal dependencies.
The check-normal footer is31.03seconds; it was an all-targets check, with dev
dependencies and existing target history. This establishes observed cost/graph,
not an isolated benchmark, cold-cache baseline or predicted savings.

### Context and target dependency matrix

The table describes direct source use, not a union of grep words. In particular,
std::time does not imply the external time crate; test modules are not normal
imports. Existing core dependencies can still pull shared crates transitively.

| Context feature | Direct portable normal dependencies | Additional target-gated normal source use |
|---|---|---|
| gateway | humanymous-core, serde, serde_json, sha2, hex, base64, getrandom, zeroize, socket2, rustls, tokio | Native operations retain their existing platform cfg and context prerequisites; gateway service fields refer to rustls even when no TLS listener is configured. |
| authority | humanymous-core, serde, serde_json, sha2, hex, base64, subtle, ed25519-dalek, getrandom, zeroize, tokio | Unix transport uses rustix/process; Windows transport uses interprocess/tokio plus widestring. Native-security/audit/network stay required. |
| audit | humanymous-core, serde, serde_json, sha2, hex, subtle, ed25519-dalek, unicode-normalization, hmac | Unix/Windows keys/control/log/sink/durable code adds base64, getrandom, zeroize, time, aes-gcm and scrypt; Unix custody/storage uses rustix process/fs. Windows privacy delegates to native-security. |
| capture | rustls, httlib-hpack | network stays required. TLS ServerConfig/ServerConnection and the HPACK Decoder field make both normal, not pilot-only. Time references are std. |
| network | humanymous-core, serde, serde_json, sha2, md-5, hex | No direct Tokio, rustls or HPACK import is needed by this parser/report context. md-5 remains frozen fingerprint compatibility, not authentication. |
| native-security | zeroize | Unix identity uses rustix/process. Windows process/volume uses winsafe; storage uses windows-permissions; protected secrets use keyring-core/windows-native-keyring-store. No direct core/network/Tokio/TLS/HPACK dependency. |

Source anchors: gateway/application.rs socket2 and sha2, service.rs TLS field,
session.rs entropy/base64; authority/credentials.rs and transport/{unix,windows};
audit/record/{canonical,normalization}, checkpoint, log/stamp.rs, keys/custody;
capture/tls/transport.rs and http2 Decoder; network/tls/fingerprint.rs and reports;
native_security/process,storage,secret. Audit verification/scan and its native
backend are test-compiled through their parent verification cfg; do not infer
normal Linux scanner capability from a leaf file. Private/test/Unsupported
boundaries must remain unchanged.

Runtime's current Windows normal humanymous-platform/windows-object edge has no
normal Rust reference in the inspected contexts. The observed platform reference
is Audit storage/wal.rs inside cfg(test), using platform::testing. Propose removal
of that unused normal edge only after root confirms the frozen import inventory;
retain the platform testing dev-dependency and its existing Windows fixture dev
dependencies. Do not enable testing as a normal feature to make checks green.

### Minimum safe wiring for root review

Keep module cfgs, the Gateway default and every context implication unchanged.
Change each context-owned normal dependency to optional, retaining its current
version/workspace inheritance, native target table and dependency feature list.
Activate it through explicit dep: entries below. Keep target-specific dependencies
in their current Unix/Windows/any-native tables; do not gate dependencies through
target cfg(feature), which Cargo does not use for this purpose. Do not narrow
Tokio/rustls/rustix/interprocess subfeatures in this first proposal.

Proposed additions to the existing feature arrays (source-owning direct use is
explicit even if another context also activates the same dependency):

| Existing feature and unchanged implication | Dependency activation entries |
|---|---|
| gateway -> authority,audit,capture,network | dep:humanymous-core,dep:serde,dep:serde_json,dep:sha2,dep:hex,dep:base64,dep:getrandom,dep:zeroize,dep:socket2,dep:rustls,dep:tokio |
| authority -> audit,network,native-security | dep:humanymous-core,dep:serde,dep:serde_json,dep:sha2,dep:hex,dep:base64,dep:subtle,dep:ed25519-dalek,dep:getrandom,dep:zeroize,dep:tokio,dep:rustix,dep:interprocess,dep:widestring |
| audit -> native-security | dep:humanymous-core,dep:serde,dep:serde_json,dep:sha2,dep:hex,dep:subtle,dep:ed25519-dalek,dep:unicode-normalization,dep:hmac,dep:base64,dep:getrandom,dep:zeroize,dep:time,dep:aes-gcm,dep:scrypt,dep:rustix |
| capture -> network | dep:rustls,dep:httlib-hpack |
| network -> none | dep:humanymous-core,dep:serde,dep:serde_json,dep:sha2,dep:md-5,dep:hex |
| native-security -> none | dep:zeroize,dep:rustix,dep:winsafe,dep:windows-permissions,dep:keyring-core,dep:windows-native-keyring-store |

This intentionally does not split audit into new public microfeatures or duplicate
dependencies under aliases to optimize unsupported targets. Target predicates
still restrict activation; it does not promise cross-target compilation. Use
dep: syntax to avoid exposing dependency-named public capabilities accidentally.
Keep hosted-pilot -> capture+dep:rcgen and credential-pilot -> native-security
unchanged. Keep rcgen as a dev dependency for existing Gateway TLS unit tests.
No change to required-features on existing binaries/integration targets, no new
default feature, and no public reexports or readiness/authority change.

The six-context implication DAG has exactly12 distinct closed sets, excluding
pilot modifiers. All remain legal and behaviorally unchanged:
empty; native-security; audit+native-security; network;
network+native-security; audit+network+native-security; capture+network;
capture+network+native-security; audit+capture+network+native-security;
authority+audit+network+native-security;
authority+audit+capture+network+native-security; and Gateway's full six contexts.
This is an analytical closure count, not twelve executed checks. Preserve any
historical12 capability/profile protocol payloads independently; no capability
manifest or unsupported result is removed by dependency narrowing.

Workspace Runtime inheritance already sets default-features=false; CLI requests
only native-security normally. Its runtime-service / runtime-service-pilot still
activate the Gateway/Authority/Audit/credential chain and optional CLI Tokio/
zeroize. Core's own transitive dependencies and CLI's direct crypto/serde edges
must remain visible; this does not claim to remove every shared crypto crate.
--all-targets tests or a multi-package workspace can unify dev/other-consumer
features and still compile more dependencies. Normal, build and dev trees are
different observations, not contradictory results.

### Required graph, correctness and measurement evidence

Before any patch root freezes exact Runtime/CLI/workspace manifests, source and
Cargo.lock. After accepted wiring, compare normal and feature trees on Windows
and a separately admitted Unix venue for each of the12 closed sets plus both
pilots, default Gateway and CLI service profiles. A native-security-only normal
tree should have no Runtime-owned TLS/HPACK/network/Tokio/authority/audit/core
edge, while preserving its true Unix/Windows security dependencies. Feature
unification from another consumer must be named rather than hidden. Compare the
existing Gateway default dependency/feature closure against the prior version;
no scoring/canonical/crypto behavior or12 capability claim may change.

Run scoped fmt, locked/offline checks and strict Clippy in each relevant source-
specific profile, existing regression/privacy-negative contracts and actual native
pilots only when separately allocated. Empty and single-context checks catch
feature leakage; default/all-features alone cannot prove optional correctness.
Dev fixture compilation must retain platform::testing without promoting it into
normal Runtime. Preserve unsupported targets and any first missing dependency.

After root accepts source and allocates targets, compare matched before/after
CLI normal build/check work with exact command, source except the manifest delta,
lock versions, target/features/profile/toolchain and finite selected inputs.
Use root-provided isolated empty targets for a genuinely cold comparison or
explicitly label first existing-target state; never clean another owner's cache.
Warm each arm independently then record three predeclared alternating replay
pairs, separating fresh/no-op from actual rebuild work. Observe Cargo artifact
identities/timings, total prep and compiler time, memory and external load; retain
all failures and target cleanup. The existing31.03s check is context only and
must not be divided by a later warm check to invent an optimization percentage.
No compiler, Cargo tree/resolution, Docker or WSL command was run for this review.

Primary mechanism: [Cargo optional dependencies and feature unification](https://doc.rust-lang.org/cargo/reference/features.html).
Actual source imports and the saved8e51 normal tree determine this proposal;
The original proposal was subsequently implemented by root at5d68cd39; the
source/receipt review and still-unmeasured comparison are below.

### Implemented wiring and matched comparison design

Root's5d68cd39 changes Runtime Cargo only in production source. Its manifest
SHA256 is c2c174b6e4a71b72637ca62958b4378ab68b22ef179886a682406c8bafb9bbd7.
The retained runtime-optional/Cargo.toml.before exactly matches the earlier
9c3353fd manifest hash. Cargo.lock.before and the current main lock both match
5d3e1453ab8c3cdba1d22443effc045740e629153784d0c3b4ac62b7231cbf00.
Optional wiring preserves default/context/pilot keys, versions/subfeatures and
target predicates, removes the unused normal Platform edge and retains testing
only as a dev edge. Current manifest bytes matched the exact implementation commit
when inspected. No source/manifest/lock change was made by this owner.

Root's [[evidence:root/20261107221648]] runtime-dependencies.json records17 isolated
normal consumer profiles, six expected API-absence refusals, unchanged280 source
inputs, Windows all-target/all-feature check/strict Clippy and177 selected pure
passes out of366 inventoried tests. The selected groups are72network+19shipping+
12journal+37Authority+10record+8Merkle+2limits+17HPACK; the full366 were not run.
Raw compiler footers4.91/5.38seconds and selected test summaries were inspected.
These times do not compare against the earlier31.03second CLI all-target check.
Registry/toolchain attestation and Unix/native execution are explicitly absent.

The first isolated native harness refused a seeded lock before compilation;
attempt01 is preserved. The accepted harness seeds the main lock separately per
profile, resolves offline, checks every external version/source/checksum against
that baseline, then runs locked checks. Main lock remains unchanged. Summary
counts alone do not independently establish which normal edges disappeared.
The retained attempt-02-native consumer manifest explicitly selects only native-
security/default-features=false; its saved command checks only that normal binary,
not dev targets. Its metadata resolves15 packages total (consumer, Runtime and
13 external packages), with Runtime feature native-security and exactly five
normal direct edges: zeroize plus Windows keyring-core, windows-native-keyring-
store, windows-permissions and winsafe. No dev edge is present in that Runtime
node. The saved tree has no rustls, Tokio, HPACK, md-5, Core, Platform, testing,
interprocess, widestring, aes-gcm or scrypt. Raw check footer is4.18seconds.
These records were read, not rerun, by this owner. This isolated consumer is not
the whole CLI: do not compare its15 metadata packages or4.18seconds against the
older CLI normal tree or31.03second all-target check. The25 external lock records
in the profile summary are a different denominator from13 resolved external
metadata packages. Actual speed benefit remains unmeasured.

Proposed finite comparison for a later explicit root lease:

1. Root materializes two sibling immutable source roots from the same5d68cd39
   code/fixtures, replacing ONLY arm A's Runtime manifest with the retained before
   file. Arm B retains the exact implemented manifest. Bind full input manifests,
   main-lock equality, compiler/linker/config/environment/profile and filesystem
   location; do not use8e51 as a different-source timing arm. Record expected
   root/target path differences and do not silently remap or trim the workspace.
2. Primary workload is CLI normal binary checking only: cargo check --locked
   --offline -p humanymous-cli --bin humanymous --no-default-features --jobs1
   --message-format=json --timings, with incremental0 and identical existing dev
   profile/flags. This excludes CLI/Runtime dev fixtures and service profiles.
   Verify its normal Runtime feature set is native-security only before timing.
   A separate build/link or service-profile trial would need its own scope.
3. Root supplies four exclusively leased empty target directories for two cold-
   artifact pairs, ordered A/B then B/A. There is no cache deletion or independent
   target creation by this owner. Empty Cargo artifact cache is not cold registry,
   filesystem, OS or machine cache; prewarm/attest selected dependency sources
   identically outside timing and retain failures. No registry fetch/network.
4. Reuse the second pair's targets for three unchanged replay pairs, alternating
   A/B, B/A, A/B. At most10 compiler commands total. Capture freshness and actual
   rebuilt units separately: all-fresh replays measure command/no-op overhead,
   not compilation scaling. Two cold pairs support bounded observations, not a
   universal statistical speedup; report both raw pairs and their order.
5. Proposed work budget is600seconds plus60seconds owned settlement, subject to
   root acceptance and its qualified controller. No simultaneous compilation,
   GPU/model/heavy GUI work, jobs change, host tuning, global cache cleanup or
   background benchmark. Recheck memory/disk admission before each command and
   stop further launches at failure/deadline. A timed-out observation does not
   terminate a live Cargo/rustc descendant: retain its handle/lease until root's
   actual owned settlement, rather than declaring cleanup by elapsed time.

Use one finite selected before/after preparation manifest per frozen source batch;
report preparation separately and do not consume its time twice or reuse stale
content hashes. Predeclare exact targets and source roots before any execution.
If both sides cannot run locked against the identical external tuples, preserve
that first refusal and revise the design explicitly rather than silently resolving
new versions or dropping dependencies from the comparison.

Each receipt records command/source/manifest/lock/tool identity, actual start/end/
exit, artifact/package/build-script events, compiler timing, normal-feature tree,
CPU/RSS sampling limits, external load, all failures and target return. Compare
paired total check time and rebuilt-work differences; include every denominator.
No aggregate ratio between an old cold all-target run and a new warm normal run.
Existing root correctness receipts stay source-specific and do not become a full
workspace/native/product qualification for materialized benchmark roots. Preserve
no-new-dependency/version/pin, Platform testing dev-only and all12 context closures.
No benchmark execution or target allocation follows from this design.

## Runtime optionalization allocated CLI check result

Root accepted the preceding design and supplied two materialized archives of
5d68cd39f1d44b30d0df2804675d83cd28025d7e plus four exclusive empty targets.
The actual10-command sequence completed on2026-11-08 KST: cold A1/B1/B2/A2,
then unchanged A2/B2/B2/A2/A2/B2. All commands exited0 with build-finished success.
No extra compilation, resolution, linking trial or workload correction occurred.

Each arm's3128 repository files were hashed before and after execution. The only
cross-arm difference is the approved Runtime manifest9c3353fdf4ead93eb7213a34c4cbf0b679b2b2de714066243b39c38f793fc556
versus c2c174b6e4a71b72637ca62958b4378ab68b22ef179886a682406c8bafb9bbd7;
both postflight comparisons have zero differences. Lock SHA256 remains
5d3e1453ab8c3cdba1d22443effc045740e629153784d0c3b4ac62b7231cbf00.
Locked offline normal-feature trees confirm native-security only for Runtime.
Both roots report identical Rust/Cargo1.99.1 identities; executable hashes and
effective relevant environment are retained. No inherited Cargo config was found
from these sibling roots through drive root or the default Cargo home.
The command uses jobs1, incremental0 and existing dev unoptimized/debuginfo2,
with normal CLI binary only, exactly as declared above.

| Actual sequence | Process wall seconds | Cargo footer seconds | Artifact events fresh / rebuilt | Build-script events |
|---|---:|---:|---:|---:|
| Cold A1 |32.4586032|32.37|0 /117|12|
| Cold B1 |17.7272136|17.65|0 /62|9|
| Cold B2 |17.7003842|17.62|0 /62|9|
| Cold A2 |33.0573195|32.99|0 /117|12|
| Replay A2 |0.3113172|0.23|117 /0|12|
| Replay B2 |0.2610730|0.17|62 /0|9|
| Replay B2 |0.2823841|0.19|62 /0|9|
| Replay A2 |0.2954551|0.22|117 /0|12|
| Replay A2 |0.3030198|0.22|117 /0|12|
| Replay B2 |0.2625772|0.17|62 /0|9|

Observed opposite-order cold-artifact wall reductions are45.39% and46.46%, with
55 fewer compiler-artifact events in B. Events are compilation units, not unique
packages. Six unchanged replays rebuild nothing: their subsecond differences
measure startup/freshness work, not edit/rebuild scaling. Replayed
build-script-executed messages do not establish that a build script ran again.
Two cold pairs do not establish a general speedup distribution, and empty
artifact targets do not establish cold OS, source, registry or filesystem caches.
The older31.03second all-target receipt is excluded from this comparison.

The controller recorded admission before every command, actual executable/argv,
working directory, target, start/PID and terminal exit contemporaneously. Total
batch time was107.7287421seconds within the600+60allocation, with no deadline
or admission refusal. Pre-source hashing took28.7947548seconds and post-source
hashing2.9703531seconds separately; these phases are outside command timings.
Per-command wall includes process launch/wait and coarse observer overhead; Cargo
footer timing is separate. Light log/spec inspection during the run is not a
controlled machine-idle proof. Process samples cover visible cargo/rustc/link
names and do not prove complete owned-child CPU accounting or peak RSS. Full
registry/git source bytes and MSVC linker/SDK bytes were not attested; unchanged
locked versions/checksums and Rust/Cargo binaries are narrower input evidence.

Raw records and copied per-command Cargo HTML timings remain under the owned
private evidence directory runtime-ab-20261108. Public
[[evidence:root/20261107135643]] baseline.json binds the summary and limitations;
raw summary SHA256 is6c034870a88263d0db4c6bf824fbf6f8220175d7ac71e29efe600216a42409c1.
Terminal controller session95887 and batch receipt report no remaining Cargo or
rustc processes. The four exact runtime-ab-{a1,b1,b2,a2}-v0 targets and materialized
roots are returned without deletion or source amendment. This component check
does not qualify native execution, runtime throughput, linked builds, workspace
regression, GPU, or Docker product e2e. No renewed execution lease is implied.

## Final8 performance record comparison candidate

Root requested an authored Rust candidate, not another benchmark. Candidate
performance-record-candidate-v1 resides only in this owner's private evidence
scope. The immutable tools-integration-5f1010d4 source remains read-only. Proposed
application adds performance_comparison.rs and its two fixtures under
crates/apps/humanymous-cli/src/qualification/delivery/, plus seven composition
lines in delivery/mod.rs. That exact preimage SHA256 is
688390f3697767793f8f9a72724d7d10e0573146de21bc701c8772a2af4efcf6;
replacement SHA256 is27189b7163c18fed1f15a098cabf9872b22aca56fef3156693d98f6cb8dc7032.
New module/fixture paths must be absent before root applies them. No manifest,
lock, profile, dependency, production source or isolated-source write occurred.

The proposed performance-compare BEFORE.json AFTER.json command runs before Git
discovery and reads at most65537bytes per file, refusing records above65536bytes.
Embedded raw JSON is limited to16384bytes. This bounds consumed record bytes,
not filesystem-open latency, adversarial filesystem identity or an OS deadline.
Strict typed records refuse missing, duplicate and unknown envelope/terminal
fields, altered raw SHA256, unknown cache/layout/venue/timing/coverage, nonpositive
or over24hour observations, unsuccessful/incomplete/timed-out terminals and
malformed UTC timestamps. Direct command metrics additionally need resolved
configuration, workload/features, lock and compiler identities consistent with
the declared scope. Current request-only command metrics deliberately fail this
condition; the candidate does not change their producer or infer effective config.

Pair comparison requires equal declared layout, venue, cache, timing boundary,
workload, controlled-input, lock, compiler, configuration, host allocation,
feature and input-coverage identities. Different variant/source labels remain
visible; duplicate raw evidence is refused. Local and hosted records are distinct
venues and cannot produce a cross-venue ratio. Likewise historical and final8,
empty-artifact and unchanged-fresh, wrapper and direct timing cannot be silently
pooled. One valid pair emits only observed wall reduction, including a negative
value for regressions, together with explicit scope and false cache-authority,
product-qualification, independent-attestation and general-speedup flags.
Declared hashes are not authenticated source/host/cache proof. Owners must bind
their selected input receipts; this parser cannot turn a declaration into one.

The two fixtures preserve exact raw terminal bytes from the actual5d68cd39 A1/B1
experiment and declare historical layout. Their controlled-input hash selects
the3127 unchanged repository files, excluding the explicitly varied Runtime
manifest; configuration/tool records and allocation identity are separately
bound. This is repository/lock coverage, not complete registry/linker attestation.
No new final8 or hosted measurement is fabricated. Six authored tests cover the
real pair and regression direction; each control mismatch including local/hosted;
bad/missing/duplicate/oversize data and checksum; rehashed failed/unfinished or
malformed terminal evidence; duplicate raw fields; and refusal to upgrade an
unresolved command metric through a sidecar. Test mutations are refusal cases,
not measured performance results.

New module SHA256 is0f7404e2e53c2b1b4b6302d5ea070679f5201ce7cae52c3897d16728bacc8f42.
Private preimage.json is1454bytes, SHA256
7f7ff2cdd0ce1fd6ebc3ac50699b19735f38bf016fb2b5563cbd313b908bd7e3,
and binds all four candidate/fixture files. Module rustfmt/check and independent
fixture raw-hash checks passed. Composition preserves unrelated preimage bytes
apart from LF representation and seven added lines; its existing formatting was
not rewritten. Cargo tests, strict Clippy and actual bounded file/CLI refusal
pilots are not run. Root must apply against the exact preimage and allocate the
scoped final8 delivery tests/pilot before any implementation qualification. No
compiler, runtime, Docker, hosted, browser or SDK allocation is consumed here.

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
