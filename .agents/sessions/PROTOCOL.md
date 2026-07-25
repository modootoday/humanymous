# Multi-session / multi-provider protocol

## Why this exists

Overlapping sessions cause:

1. **File races** — two agents edit the same package and silently overwrite each other.
2. **False done** — one agent marks complete while another still owns the failing path.
3. **Handoff loss** — Claude memory is invisible to Grok/Codex/Gemini.
4. **Double commits / thrash** — same fix applied twice in divergent ways.
5. **Detection freeze breaches** — parallel “small” scoring edits compound.
6. **Git races** — concurrent `add`/`commit`/`pull`/`push` fight over `index.lock`, HEAD, and the remote.
7. **Wargame theater** — bulk empty commits or mid-series git noise during red/blue series
   (rule `61`, skill `red-blue-wargame-round`: default no per-round commits).

## Session lifecycle (mandatory)

```text
START → READ board → CLAIM work-lane → WORK → VERIFY
       → CLAIM git-ops → git add/commit[/push] → RELEASE git-ops
       → RELEASE work-lane (+ handover if incomplete)
```

### 1. START (first tool actions)

1. Read `AGENTS.md` hard constraints.
2. Run `session-board list` (or open `.agents/sessions/ACTIVE.md`).
3. Run `git status` — note other dirty lanes.
4. If another claim is **stale** (>4h, or user says abandoned), release it with a note
   before re-claiming (do not steal a fresh claim without asking).

### 2. CLAIM

- Claim **one primary lane** before non-trivial edits.
- List concrete path globs in the claim.
- Set `provider` to one of: `claude` | `grok` | `codex` | `gemini` | `human`.
- Set `session` to a short id (provider session id if known, else `local-HHMM`).

**Conflict:** if the lane is held by another active claim → do not edit those paths.
Options: pick another lane, wait, or ask the user to force-release.

### 3. WORK

- Prefer edits only under claimed paths.
- **Read-only** elsewhere is always OK.
- Cross-lane dependency (e.g. docs need a new signal id): either
  - expand claim (ask if detection-core is free), or
  - leave a TODO on the board for the owning lane, do not drive-by edit.

### 4. VERIFY

- Unit tests for touched packages.
- Any e2e → Docker only (`make e2e` / skill `red-blue-validate`).
- Do not claim “green” if another session owns the failing surface.

### 5. GIT (serial)

- Read-only git needs no claim.
- Any write (`add`, `commit`, `pull`/`rebase`/`merge`, `push`, `stash`, branch switch):
  1. `git-coord preflight`
  2. `git-coord claim`
  3. perform git ops (stage **only your paths**)
  4. `git-coord release` immediately
- Full rules: `GIT-PROTOCOL.md`. **Never** force-push shared branches without user OK.

### 6. RELEASE work lane

- Always release the work lane when the session ends, stalls, or switches provider.
- If work remains: write a **handover block** into `ACTIVE.md` (and optionally run
  skill `handover-pack`).
- Promote durable lessons to `.agents/lessons/HARD-WON.md` — not Claude-only memory.

## Nested / stacked sessions

| Pattern | Rule |
|---------|------|
| Same provider, two chats | Two claims or sequential; never two writers on one lane |
| Grok + Claude same hour | Different lanes preferred; shared board is the source of truth |
| Provider switch mid-task | Release + handover first; new provider re-claims |
| User runs agent while another is mid-edit | New agent must list board before writing |

## Board format (ACTIVE.md)

See `ACTIVE.example.md`. Machine claims under `claims/<lane>.json` mirror the table
for scripts. **Human-readable ACTIVE.md wins on conflict** if a human edited it.

## Staleness

| Lane type | TTL |
|-----------|-----|
| Work lanes | **4 hours** |
| `git-ops` | **30 minutes** (must not be held across long coding) |

`session-board list` marks stale claims. Releasing stale claims is allowed with
`-Force` / `force` after a status note.

## What not to put on the board

- Secrets, tokens, OAuth material
- Full transcripts
- Speculative multi-week roadmaps (use SoT / plan instead)
