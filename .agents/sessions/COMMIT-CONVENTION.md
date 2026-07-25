# Git commit message convention (multi-provider — canonical)

**Status:** mandatory for every agent-authored commit on this repository.  
**Audience:** Claude Code · Grok Build · OpenAI Codex · Gemini CLI · humans pairing with them.  
**Research baseline:** 2026-07 (vendor docs, GitHub API, product source).

This document is the **source of truth** for commit message shape and **LLM attribution**.
Derived surfaces (`AGENTS.md`, `git-coord`, rules `91`/`92`) must not contradict it.

---

## 1. Why attribution is required

Multiple LLM providers edit this tree. Without a stable trailer:

- `git log` cannot show which tool produced a change
- incident review cannot map regressions to a provider session
- session-board claims alone are ephemeral (gitignored, machine-local)

We follow the **same trailer pattern Claude Code ships by default**
(`Co-Authored-By: Name <email>`), using **vendor GitHub-linked emails**
so GitHub can resolve **profile avatars / contributor cards**.

---

## 2. How GitHub resolves co-author avatars

GitHub matches `Co-authored-by` trailers by **email address**, not display name.  
The email must be associated with a GitHub user or bot account.  
Fake domains (e.g. `@agents.humanymous.local`) **never** show a vendor avatar.  
Inventing unowned vendor addresses (e.g. random `noreply@…`) risks
**mis-attribution to third parties** who claim that email.

