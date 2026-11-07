# Chat names, goals and execution ownership

Project chat titles use `Humanymous · <area> · <responsibility>`.
Use one stable bounded-context area and a short responsibility, not an instruction,
path, transient status, priority or full goal. Keep the project name's spelling.
Server adapters use a language-neutral responsibility as their coverage expands.
The exact current titles and ownership live in `.agents/sessions/THREADS.md`.
Only rename chats in this project; preserve unrelated projects and historical runs.

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
