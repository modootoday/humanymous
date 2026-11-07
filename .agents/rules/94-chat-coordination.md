# Chat names, goals and execution ownership

Project chat titles use exactly `Humanymous · {L0|L1|L2|L3|L4|L5|X} · {English responsibility}`.
The primary role and bottom-up gates follow `.spec/sot/structural-development.sot.md`;
X is cross-cutting tooling, review, measurement or integration, not a runtime layer.
Keep titles concise and preserve stable IDs. The exact23 titles and concrete roles
are projected into `.agents/sessions/THREADS.md`. Titles do not grant source paths,
cancel in-flight work or change the current claim/protocol/resource permissions.
Only rename chats in this project; preserve unrelated projects and historical runs.

Within each cluster, progress from accepted contract to pure units, actual provider
boundary, service/host integration and product qualification. Upper owners prepare
consumer contracts while lower proofs mature; fake/mock capability is never promoted
to real readiness. Fmt first, prepare only bounded selected targets and return them
at terminal. Batch nonblocking evidence updates; report actual contract/source-ready
changes, starts/terminal failures or exact blockers instead of repeated status checks.

When explicitly asked to set a goal, first inspect the actual goal state. Preserve
an unfinished goal and record accepted scope changes in its plan. Create a goal
only when none is unfinished. Do not invent completion, a token budget, or a
resume operation absent from the API. Report blocked/paused state accurately.
An active chat turn is not proof that its persistent goal is active.

Each owner implements its accepted slice through source, meaningful unit tests,
actual boundary tests in their allocated venue, exact result accounting, Specfold records and an
attributed owned-path commit. A research memo/source-only checkpoint is not full
completion. Keep implementing independent approved work while coordinating seams.
Do not broaden write access, bypass another owner, or create unlimited new chats.

Use the context lanes in `.agents/sessions/LANES.md`. One writer per actual path;
distinct lane names never authorize overlapping writes. When migrating a legacy
claim, release only your own claim, check availability, then claim the new lane.
No force release, alternative alias or lane rename may hide an occupied path.
`git-ops` remains the single global index/HEAD/remote mutex.

Check every native command's exit code immediately. A failed Git claim ends the
transaction before staging, committing or releasing. After acquisition, verify
the stored provider and session against this transaction; preflight/free checks
are not ownership. `$ErrorActionPreference` alone does not propagate every native
failure. Never release a claim belonging to another provider or session.

Report a commit only after the actual commit command succeeds and, while the
transaction is still owned, verify the literal HEAD SHA and intended changed-path
inclusion in that commit. Prepared, staged, preflight-blocked and mutex-blocked
work is not committed. Use those pending states explicitly; never invent a SHA
or announce success before the command returns. Distinguish source, evidence and
policy checkpoints from actual test results and publication.

For a directly invoked PowerShell script (`& script.ps1`), `$LASTEXITCODE` may
remain from an earlier native process. Handle script throws and capture `$?`
immediately; do not use stale native status as script-success proof. For a native
wrapper such as `pwsh -File`, check its actual process exit code. Neither status
substitutes for verifying the stored claim provider/session before Git mutation.

An actual runtime pass requires its terminal process handle/exit, exact test
footer and source closure, not an active process or partial log. Report closure
file count separately from comparison output: Compare-Object side indicators are
difference records, not compared-file counts. State the compared paths, hash
scheme and observed differences; never infer an unchanged closure from a missing
or incomplete snapshot. Preserve first failures and distinguish local/hosted runs.

Local work includes coding, supported Windows/WSL compilation, pure units, fmt,
scoped Clippy and allocated ordinary process/IPC/fixture tests. Root may assign
at most four local pipelines A/B/C/D, each with an explicitly assigned distinct
target, build jobs1 and incremental0. An idle slot is not authorization to start.
Admit new work only at commit<85% and free physical memory>20GiB. Do not overlap
targets or run CEF/native-heavy/bindgen together. Native Docker, CEF and SDK heavy
builds are exclusive against other cold compilation streams, including initial
Windows/WSL targets. Four slots are a ceiling, not evidence that four concurrent
cold streams are safe. Warm unit or bounded actual test processes may overlap
only with explicit budget admission. Before starting a deferred heavy build,
confirm all other cold streams are terminal and recheck commit<85%/free>20GiB.
At commit>=88% or free physical
memory<16GiB, automatically defer new compilation and reduce the next allocation
count; never kill another owner's processes. GPU inference needs a separate
explicit bounded allocation and owned-process stop policy.

Root's later guard observation reached88.2594% host commit while initial WSL
runtime/Python compilation overlapped an Apache Docker build. The exact owned
Apache build was stopped and cleaned up; its45 cases remained NOT_RUN. This is
the basis for cold-stream exclusion, not a passing Apache result. Preserve caches;
do not shut down WSL, drop global caches, prune Docker state or kill foreign
processes to force admission. Such actions require their own authorization and
never follow from a slot assignment.
The single execution-policy canon remains program-roadmap/delivery SoTs, including
resource measurements and the root lock-integration window for manifest changes.
Selected Linux pre-push checks may use Windows Docker Desktop `act` with the same
reviewed workflow YAML, source and pinned image; rule96 governs engine affinity.
GitHub remains required for unavailable local OS/architecture profiles, SCM/account
profiles, event/permission semantics, genuine hosted contracts and the final matrix.
Do not substitute mocks or silently skip required work. Label actual venue and
bind source/toolchain/commands/results; local runs never become hosted evidence.
Unknown/ignored/filtered/failed/unrecovered evidence stays explicit.

The measured basis is `evidence:root/20261107092034`: control d610ab4, workload
8132117, six matched passing suite/repeat pairs. Initial workspace build was
100.332s local versus157.570s GitHub; native process17.536s/31.047s; collector
40.546s/35.410s. Repeated build was3.572s/6.022s. Image, Rust/lock identity,
4CPU quota, 2GiB memory limit and UID0 matched; nproc was32 versus4. This does not
prove VM equivalence, fully cold caches or a universal speedup. The2GiB container
limit belongs only to that measured profile, not SDK, CEF or model workloads.

User-authorized local model setup/smoke is a separately bounded component profile,
not a substitute for the required product, venue-specific or full paired gates.

Publication is not currently authorized: no tags, releases, package upload or OIDC
token exchange. Protected GitHub environment setup and planned publisher bindings
are preparation only. The full program goal remains owned by the integration chat.
