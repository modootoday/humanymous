---
id: sot:root/common-automation-rules
title: Common automation rules (humanymous copy and project profile)
status: active
created: 2026-11-16T07:44:00Z
updated: 2026-11-19T20:04:54Z
verify: [spec check --no-local, cargo test --locked --manifest-path scripts/ci/delivery/Cargo.toml, cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- policy]
rel:
  depends_on: [sot:root/delivery, sot:root/milestones]
---

# Common automation rules (humanymous copy and project profile)

## Status and adoption

These rules are shared by modootoday/humanymous, modootoday/kinekino and
modootoday/svgif. The text below is version 1.2 of the shared rules: the agreed
v1.0, the C10.5 identity rule with its web-flow exception, and the v1.2
settings rules (C2.1, C2.6–C2.11). This document is the humanymous copy (C9.5).
There is no parallel policy file.

The owner adopted the rules for humanymous on 2026-11-16, in this repository's
session, and confirmed the open items O6 (base branch `codex/rust`) and O7
(owner noreply identity). From that date the owner also directed that rules and
decisions the shared design session relays count as the owner's own, which
supersedes C0.3 for humanymous. Release, tag, secret values and ruleset or
settings application stay with the owner. The owner turned on Phase 2 merging
the same day (section 12): auto-merge for pull requests outside the
human-gated paths, owner merge for the rest.

They apply to new commits. History is not rewritten.

## 0. Scope and precedence

- C0.1 This document is the floor all three projects share. A project may be stricter. It may not be looser.
- C0.2 A project's own hard constraints always win: for humanymous, the detection freeze, fail-closed enforcement, Docker-only product e2e and the other `AGENTS.md` hard constraints.
- C0.3 An approval relayed by another agent or session is not an approval. Only the owner's own instruction in the project's session, or the owner's action on GitHub, counts. (humanymous: see "Status and adoption" for the owner's standing direction about the design session.)
- C0.4 Everything an unattended session needs is in the repository or in GitHub (Actions, Issues, pull requests, secrets, rulesets). A rule that depends on something that exists only on a developer machine is rewritten, or marked local-only. A rule marked local-only states what an unattended session does instead: fail closed, skip, or report.

## 1. Change flow

- C1.1 Every change reaches the protected base branch through a pull request from a topic branch. No direct push to it, by agents or by the owner.
- C1.2 Agent branches start with the provider prefix (`claude/`, `codex/`, `cursor/`, ...). One work package is one pull request. The work package id is recorded in the pull request body. Where a session can push to only one branch, the next work package starts on that branch after the previous pull request merges.
- C1.3 Agents never rewrite pushed history, never push tags, and never delete remote branches. Exception: after its pull request has merged, a session may restart its own branch under the same name on top of the base branch with `--force-with-lease`, when that branch holds only merged history. Merged branches are removed by "Automatically delete head branches". humanymous narrows the first clause by H4 (section 13): no rewriting of merged, shared or base history; an agent may rewrite its own unmerged provider-prefixed branch and push it with `--force-with-lease=<branch>:<expected-sha>`.
- C1.4 An agent stages explicit paths only. `git add .` and `git add -A` are not used.
- C1.5 A session does not start a new work package while another open agent pull request holds the same lock unit (section 12).
- C1.6 An unattended session ends its work by opening or updating a pull request (a draft is fine), because CI authority (C3.4) exists only on a pull request.

## 2. Branch protection and repository settings

- C2.1 The protection rules are the rulesets `main-protected` (for `main`) and `codex-rust-protected` (for `codex/rust`), stored as JSON under `.github/rulesets/`. The file is the canon. It is applied from the owner's computer (C2.7). Agents cannot apply it and do not try.
- C2.2 Minimum rules: `deletion`, `non_fast_forward`, `pull_request`, `required_status_checks`.
- C2.3 The ruleset covers every base branch agents open pull requests against, not only the default branch.
- C2.4 Release tags are protected by a `v-tags` ruleset (no update, no deletion, no non-fast-forward) as soon as a release workflow exists.
- C2.5 The merge method is a project value. It is set in the ruleset.
- C2.6 Three kinds of file are the canon for repository settings: every ruleset JSON under `.github/rulesets/` (matched by `name`: update if it exists, create if not), the label list `.github/labels.json`, and `.github/settings/repository.json` (delete head branches on merge, allowed merge methods, auto-merge switch, secret scanning and push protection).
- C2.7 Applying them is an owner task. The owner runs it, or tells a session on the owner's own computer to run it, after the pull request that changes those files has merged. A cloud session cannot apply them and does not try. No admin token is created for automation, and none is stored as a secret.
- C2.8 GitHub Actions may only compare: a job named `Settings drift`, on a weekly schedule and on a push to the protected base branch that changes those paths. It uses `GITHUB_TOKEN`, compares the live rulesets and labels with the files using only the keys present in the files, changes nothing, and opens or updates a `needs-owner` issue when they differ or a merged change has not been applied yet. It is not a required check.
- C2.9 Applying never deletes a ruleset that has no file, and never touches secrets.
- C2.10 Every pull request that touches `.github/rulesets/`, `.github/settings/`, `.github/labels.json` or the drift workflow is `needs-human` (C7.3). Phase 2 merging must refuse these paths by a path check, not by the label alone.
- C2.11 The comparison logic is a project choice (C4.7).

