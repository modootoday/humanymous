# Git commit attribution (mandatory)

Every **agent-authored** commit on this repository MUST include a stable
`Co-Authored-By` trailer whose **email is GitHub-linked** (avatar / contributor
card). Pattern matches Claude Code’s product default.

## Required

1. Conventional Commits subject (`feat:`, `fix:`, `docs:`, …) — SoT-37 mapping.
2. Trailer from the **vendor registry** (email is what GitHub uses for avatars):

   | Provider | Trailer |
   |----------|---------|
   | claude | `Co-Authored-By: Claude <noreply@anthropic.com>` |
   | grok | `Co-Authored-By: Grok Build <304785771+grokkybara[bot]@users.noreply.github.com>` |
   | codex | `Co-Authored-By: codex <codex@openai.com>` |
   | gemini | `Co-Authored-By: Gemini CLI <gemini-cli@users.noreply.github.com>` (no vendor avatar yet) |

3. Recommended: `X-Agent-Provider: claude|grok|codex|gemini`
4. Use `scripts/agents/git-coord.ps1 commit -Provider …` (or `git-commit.sh`) so trailers are not forgotten.
5. Do **not** use raw `git commit -m "…"` without the trailer when acting as an agent.
6. Do **not** invent unowned emails (`noreply@x.ai`, `@agents.humanymous.local`, or third-party `github.com/grok` / `github.com/xai` users).

## Grok / xAI note

- Official org: `github.com/xai-org` (SpaceXAI). Product repo: `xai-org/grok-build`.
- Only verified GitHub-linked automation identity for Grok Build publishing:
  **`grokkybara[bot]`** (`304785771+…@users.noreply.github.com`).
- Grok Build does not (as of 2026-07) ship a Claude-style default trailer; this
  repo still **requires** the line above via tooling.

## Full canon

`.agents/sessions/COMMIT-CONVENTION.md`

## Related

- `git-ops` mutex: `.agents/rules/91-git-contention.md`
- `scripts/agents/check-commit-attribution.ps1`
