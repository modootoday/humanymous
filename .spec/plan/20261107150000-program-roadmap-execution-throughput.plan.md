---
id: plan:root/20261107150000
title: Evidence-based parallel implementation throughput improvements
status: active
created: 2026-11-07T15:00:00Z
updated: 2026-11-07T15:00:00Z
rel:
  depends_on: [sot:root/program-roadmap, sot:root/performance-optimization, sot:root/rust-migration]
---

# Bottleneck investigation and bounded execution improvements

The user requested investigation of slow implementation across chats. This plan
separates measured command time, observed preparation intervals and inferred
coordination delays. It does not change detection policy, resource admission,
publication authority, required tests or the final integration gate.

## Observed baseline

The app inventory snapshot contains23 Humanymous chats:11 active,11 idle and one
not loaded. These are app activity states, not CPU utilization or completion.
At2026-11-07T14:58:33Z the host reported63.34GiB physical memory,30.78GiB free,
and82percent committed memory. Admission thresholds still apply; these values
alone do not prove the machine or its storage was idle.

The last100 commit subject prefixes were docs55, test16, feat11, fix9, ci5,
build2, perf1 and refactor1. Prefix frequency is not a measure of developer
productivity: some test commits contain source or essential evidence. It does
show that canonical-record maintenance is a substantial part of the current flow.

| Batch | Actual command measurements | Observed surrounding interval and limitation |
| --- | --- | --- |
| Model68898522 | test4.372s, check1.518s, Clippy1.407s, fmt0.151s; total7.448s | Archive timestamp14:42:15 to test start14:47:46 is approximately331s; archive to summary approximately388s. These are observed file/command timestamps, not isolated disk-I/O timings |
| Model broad census |70,103 selected paths,65,935 toolchain documentation paths (94.05percent) | Process creation14:42:42.693Z to owner cancellation around14:47:01 spans approximately258s. Cancelled partial census is not closure proof |
| Model narrowed census |4,023 compared files, zero differences |2,589 frozen-workspace files,1,382 external-registry files,52 toolchain files. Only52 files belonged to the four selected path packages; unrelated docs/evidence/legacy sources were still included |
| Performance R1 |55,842 broad registry records versus1,041 selected records | Selected scope is53.64times smaller by record count (98.14percent fewer); this is not an execution speedup measurement |
| Initial four library builds | Rounded Cargo footers total6.07s | First-start to all-terminal upper bound69.491s includes gaps/observation and cannot be attributed entirely to hashing |
| Passae57c13e |37 Rust tests plus check/Clippy/fmt passed; unchanged Go baseline16 passed separately | Selected718-file comparison was sufficient for its explicitly limited component claim; config/tool/SDK byte closure was not captured |

Model fmt failed LAST, after long preparation and successful test/check/Clippy.
A0.151second formatting failure could have been detected first. Model test end
to check start also has a24.82second gap; its cause was not instrumented.

## Confirmed and inferred causes

1. Overbroad enumeration is confirmed: unused toolchain documentation and the
   entire user registry dominated examples. Full-workspace metadata also blocked
   Audit on unrelated ASN1 cache while its scoped dependency tree resolved offline.
2. Target reservations and compiler slots were held during preparation rather
   than execution. Root moved the release example to an existing idle target to
   avoid a reserved-model-target wait; no concurrent target use was allowed.
3. Excessively granular root reauthorization is an observed coordination problem.
   Routine formatting, archive argument correction, own-source fixes and new
   receipt preparation repeatedly waited for explicit follow-up. Root shares
   responsibility for this process overhead.
4. Native-security APIs are a real dependency seam for CLI and .NET; source-ready
   handoffs repeatedly ended turns while downstream consumers waited. This is
   distinct from resource exhaustion and needs stable typed contracts and an
   assigned next execution action, not more independent chat count.
5. Git writes remain necessarily serial. Actual mutex conflicts occurred, but
   their aggregate time is unmeasured; do not label Git the dominant bottleneck.
6. Hosted product loops spend time building before reporting insufficiently
   separated failures. Browser493 and bb each failed16/18 with different diagnostic
   strength; native archive built successfully before a combined member refusal.
   Earlier safe typed failure projection is needed, without exposing raw output.
7. Continuous source and review-ledger churn reopens Pages/audit work. Intermediate
   proof must stay source-bound, but it need not trigger a full render or a separate
   independent reread of unrelated modules after every small checkpoint.

Storage latency, antivirus effects, model-provider rate limits and context-length
costs are plausible contributors but were not measured here. No causal percentage
or guaranteed total speedup is assigned to them.