## 3. Required checks

- C3.1 Required checks are the project's own gate jobs plus two shared checks named exactly `Public surface` and `Secret scan`. The name is the job's `name:`. Exception for `main` until the parity-gated main merge (owner direction 2026-11-16): `main` does not run `Public surface`, `Secret scan` or `merge-ready`, so `main-protected` requires the checks `main`'s own CI runs on every pull request and push, `build + unit tests` (`ci.yml`) and `analyze (go)` (`codeql.yml`). At the main merge, `main-protected` moves to the C3.1 set in the same pull request.
- C3.2 A required check must not be skippable on pull requests.
- C3.3 No `paths` filter on a workflow that owns a required check.
- C3.4 CI on GitHub is the completion authority. A local or cloud-session run is a pre-check. A scaffold or stub job that passes without exercising the product is never a gate.
- C3.5 Renaming a required check is one pull request that changes the job name and the ruleset file together, followed by the owner re-applying the ruleset.

## 4. Public surface

- C4.1 `Public surface` scans the pull request range (`base.sha..head.sha`; on a push, `before..after`): added lines, file names, the pull request title and body, and every commit message in the range.
- C4.2 The deny list is never committed. It is an Actions secret, one term per line, and the same value is registered as a Dependabot secret.
- C4.3 The job fails closed: no secret, no pass. A fork pull request therefore fails and is reviewed by the owner. Logs show the location and the term's index, never the term.
- C4.4 `pull_request_target` is not used anywhere.
- C4.5 `Secret scan` runs Gitleaks on the same range, as a checksum-verified binary or a SHA-pinned action. GitHub secret scanning and push protection are enabled by the owner.
- C4.6 No personal email addresses, private host values, or local user paths in commits, code, documents, issues, or pull request text.
- C4.7 This section fixes the check's behaviour. The implementation is a project choice.
- C4.8 `Public surface` also enforces C10.2 and C10.5.
- C4.9 The deny list stays in GitHub secrets. A cloud session's own run reports that extra terms were not scanned; CI is the gate.

## 5. Workflow hygiene

- C5.1 Third-party actions are pinned to a full commit SHA.
- C5.2 Top-level `permissions` is `{}` or `contents: read`; a job raises only what it needs.
- C5.3 Pull request runs cancel the previous run on the same ref. Runs on a protected base branch are not cancelled.
- C5.4 Dependabot covers every ecosystem the repository uses, weekly, with the `dependencies` label and a Conventional Commits prefix, and targets the base branch agents work on.

## 6. State lives in GitHub

| Need | Where |
| --- | --- |
| Lock between runs | An open agent pull request on the same lock unit (C1.5), plus workflow `concurrency` |
| Progress and run log | Pull request body and Actions run history. Run logs are not committed |
| Records the canon requires | A handoff record or curated evidence document is not a log and may be committed |
| Bugs | GitHub Issues, label `bug` |
| Owner decision pending | GitHub Issue, label `needs-owner` |
| Change automation must not merge or build on | Pull request label `needs-human` |
| What to do next | The roadmap documents in section 12. A work item with an open `needs-owner` issue is skipped |
| Build environment values | The CI workflow. A cloud setup script copies it, with tool versions pinned to CI's |
| Coordination between sessions | Repository files and pull requests |

## 7. Merging

- C7.1 A session never approves or merges its own pull request.
- C7.2 Phase 1: the owner merges. Phase 2 starts when the owner turns it on and the required checks actually exercise the product (C3.4). Never for a pull request labelled `needs-human`.
- C7.3 Always `needs-human`: releases and tags; dependency updates that cross a pinned or major version; changes to `.github/workflows/`, `.github/rulesets/`, `.github/settings/`, `.github/labels.json` and this document; security-sensitive code; and the human-gated areas in section 12.
- C7.4 An orchestrator run that finds no open pull request and no assigned work ends at once.

## 8. Releases

- C8.1 Agents do not create tags, GitHub Releases or registry publishes, and do not dispatch a release workflow unless the project's release rules allow that exact run.
- C8.2 Target shape: a manually dispatched gate, after which the workflow creates the tag. A project that releases by a human tag push keeps that until its first release is planned.