Authoritative rules: [Creating a commit with multiple authors](https://docs.github.com/en/pull-requests/how-tos/commit-changes/creating-a-commit-with-multiple-authors).

---

## 3. Message shape (Conventional Commits + trailers)

```text
<type>(<optional-scope>): <summary ≤72 chars>

<optional body — complete sentences, why not how>

Co-Authored-By: <Display Name> <vendor-or-bot-email>
```

### 3.1 Subject (`type`)

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

Breaking changes: `feat(gate)!: …` or footer `BREAKING CHANGE: …` (MAJOR / pre-1.0 rules per SoT-37).

### 3.2 Body

- Explain **why**, not a file list.
- Mention detection freeze / e2e impact when relevant.
- No secrets, tokens, or private SoT exploit detail in public commits.
- Claude Code often also inserts a free-text line such as
  `Generated with Claude Code` — optional; **trailer is mandatory**.

### 3.3 Required attribution trailer

Exactly one primary agent trailer (GitHub accepts `Co-authored-by` /
`Co-Authored-By`; we emit **`Co-Authored-By`** for Claude parity):

```text
Co-Authored-By: <Display Name> <email>
```

Optional human director:

```text
Co-Authored-By: <Human Name> <human@example.com>
```

Optional machine-parseable provider id (not shown as a GitHub co-author):

```text
X-Agent-Provider: claude|grok|codex|gemini|human
```

---

## 4. Provider registry (GitHub-linked identities)

| Provider key | Canonical trailer | GitHub avatar? | Official channel / notes |
|--------------|-------------------|----------------|---------------------------|
| `claude` | `Co-Authored-By: Claude <noreply@anthropic.com>` | Yes when email is linked to Anthropic’s Claude identity (historically `github.com/claude`; early mis-claim of this address is documented and fixed on vendor side) | **Claude Code product default.** Model-tagged form also seen: `Claude Sonnet X.Y <noreply@anthropic.com>`. Toggle: `includeCoAuthoredBy` / `attribution` in Claude settings. |
| `codex` | `Co-Authored-By: codex <codex@openai.com>` | **Yes** — community-confirmed avatar for OpenAI Codex GitHub identity | Prefer this for UI. Product source default is also `Codex <noreply@openai.com>` (`commit_attribution.rs`); **both accepted** by checker. Connector bot alt: `chatgpt-codex-connector[bot] <199175422+chatgpt-codex-connector[bot]@users.noreply.github.com>`. |
| `grok` | `Co-Authored-By: Grok <grok@x.ai>` | **No avatar yet** — email is **not** linked to a GitHub user (API `author.login` null on public commits). Community / multi-agent de-facto (PrimeFaces, lfortran, …). | See **§4.1**. Optional model tag in display: `Grok 4 <grok@x.ai>`. Do **not** use third-party logins `github.com/grok` / `github.com/xai`. |
| `gemini` | `Co-Authored-By: Gemini CLI <gemini-cli@users.noreply.github.com>` | **No reliable vendor avatar** (placeholder until Google publishes a linked identity) | No Claude-parity default found. Do **not** invent `noreply@google.com`. Revisit when Google ships a bot/`users.noreply` id. |
| `human` | _(omit agent trailer)_ | human’s account | Pure human commits: no fake agent trailer. |

**Do not** use personal human email as the agent identity.  
**Do not** invent ad-hoc vendor emails not listed above.

### 4.1 Grok / xAI research (canonical decision)

| Finding | Detail |
|---------|--------|
| Official org | [`github.com/xai-org`](https://github.com/xai-org) — “SpaceXAI Org”, blog `https://x.ai/`. |
| Official product repo | [`github.com/xai-org/grok-build`](https://github.com/xai-org/grok-build). |
| Product Co-Authored-By | **Not shipped** as a Claude-style default in Grok Build docs (2026-07). |
| **Community de-facto trailer** | `Co-Authored-By: Grok <grok@x.ai>` (or `Grok 4 <grok@x.ai>`). Used in public CONTRIBUTING guides (e.g. PrimeFaces) and ~1.7k public commits with that author email. |
| Avatar status | **`grok@x.ai` is not associated with any GitHub account** → co-author chip has **no profile photo** until xAI links the address (same class of risk as early Claude `noreply@anthropic.com` mis-claims). |
| Official monorepo bot (not for agent trailers) | `grokkybara[bot]` on `xai-org/grok-build` only — monorepo sync, not end-user agent identity. |
| **Not** xAI | [`github.com/grok`](https://github.com/grok), [`github.com/xai`](https://github.com/xai), [`github.com/grokxai`](https://github.com/grokxai) — third parties. **Never** use for trailers. |
| **This repo’s choice** | Follow **community convention** `Grok <grok@x.ai>` for string parity with Claude/Codex-style trailers. Prefer avatar-linked identity only after xAI publishes one. |

If xAI later ships a GitHub-linked co-author email, **update this table first**, then `git-coord` / checker.

### 4.2 Example commits

```text
feat(e2e): Docker-native attack-assert service

Fail closed on profile errors and skips. Assertions run inside compose.

Co-Authored-By: codex <codex@openai.com>
X-Agent-Provider: codex
```

```text
feat(agents): session lanes and exclusive git-ops

Co-Authored-By: Grok <grok@x.ai>
X-Agent-Provider: grok
```

```text
docs: fix operator runbook typo

Generated with Claude Code

Co-Authored-By: Claude <noreply@anthropic.com>
X-Agent-Provider: claude
```

---

## 5. Enforcement

| Mechanism | What it does |
|-----------|----------------|
| **Normative rules** | `.agents/rules/92-git-commit-attribution.md` (always-on for agents) |
| **git-coord commit** | Builds the message + injects trailers; refuses missing provider |
| **git-coord preflight** | Reminds that attribution is required |
| **check-commit-attribution** | Scans recent commits; fails if agent-looking commits lack known trailers |
| **CI (optional local)** | May call the checker on `main` PR range later |

### 5.1 Required workflow for agents

```powershell
pwsh -File scripts/agents/git-coord.ps1 preflight
pwsh -File scripts/agents/git-coord.ps1 claim -Provider grok -Session "…"
git add <only-your-paths>
pwsh -File scripts/agents/git-coord.ps1 commit -Provider grok -Subject "feat(agents): …" -Body "…"
# or: -MessageFile path/to/msg.txt  (subject+body only; trailers appended)
pwsh -File scripts/agents/git-coord.ps1 release -Note "committed <sha>"
```

**Agents must not** call raw `git commit -m` without the attribution trailer.  
If a host tool injects its own `Co-Authored-By` (Claude Code), keep the
**vendor registry** line; do not strip Claude’s trailer when it matches §4.

### 5.2 Multi-provider single commit

When one commit intentionally merges work from two agents (rare; prefer separate commits):

```text
Co-Authored-By: Grok <grok@x.ai>
Co-Authored-By: codex <codex@openai.com>
X-Agent-Provider: grok
X-Agent-Provider: codex
```

List the **primary** implementer first. Document the fold in the body.

---

## 6. What is out of scope

- Rewriting historical commits that lack trailers (no force history rewrite).
- Changing git `author.name` / `author.email` to the LLM (author stays the human/machine account; **trailer** carries agent identity).
- Legal copyright claims — trailers are **engineering provenance**, not a legal assertion.
- Claiming `github.com/grok` or `github.com/xai` as xAI product identities (they are not).

---

## 7. Verification

```powershell
pwsh -File scripts/agents/check-commit-attribution.ps1
# optional: -Since HEAD~20
```

Exit non-zero if any scanned commit is missing a known agent `Co-Authored-By`
while the subject suggests automated work, or if `-RequireAll` is set.

---

## 8. Source notes (research)

| Source | Use |
|--------|-----|
| GitHub multi-author docs | Email must be account-linked for contributions/UI |
| Claude Code defaults / issues | `Claude <noreply@anthropic.com>`; optional model name in display |
| `openai/codex` `commit_attribution.rs` | Default `Codex <noreply@openai.com>` |
| openai/codex discussion #2807 | Avatar with `codex <codex@openai.com>` |
| GitHub API + public commit search | `grok@x.ai` widely used but unlinked; `grokkybara[bot]` is monorepo-only |
| Community CONTRIBUTING examples | PrimeFaces et al. — `Grok … <grok@x.ai>` |
| docs.x.ai/build | Grok Build product docs — no Co-Authored-By setting documented (2026-07) |
