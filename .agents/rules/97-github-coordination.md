# GitHub-coordinated sessions (cloud and scheduled)

This rule applies to sessions that run in an ephemeral cloud container or as a
scheduled task. Such a session has none of these:

- the shared `C:\workspaces\humanymous` checkout;
- a persistent `.agents/sessions/ACTIVE.md` (it is gitignored);
- `pwsh`;
- private `.spec.local/` evidence.

Rules 90 and 91 assume those local facilities. Where this rule names a GitHub
replacement, the session uses it instead. Every other rule still applies.

Canon and phases: `plan:root/20261116070000` (GitHub infrastructure coordination).
The unattended run loop, authority matrix and permission model are
`sot:root/unattended-operation`.

## Coordination state

| Local facility | GitHub replacement |
| --- | --- |
| Board claim, lane occupancy | An open pull request or pushed branch whose description names one lane from `lanes.json`. One open item per lane. `detection-core` and `gate-edge` stay mutually exclusive. |
| `git-ops` mutex | The remote is the arbiter. A session is the only writer of its own clone. On a non-fast-forward push it stops, fetches and merges; it never forces. |
| Handover, heartbeat, progress | Commit messages, `.spec/evidence/*` documents, pull-request bodies and Actions run records. Never commit log files. |
| Owner decisions and bugs | GitHub Issues labelled `needs-owner` or `bug`. |
| `.agent-runs/` hook audit | Stays local and untracked. Actions run logs cover CI actions. |
| `.spec.local/` raw evidence | Stays private. CI `delivery policy` refuses tracked files. A cloud session has no private evidence, so Cap0 tools report every handoff as absent. Never recreate or template a handoff to fill that gap. |

## Branches and refs

The pull-request flow is active (owner decision 2026-11-16,
`sot:root/common-automation-rules` C1). A cloud session works on its own
`<provider>/<slug>` branch (`claude/`, `codex/`, ...), pushes it, and opens or
updates a pull request into `codex/rust`. The pull request body carries
`Agent-Provider:`, the work package and the lane. Phase 2 (owner direction
2026-11-16): when `delivery merge-gate <base> <head>` reports no human-gated
path, the session turns on auto-merge (merge commit) and moves on; otherwise it
adds the `needs-human` label and moves on, and the owner merges. Until the
`main-protected` ruleset is applied, the session still does not push to
`codex/rust` or `main` directly. Blocked work goes into the pull request body or
a `needs-owner` Issue, and the session moves to the next unit.

Merge, release, tag, OIDC exchange, Pages deploy, secret values and ruleset or
settings application stay with the owner (rule 93, common rules C2.7, C7, C8).

## Commits

Rule 92 applies: no vendor co-author trailer; author and committer are the
owner's noreply identity, set with repo-local `git config`; the provider is
named in the PR body and the branch name. `git-coord.sh preflight` is recommended. The
PowerShell path is optional in this mode, because the container has no `pwsh`
and no shared board. Stage only owned paths.

## Docker

The container's own Docker engine, or the CI `docker` job on a GitHub-hosted
runner, is the venue (rule 96, "Cloud containers"). Record which venue ran. A
scaffold image that echoes a fixed receipt is never product e2e (rule 60).
