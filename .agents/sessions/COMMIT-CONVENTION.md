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
5. Never change git `author.name`/`author.email` to an LLM identity, and never
   invent an address.

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
| `delivery commit-trailers <base>..<head>` | Rust check; fails on any `Co-authored-by` whose address is not `@users.noreply.github.com` |
| CI | Runs the Rust check on the pull-request range once the PR flow is active (`plan:root/20261116070000`) |

```bash
cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- commit-trailers origin/codex/rust..HEAD
```

---

## 4. History

- Commits before 2026-11-16 carry provider trailers (`Claude …`, `codex …`,
  `Cursor Agent …`, `Grok …`) under the retired registry. They are not rewritten.
  Run the check on new ranges only.
- To find a provider's past work, use the old trailers for commits before the
  decision, and branch names or PR `Agent-Provider` fields after it.