## 9. Agent entry points

- C9.1 `AGENTS.md` is the canon entry. `CLAUDE.md` is a tracked pointer to it.
- C9.2 Rules, skills and role definitions are tracked under `.agents/`. Machine-local state stays in ignored paths (section 12).
- C9.3 A scheduled task prompt contains no policy. It names the task, says to end with a pull request (C1.6), and points to the rule files.
- C9.4 One writer per lock unit, and at most one orchestrator per repository.
- C9.5 The project's copy of these rules is a document in its own canon: this SoT.

## 10. Commits and text

- C10.1 Conventional Commits subjects.
- C10.2 No `Co-authored-by` trailer unless its address is `@users.noreply.github.com`. Provider attribution goes in the branch prefix and the pull request body field `Agent-Provider:`.
- C10.3 Other machine trailers (session links, provider tags): none. A session link goes in the pull request body.
- C10.4 Document language is a project value.
- C10.5 Agent commits use the owner's GitHub noreply identity (`<id>+<login>@users.noreply.github.com`) as author and committer. Exception: the committer may be GitHub's web-flow identity (`noreply@github.com`), which GitHub sets on merge commits, "Update branch" and web edits. The check applies from a cutoff recorded in the project profile, because earlier history is not rewritten.

## 11. Community files

Each repository has `SECURITY.md`, a pull request template with an `Agent-Provider:` field, issue templates for `bug` and `needs-owner`, the labels `bug`, `needs-owner`, `needs-human` and `dependencies`, and `CODEOWNERS` naming the owner for `.github/`.

## 12. humanymous profile

