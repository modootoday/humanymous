# Git commit attribution (mandatory)

Every **agent-authored** commit on this repository MUST include a stable
`Co-Authored-By` trailer identifying the LLM provider (Claude Code pattern).

## Required

1. Conventional Commits subject (`feat:`, `fix:`, `docs:`, …) — SoT-37 mapping.
2. Trailer from the **vendor / community registry**:

   | Provider | Trailer |
   |----------|---------|
   | claude | `Co-Authored-By: Claude <noreply@anthropic.com>` |
   | grok | `Co-Authored-By: Grok <grok@x.ai>` |
   | codex | `Co-Authored-By: codex <codex@openai.com>` |
   | gemini | `Co-Authored-By: Gemini CLI <gemini-cli@users.noreply.github.com>` (placeholder) |

3. Recommended: `X-Agent-Provider: claude|grok|codex|gemini`
4. Use `scripts/agents/git-coord.ps1 commit -Provider …` (or `git-commit.sh`) so trailers are not forgotten.
5. Do **not** use raw `git commit -m "…"` without the trailer when acting as an agent.
6. Do **not** invent unowned domains (`@agents.humanymous.local`) or third-party
   GitHub logins (`github.com/grok`, `github.com/xai`, `github.com/grokxai`).

## Grok / xAI note

- **Canonical trailer (community de-facto):** `Grok <grok@x.ai>`  
  (optional display: `Grok 4 <grok@x.ai>`).
- Official org: `github.com/xai-org`. Product: `xai-org/grok-build`.
- As of 2026-07, `grok@x.ai` is **not** linked to a GitHub user → **no co-author avatar**.
- Do not use monorepo bot `grokkybara[bot]` for end-user agent attribution.
- When xAI publishes a linked identity, update `COMMIT-CONVENTION.md` first.

## Full canon

`.agents/sessions/COMMIT-CONVENTION.md`

## Related

- `git-ops` mutex: `.agents/rules/91-git-contention.md`
- `scripts/agents/check-commit-attribution.ps1`
