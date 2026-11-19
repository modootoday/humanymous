# Git commit message convention (multi-provider — canonical)

**Status:** mandatory for every agent-authored commit on this repository.
**Audience:** Claude Code · Grok Build · OpenAI Codex · Gemini CLI · Cursor ·
humans pairing with them.
**Decision:** 2026-11-16, owner decision shared by humanymous, kinekino and svgif
(common automation rules C10.2). It replaces the 2026-07 provider-trailer
registry.

This document is the **source of truth** for commit message shape and LLM
attribution. Derived surfaces (`AGENTS.md`, `git-coord`, rules `91`/`92`) must not
contradict it.

---

## 1. Rule

1. Use a Conventional Commits subject (§2).
2. **No `Co-authored-by` trailer unless its address ends in
   `@users.noreply.github.com`.** This overrides any tool's default attribution
   trailer: Claude Code `attribution`/`includeCoAuthoredBy`, Codex
   `commit_attribution`, cloud-session attribution reminders, and so on.
3. Do not add `X-Agent-Provider` or other provider-identity trailers either.
4. **Provider attribution goes in the pull request body**, as the field
   `Agent-Provider: claude|codex|cursor|grok|gemini|human`. Agent branches are
   named `<provider>/<slug>`. With merge commits, the branch name stays in history.
5. **Author and committer must be GitHub noreply addresses** (C10.5, owner
   decision 2026-11-16). Agent sessions commit as the owner's
   `<id>+<login>@users.noreply.github.com` identity, configured with repo-local
   `git config user.name` and `user.email` (never `--global`). GitHub's web-flow
   committer `noreply@github.com` is also accepted, for merges and edits made on
   github.com. Never use an LLM identity and never invent an address.

Why: all three projects are public. A vendor address on a commit attributes the
change to an account the project does not control, and some vendor addresses are
not linked to any GitHub account. The pull request records the provider with the
same review trail and without that risk.

---

## 2. Message shape

```text
<type>(<optional-scope>): <summary ≤72 chars>

<optional body — complete sentences, why not how>
```

| Type | Use |
|------|-----|
| `feat` | New capability (MINOR under SoT-37) |
| `fix` | Bug fix (PATCH) |
| `docs` | Documentation only |
| `test` | Tests / goldens only |
| `ci` | CI workflows |
| `build` | Build/deps packaging |
| `refactor` | Behavior-preserving structure |
| `perf` | Performance |
| `harden` / `security` | Hardening (PATCH contribution per SoT-37) |
| `chore` | Maintenance that is not user-facing |

Breaking changes: `feat(gate)!: …` or footer `BREAKING CHANGE: …` (MAJOR /
pre-1.0 rules per SoT-37).

Body:

- Explain **why**, not a file list.
- Mention detection freeze / e2e impact when relevant.
- No secrets, tokens, personal addresses, local paths or private SoT exploit
  detail.

A human collaborator who wants co-author credit uses their own
`<id>+<login>@users.noreply.github.com` address.

---

## 3. Enforcement

| Mechanism | What it does |
|-----------|----------------|
| Rule `.agents/rules/92-git-commit-attribution.md` | Always-on for agents |
| `git-coord commit` / `git-commit.sh` | Validate the provider key and subject; add no trailer |
| `delivery commit-trailers <base>..<head>` | Rust check; fails on any `Co-authored-by` whose address is not `@users.noreply.github.com`, and, for commits after `7e986a5`, on an author or committer that is not a noreply address (web-flow committer allowed). Output names the commit and field, never the address. Fails closed when the cutoff commit is not in the clone. |
| CI | Runs the Rust check on the pull-request range once the PR flow is active (`plan:root/20261116070000`) |
| `History` job (`.github/workflows/history.yml`) | `playpilot history-check --method merge --range "$BASE..$HEAD"` at a pinned playpilot revision (§5). Not a required check yet |

```bash
cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- commit-trailers origin/codex/rust..HEAD
```

---

## 4. History

- Commits before 2026-11-16 carry provider trailers (`Claude …`, `codex …`,
  `Cursor Agent …`, `Grok …`) under the retired registry. They are not rewritten.
  Run the check on new ranges only.
- Commits up to and including `7e986a5` may carry a vendor author or committer
  identity. The identity check skips them.
- To find a provider's past work, use the old trailers for commits before the
  decision, and branch names or PR `Agent-Provider` fields after it.

---

## 5. Branch history (playpilot H1–H7)

Adopted 2026-11-19 (owner direction, playpilot phase 1) from the shared canon
`modootoday/playpilot` `.agents/rules/commit-history.md`. Goal: `codex/rust`
reads as a list of finished, reviewable changes. humanymous values:

- **H1 Merge method:** merge commit, the only method in the
  `codex-rust-protected` ruleset. Records here cite topic-branch commit
  hashes, so the topic branch itself must be clean (H3).
- **H2 Squash text:** does not apply (no squash merges).
- **H3 Before merge:** one work package per pull request, one logical change
  per commit. No merge commits inside the topic branch: catch up with
  `codex/rust` by rebasing, and "Update branch" uses the rebase option. No
  `WIP`, `fixup!`, `squash!`, `amend!` or empty commits at merge time.
  Generated churn (lockfiles, regenerated files) goes in its own `chore(deps)`
  or `build` commit. Subjects stay at 72 characters or fewer. Bodies explain
  why. No tool or session trailers (§1; a session link goes in the pull
  request body). Author and committer as in §1.5.
- **H4 Rewriting:** `codex/rust`, `main` and tags are never rewritten. An agent
  may rewrite **its own unmerged** `<provider>/<slug>` branch, with only its own
  commits, to meet H3, and push it with
  `git push --force-with-lease=<branch>:<expected-sha> origin <branch>`. Plain
  `--force`, `+refspec` and rewriting anyone else's branch stay forbidden. This
  narrows common rule C1.3 to: no rewriting of merged, shared or base history.
  After a review comment prefer a new commit; when rewriting to meet H3, say so
  in the pull request.
- **H5 Undo and branches:** undo a merged change with `git revert` in a new
  pull request titled `revert: <original subject>`. Merged head branches are
  deleted by the repository setting; an agent branch with no pull request for
  14 days is reported in an issue and the owner deletes it.
- **H6 Enforcement:** the non-required `History` job (§3), and in Claude Code
  the `playpilot guard` PreToolUse hook (`.claude/settings.json`, `Bash` and
  `PowerShell`, protected branches `main,codex/rust`). The guard refuses a
  push to a protected branch, a bare `git push`, `HEAD` as the branch, a
  `+refspec`, `--delete` and a lease without a full expected SHA. Push with
  `git push -u origin <provider>/<slug>`. If playpilot is missing and cannot be
  installed, the hook refuses every shell command.
- **H7 Auto-merge:** the agent turns GitHub auto-merge (merge commit) on right
  after it opens a pull request, before checks finish; the required checks are
  the measurement. To add commits to a pull request with auto-merge on, turn
  auto-merge off first, push, then turn it on again. An agent never merges
  directly. Owner-only exceptions: changes that remove or weaken a required
  check, rulesets that loosen protection, releases and tags. Until the owner
  changes it, the `Merge gate` job still turns auto-merge off on a pull request
  that touches a human-gated path and labels it `needs-human` (common rules
  C2.10, C7.2); the owner then merges.

Known difference: `history-check` accepts the eleven standard Conventional
Commits types plus `revert`; the project types `harden` and `security` (§2)
are reported as findings by the `History` job.