## Priority actions and ownership

| Priority | Action | Owner and preserved boundary |
| --- | --- | --- |
| P0 | Run scoped fmt/whitespace and cheap schema/config checks FIRST | Every code owner; preserve failure and then correct normally owned source |
| P0 | Select exact package/target/features/profile inputs; never recurse unrelated sysroot docs or all user cache | Performance defines common bounded Rust preparation; source/fixture/path/build-script/root graph/config inputs stay explicit |
| P0 | Reserve build targets only for actual compiler/executable intervals; return immediately on terminal result | Root allocates stable targets; owners release before readonly serialization/review |
| P0 | Keep routine fixes inside an already authorized lane and attempt class | Record previous failure, freeze corrected bytes and retry with fresh admission; no new human permission for ordinary reversible fixes. Scope/graph/unsafe/policy/privileged changes still require their existing decisions |
| P0 | Replace status chatter with compact ready/started/terminal/failure/contract-change messages | All owners; exact handle, source, target, result and next dependency, without claiming an unexecuted pass |
| P1 | One immutable materialization per compatible candidate batch, shared read-only | Root/CI; no overwriting live snapshots, trimming workspace manifests or importing mismatched compiled evidence |
| P1 | Batch source, tests and associated spec/evidence checkpoints | Owners and audit/Pages; retain first failures/raw receipts, but avoid separate bookkeeping-only transactions for every sentence |
| P1 | Freeze cross-chat APIs early and prepare downstream consumers against that contract | Native-security/Authority/Runtime; missing genuine producer remains Unsupported until actual qualification |
| P1 | Add narrow genuine-hosted generic wargame invocation independently of unrelated browser/archive work | CI/Lab/Wargame;26 indexed statuses and unsupported cases remain explicit, full red remains red |
| P1 | Derive bounded public failure facts before an assertion discards private result context | Lab/CI; one strict shared validator, no raw credentials/body/paths, no scoring or pass-condition weakening |
| P2 | Evaluate incremental local edit profile and targeted compiler-cache experiments | Performance/CI; qualification profiles and trust policy remain separate; cache hits never substitute for actual tests |

Candidate fast-preflight ceiling is5,000 files/2GiB total/512MiB individual file/
64 selected roots/30seconds cooperative elapsed budget. These are proposed
selection budgets, not OS-call deadlines or permission to skip required inputs.
An oversized selection reports incomplete preparation and is narrowed explicitly;
final broad qualification has a separately budgeted profile. Reuse path selection,
not stale content hashes based only on mtimes, versions or an ownership claim.

## Research and implementation constraints

Cargo metadata --no-deps reports workspace members without fetching dependencies,
but resolve is null; it is not a complete dependency-closure proof. filter-platform
narrows resolve while packages can still contain all dependencies. Use selected
Cargo tree/artifact inputs for the actual target and feature scope.

Cargo --timings reports compiler units/concurrency and critical paths. Stable
target directories and profile separation preserve useful warm work. Local edit
incremental compilation and sccache need separate experiments: upstream sccache
does not cache system-linker crates and conflicts with ordinary incremental mode.
Do not blanket-enable it for CEF/native binaries or bypass cache provenance rules.

GitHub matrix max-parallel explicitly limits concurrency even when runners are
available. Measure queue/setup/fetch/build/test separately before adjusting it;
heavy CEF/Docker jobs and cheap package units need different profiles. Windows
native files stay on the Windows filesystem and WSL native source/target on WSL
storage; Desktop Docker engine affinity remains unchanged.

Primary references reviewed2026-11-07:

- https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
- https://doc.rust-lang.org/cargo/reference/timings.html
- https://doc.rust-lang.org/cargo/reference/build-cache.html
- https://doc.rust-lang.org/cargo/reference/profiles.html
- https://github.com/mozilla/sccache/blob/main/docs/Rust.md
- https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/run-job-variations
- https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching
- https://learn.microsoft.com/en-us/windows/wsl/filesystems

## Success measurements

Instrument materialize, resolve, enumerate, hash-before, queue, each command,
hash-after and record with actual timestamps. Compare preparation-to-command
ratio, time to first compiler/test result, target idle-reservation time, repeated
full scans, contract wait age and completed functional slices. Proposed initial
goal: ordinary warm-unit preparation under30seconds and at least80percent less
than the observed331second Model example. This is a target, not an achieved
speedup. Mandatory units, strict lint, actual pilots, Docker e2e, source binding,
fail-closed behavior and final main integration requirements remain unchanged.
