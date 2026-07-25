# Hard-won lessons (cross-provider durable)

Extracted from multi-provider session history (Claude Code project memory + deep-review
sessions, Grok Build agent-standardization thread, Codex project rollouts, Gemini CLI
history). **These are constraints, not suggestions.** Full narratives stay in Claude
`~/.claude/projects/.../memory/` and `sots/38-truth-debt-remediation.md` (local/private).

## Detection & scoring

1. **Never gate a server hard-rule on a client-forgeable flag** (e.g. adBlock, self-reported
   “probed”). That *disarms* server-auth rules (HR-19 regression lesson).
2. **Shared scoring, different planes.** Core and Gate feed `internal/scoring` with
   *materially different* client reports. A rule premise that holds for Core/WASM may
   **DENY 100% of humans on Gate** if the Gate loader posts empty/hardcoded signals.
   Assert the premise **per plane** (SoT-38 B1).
3. **RIT completion ≠ real browser.** Anchoring human-proof rules only on RIT completion
   can regress TLS-static bots while still failing real humans.
4. **Detection freeze** unless explicitly requested; verdict-altering changes are MAJOR
   (SoT-37). Prefer SoT-38 freeze-spend process over casual weight edits.
5. **Human baseline is first-class.** Improving bot TPR by DENYing the baseline is a
   regression, not a win.

## E2E & topology

6. **Docker is e2e completion authority** (`make e2e` / `scripts/e2e-docker.sh`). Loopback
   cannot fire multi-subnet correlation; lab nets enforce defense-only.
7. **Gate e2e “human” fixtures must match the shipped loader.** Fabricating
   `mouse.samples: 40` when the Gate injects `samples: 0` makes CI vacuous-green (SoT-38).
8. **isDatacenterIP / IP intel stubs** can mass-CHALLENGE real humans off-loopback. Never
   ship demo stubs that fire on every non-loopback address without a documented policy.
8a. **A check that repairs or excludes before it verifies passes vacuously.** Two live
   instances: `assert-attack.mjs` drops `skipped` **and** `error` records from the denominator
   and downgrades errors to `WARN` (a real errored profile printed `PASS` on 2026-07-26, so
   `44/44` means "44 of the 44 that ran"); `ci.yml` runs `sync-adapters` *then*
   `verify-agents-layout`, auto-healing stale committed files. Fix shape: assert an expected
   floor (`bots.length >= 45`) and `git diff --exit-code` after any generate step.
8b. **The reference human must reach ALLOW, not CHALLENGE-scored-TN.** The canonical run
   reports `human FP 0` while the baseline sits at CHALLENGE — and with no enforcement path to
   `/pass`, that human is blocked with no recourse. A metric with no threshold is not a gate
   (see #5).

## Pass / a11y

9. **No multi-minute lockouts** in Pass wargame loops; keep `/new` immediate. Prefer
   engine-signal cost escalation over stateful blocks.
10. **Never gate Pass on motor richness or speed.** Accessibility is a ship-blocker.

## Honesty & docs

11. **Truth-debt is invisible to TODO scanners.** Deferred work lives in prose/SoT/docs.
    When changing claims, update derived docs and SoT-38 dispositions.
12. **Do not publish raw `sots/` to public git** by “helpfully” un-ignoring it. Canon is
    private; user docs are derived (SoT-29 / SoT-38 §9.6).
13. **No over-claim:** T4 ceiling is intentional; reference-measured ≠ guarantee.
14. **English-only** for shipped user docs and public README surface; keep terminology
    consistent (Core / Gate / Ledger — not legacy Sentinel/engine/Audit Console in new text).

## Engineering process

15. **No speculative over-engineering.** Minimum convention-correct change; confirm gaps
    against *code*, not imagination (user feedback → SoT-37 era).
16. **Multi-perspective + adversarial critique before large SoTs.** Critique findings must
    become requirements (“critique reflected”), not meeting notes (SoT-38 v1.0→v1.1).
17. **Census findings are hypotheses until source-verified.** Prefer re-open the file over
    trusting a prior agent’s claim.
18. **Active remediation plan:** when present, `sots/38-truth-debt-remediation.md` wins
    over older SoT *aspirations* for disposition of deferred/misdocumented items.

## Provider-specific

19. **Claude Code** alone persists rich project `memory/*.md`. Other providers do **not**
    see it — keep durable lessons here under `.agents/lessons/` and in AGENTS.md pointers.
20. **Gemini CLI** user `settings.json` may lack `context.fileName: AGENTS.md`; rely on
    **project** `.gemini/settings.json` (committed). Re-run `sync-adapters` after skill edits.
21. **Codex** may also load `~/.codex/AGENTS.md`; project root `AGENTS.md` must remain the
    sharper source of truth for this repo.

## Git contention (multi-session)

22. **Git writes are a global mutex** (`git-ops` lane). Parallel work lanes may edit files;
    only one agent runs `add`/`commit`/`pull`/`push`/`stash`/branch switch at a time.
23. Hold `git-ops` only for the transaction (TTL ~30m). Never leave it claimed while coding.
24. If `.git/index.lock` exists, wait — do not start a second writer.
25. On non-fast-forward push: stop, fetch, rebase/merge under `git-ops`; never force `main`.
26. **Every agent commit must attribute the provider** via `Co-Authored-By` in the project
    registry (`COMMIT-CONVENTION.md` / rule `92`). Use `git-coord commit -Provider …`.
    Claude Code’s default trailer is not enough alone for multi-provider tracking — use
    the stable `@agents.humanymous.local` identities (or keep both).
