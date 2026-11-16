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

Until Root activates the pull-request flow (phase 2 of the plan: rulesets
committed **and** applied, required checks named), a cloud session writes only to
the branch its owner assigned, which today is `codex/rust`. It pushes only while
an owner push permission is in force. Once activated, a session works on its own
`claude/`, `codex/` or similar branch and opens a pull request. A person still
merges.

Release, tag, OIDC exchange, Pages deploy, main merge and ruleset application
stay with a person (rule 93).

## Commits

Rule 92 attribution still applies. `git-coord.sh preflight` is recommended. The
PowerShell path is optional in this mode, because the container has no `pwsh`
and no shared board. Stage only owned paths.

## Docker

The container's own Docker engine, or the CI `docker` job on a GitHub-hosted
runner, is the venue (rule 96, "Cloud containers"). Record which venue ran. A
scaffold image that echoes a fixed receipt is never product e2e (rule 60).
