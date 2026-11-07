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
actual hosted boundary tests, exact result accounting, Specfold records and an
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

Local work is coding, bounded pure units, fmt and scoped Clippy. Root may assign
at most two local pipelines: A for the approved shared target, B for an approved
lightweight isolated target. Both use build jobs1 and incremental0; an idle slot
is not authorization to start. Do not overlap targets or run CEF/native-heavy/
bindgen together. At commit>=88% or free physical memory<16GiB, defer new B work
and reduce the next allocation to one; never kill another owner's processes.
The single execution-policy canon remains program-roadmap/delivery SoTs, including
resource measurements and the root lock-integration window for manifest changes.
Actual process/IPC/SCM/browser/Docker/Cross-OS tests use
the standard free hosted lanes; do not substitute mocks or silently skip required
work. Unknown/ignored/filtered/failed/unrecovered evidence stays explicit.

Publication is not currently authorized: no tags, releases, package upload or OIDC
token exchange. Protected GitHub environment setup and planned publisher bindings
are preparation only. The full program goal remains owned by the integration chat.