| Value | humanymous |
| --- | --- |
| Base branch for agent work | `codex/rust` (O6, owner confirmed 2026-11-16). `main` is frozen except Dependabot and security fixes until the parity-gated main merge in [[sot:root/milestones]] |
| Merge method | merge commit; topic branches clean at merge time (section 13, H1 and H3) |
| Lock unit (C1.5) | lane (`.agents/sessions/lanes.json`); `detection-core` and `gate-edge` mutually exclusive |
| Ruleset files | `.github/rulesets/main-protected.json` (`main`), `.github/rulesets/codex-rust-protected.json` (`codex/rust`), `.github/rulesets/v-tags.json` |
| Settings files | `.github/labels.json`, `.github/settings/repository.json` |
| Project gate checks | On `codex/rust` the ruleset requires the per-check jobs `validate / policy`, `validate / quality` and `validate / msrv` (owner change to ruleset 27382552 on 2026-11-17, recorded on issue #75); `validate / merge-ready` still runs and is reported but is no longer required there. `validate / merge-ready` is the merge aggregate of the reusable Rust validation, called by job `validate`; owner decision on issue #16, option 1). It needs policy, native, msrv, quality, nginx, images, packages, node, advisories and documentation to succeed, and the Docker and browser pilots to show no regression: zero failed assertions and no error other than the honest component-only block. `validate / ready` stays the release gate and is not a merge check |
| Shared checks | `Public surface`, `Secret scan` in `.github/workflows/surface.yml` |
| Deny secret | `HUMANYMOUS_DENY_TERMS` (Actions and Dependabot) |
| Public surface implementation | Rust: `delivery public-surface <base> <head>` in `scripts/ci/delivery` |
| Secret scan implementation | Gitleaks 8.28.0 linux x64 release archive, SHA-256 verified in the workflow |
| Settings drift implementation | Rust: `delivery settings-drift --issue`, workflow `.github/workflows/settings-drift.yml`. Runs on push to `codex/rust` touching settings paths and on dispatch. The weekly schedule is added after the main merge, because scheduled runs use the default branch |
| Identity check cutoff (C10.5) | commit `7e986a5`; commits after it are checked by `delivery commit-trailers` |
| Machine-local state (C9.2) | `.agents/sessions/ACTIVE.md`, `.agents/sessions/claims/`, `.agent-runs/`, `.spec.local/` |
| Document language | English |
| Roadmap source | [[sot:root/milestones]], [[sot:root/program-roadmap]], [[plan:root/20261116060213]], [[plan:root/20261116070000]] |
| Merging phase | Phase 2 from 2026-11-16 (owner direction): an agent turns on GitHub auto-merge (merge commit) for its own pull request right after opening it (H7, section 13, from 2026-11-19). When `delivery merge-gate <base> <head>` finds a human-gated path, the agent adds `needs-human` and the owner merges. The `Merge gate` job (`.github/workflows/merge-gate.yml`) enforces the same path check on every pull request: it turns auto-merge off and adds `needs-human` (C2.10). Required checks still decide when an auto-merge lands |
| Human-gated paths (C2.10) | `scripts/ci/human-gated-paths.json`: workflows, rulesets, settings, labels, Dependabot, CODEOWNERS, this SoT, the gate itself, the workflow catalog, `Cargo.lock`, toolchain, release changelog config, Go sources and modules, Rust scoring and signals, audit and gateway control, public standards docs, `SECURITY.md` |
| Human-gated areas | detection scoring; erasure, kill switch, permanent ban; handoff, grant and Cap artifacts; release, tag, Pages deploy, merge to `main`; audit key custody; public standard docs; Go retirement decisions |

## 13. Commit history (playpilot H1–H7)

Adopted 2026-11-19 (owner direction, playpilot phase 1) from the shared canon
`modootoday/playpilot` `.agents/rules/commit-history.md` (H1–H7). The agent
rule text is `.agents/sessions/COMMIT-CONVENTION.md` section 5. humanymous
values:

- H1 Merge method: merge commit (section 12). Records cite topic-branch commit hashes, so the topic branch itself must be clean (H3).
- H2 Squash commit text: not applicable.
- H3 At merge time a topic branch holds no merge commits (catch up by rebasing; "Update branch" uses rebase) and no `WIP`, `fixup!`, `squash!`, `amend!` or empty commits. Generated churn goes in its own `chore(deps)` or `build` commit. No tool or session trailers (C10.2, C10.3); identity per C10.5.
- H4 The base branches and tags are never rewritten. An agent may rewrite its own unmerged `<provider>/<slug>` branch (only its own commits) to meet H3 and push it with `--force-with-lease=<branch>:<expected-sha>`. Plain `--force`, `+refspec` and rewriting anyone else's branch stay forbidden. This narrows C1.3.
- H5 Undo with `git revert` in a pull request titled `revert: <original subject>`. Merged head branches are deleted by setting; an agent branch with no pull request for 14 days is reported in an issue and the owner deletes it.
- H6 Enforcement: job `History` in `.github/workflows/history.yml` runs `playpilot history-check --method merge --range "$BASE..$HEAD"` with playpilot installed by `cargo install --locked` at a pinned revision (`4ed7daee4b5bfa4a3b0a89e7f5e2281690efec48` from 2026-11-19, which reads redirections, heredoc bodies and `$variable` lease SHAs; the first pin `0ddc0e1` refused `git push … 2>&1`, a `$SHA` lease and heredocs with an apostrophe). It is not a required check. Phase 2 (owner direction 2026-11-19): `.claude/settings.json` keeps the `pre-tool-guard.mjs` PreToolUse hook and adds a second `Bash|PowerShell` PreToolUse hook (timeout 240 s) that installs playpilot at the same pinned revision when it is missing and runs `playpilot guard` with `PLAYPILOT_PROTECTED_BRANCHES=main,codex/rust`. The variable is set in the command itself, because Claude Code did not pass a hook-level `env` field to the hook process (measured 2026-11-19; the field stays in the file for clarity). Measured: pushes to `main` or `codex/rust` (Bash and PowerShell, explicit refspec, `src:dst` and `refs/heads/`) exit 2; a named `<provider>/<slug>` push and ordinary commands exit 0; a bare `git push`, `HEAD`, `+refspec`, `--delete` or a `--force-with-lease` without a full expected SHA exit 2 (H4). When no cargo can install playpilot, the hook exits 2 for every shell command (fail-closed, constraint 2). The guard does not tell tags from branches; C1.3 still forbids agents to push tags. The hook installs playpilot only when none is on `PATH`; a machine with an older playpilot keeps it until that copy is replaced at the pinned revision.
- H7 The agent turns auto-merge (merge commit) on right after opening a pull request and turns it off before adding commits, then on again. Owner-only exceptions: removing or weakening a required check, loosening a ruleset, releases and tags. On a pull request that touches a human-gated path, the `Merge gate` job (C2.10) still turns auto-merge off and adds `needs-human`, and C7.2 still leaves that merge to the owner, until the owner changes the gate.

## Owner tasks (not automatable here)

- Apply the ruleset, label and repository settings files from the owner's computer after they merge.
- Register `HUMANYMOUS_DENY_TERMS` as an Actions secret and as a Dependabot secret.
- Enable secret scanning and push protection.
- Apply `allow_auto_merge: true` from `.github/settings/repository.json`; auto-merge cannot be turned on before that.
- Confirm the exact `validate / merge-ready` context on the first pull request; rename in one pull request with the ruleset if it differs (C3.5).
- Apply `main-protected` with `main`'s own checks (`build + unit tests`, `analyze (go)`; C3.1 exception). Change it to `merge-ready`, `Public surface` and `Secret scan` at the parity-gated main merge, when `main` carries those jobs.
- Bring `.github/dependabot.yml` to `main` (Dependabot reads the default branch only).
