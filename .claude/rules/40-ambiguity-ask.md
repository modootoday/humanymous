# Ambiguity: ask, don't assume

Research (AMBIG-SWE / agent instruction patterns) shows agents default to
**silent non-interactive** choices when requirements are ambiguous.

- If two interpretations would produce different architectures or public APIs, **ask**.
- If a change could break detection freeze vs. is "just refactor", confirm with the user.
- Prefer stating assumptions in one short bullet list when the user is unavailable, then stop before irreversible git/publish actions.
