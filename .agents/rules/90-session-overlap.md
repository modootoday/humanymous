# Multi-provider session overlap (always-on)

Before non-trivial edits in this repo:

1. **Read the board** — `.agents/sessions/ACTIVE.md` (seed via `session-board list` if missing).
2. **Claim a lane** — `scripts/agents/session-board.* claim` (see `.agents/sessions/LANES.md`).
3. **Do not write into another session’s claimed paths** without user-approved force-release.
4. **Release on exit** — even if incomplete; leave a handover on the board.
5. **Provider switch** = release + handover first; never “silently continue” in a new tool.
6. **Git writes** — exclusive `git-ops` claim via `git-coord` (see `91-git-contention.md`).

Parallel work is encouraged **across different work lanes**. Same lane = single writer.
Git mutations are always single-writer regardless of work lane.

Shared scoring / Gate seams: treat `detection-core` and `gate-edge` as **mutex** unless
paths are explicitly disjoint and both claims document that.

Durable state lives in the board + `.agents/lessons/` — not only in one provider’s chat memory.

Skill: `coordinate-sessions`. Protocol: `.agents/sessions/PROTOCOL.md`.
