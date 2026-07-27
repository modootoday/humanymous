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
8c. **CI resource budgets are set from Actions runs, never from a local build.** A local
   `bots:minimal` measurement produced a 2.5 GiB detector peak cap against a real peak of
   4.50 GiB, and a 48 MiB kernel-smoke cap that `qemu-system-x86_64` alone (77.8 MiB of
   layer) could never fit. Both shipped green because an earlier job failure meant the
   Docker jobs never ran. Corollaries: **echo what you measured before asserting it** (bare
   `test "$bytes" -le N` makes a red budget unattributable); and do not assert a lower
   bound on retained storage after `docker image prune --all`, which legitimately drops
   below the baseline the runner shipped with (measured `final_delta_bytes=-1784422400`).
8d. **Shrinking an image can silently retire red coverage.** `playwright-core install
   --no-shell` saved bytes and killed ten catalog profiles that launch Chromium's headless
   shell; the ladder still printed `100%` because skipped profiles left the denominator
   (see #8a). Size work on `bots` must be proven by a full catalog run, and profiles must
   not be re-pointed at a different binary to fit a budget — that changes the very
   fingerprint they exist to measure.
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
21z. **`sync-adapters.sh` and `sync-adapters.ps1` must emit byte-identical trees.** CI runs
    the **bash** one and `git diff --exit-code`s the result, so output committed from a
    Windows `pwsh` sync fails every push. `Set-Content` appends a terminator to a body that
    already ends in one (one stray newline per persona blocked CI for two days) and writes
    CRLF; generated files are written with `WriteAllText` + explicit LF so the result does
    not depend on the developer's `core.autocrlf`. Verify a generator change by running
    **both** and diffing, not just the one on your platform.

## Red/blue wargame process

21a. **Sequential wargame is red→measure→blue**, not commit theater. Put the attack in
    `test/redteam` or `test/gate/e2e.mjs`, prove with **Docker** for that plane, then guard.
    Default: **no per-round commits**; no bulk `--allow-empty`. Formalize after the series
    with `git-coord`. Rule `61` + skill `red-blue-wargame-round`.
21b. **Plane honesty:** Core catalog ≠ Gate edge/admin ≠ Pass. Host runner is not e2e done.
21c. **Never invent `test/wargame/`** — extend existing harnesses and the three Core registries.
21d. **Observed-but-forgotten tells launder** (wargame 2026-07-27, R1). A per-attempt bot tell
    that is DENY'd but not PERSISTED lets the attacker retry past the trigger and launder the
    DENY into a trust upgrade — `l7.pow.too_fast` was re-scored per submit, so a native solver
    revealed itself, got DENY, then resubmitted the same nonce after a delay for `pow.solved`.
    Persist any impossible-for-a-human observation to the session (`powTooFast`, never cleared).
21e. **Timing-based controls are fragile as catalog members** (R1). A control whose verdict
    depends on network round-trip vs a threshold (too_fast fires only when RTT < the browser
    floor) is non-deterministic across environments and leaks ALLOW ~20% on a slow path — keep
    such attacks as `cmd/redteam` probes + a deterministic handler test, not flaky must-block
    catalog entries (a member that sometimes ALLOWs breaks the assert intermittently).
21f. **A defense comment can over-claim the mechanism** (R2). "Timeouts bound
    connection-exhaustion" bounded per-connection DURATION, not CONCURRENCY — `Accept()` took
    unlimited concurrent slowloris connections. Read what the code enforces, not what it says;
    a concurrency cap (`netutil.LimitListener`) is the missing half.
21g. **An "unknown" classification under a KNOWN-only claim is a tell, not a pass** (R3,
    web-researched 2026 h2 protocol-split). `EngineFromH2` returned "unknown" for a Go h2
    frame layout and `engineConsistent` treated unknown as no-evidence → a Chrome-UA client
    with a library HTTP/2 fingerprint reached ALLOW. A real browser ALWAYS has a KNOWN h2
    fingerprint, so browser-UA + unknown-h2 is suspicious. Watch for stale classifier
    heuristics (the Go-detection check `hasMaxConc && m[4]==65535` never matched modern Go).
21h. **When the only fix is verdict-altering, split it: freeze-safe residual now, enforcement
    on freeze-spend** (R3). A weight-0 score-exempt signal (referenced by no hard rule)
    surfaces the tell in Audit/Console/NET-POLICY without moving the verdict — freeze-safe by
    construction (empty-overlay verdict unchanged; freeze golden passes). Verdict enforcement
    is a separate, user-authorized detection event (rule 20/61). Ask before spending freeze.
21i. **A weighted registry signal can be DEAD or STRUCTURALLY unobservable** (R4). `l5.header.order`
    (w20) was never emitted, and the data it needs (h1 wire header order) is destroyed upstream —
    Go's net/http map + `sort.Strings` in the adapter. A field doc claiming "wire order" while the
    adapter delivers a sorted set is a truth-debt landmine (a stored "order" hash is really a set
    hash; re-enabling an order check on it false-positives). Audit registered-vs-emitted signals;
    mirror `CasingReliable` with an `OrderReliable` flag; keep the id/description byte-identical
    when annotating (freeze). Same class as the JA4H dead-code removal (SoT-38 / rule 80).

## Git contention (multi-session)

22. **Git writes are a global mutex** (`git-ops` lane). Parallel work lanes may edit files;
    only one agent runs `add`/`commit`/`pull`/`push`/`stash`/branch switch at a time.
23. Hold `git-ops` only for the transaction (TTL ~30m). Never leave it claimed while coding.
24. If `.git/index.lock` exists, wait — do not start a second writer.
25. On non-fast-forward push: stop, fetch, rebase/merge under `git-ops`; never force `main`.
26. **Every agent commit must attribute the provider** via `Co-Authored-By` in the project
    registry (`COMMIT-CONVENTION.md` / rule `92`). Use `git-coord commit -Provider …`.
    Claude Code’s default trailer is not enough alone for multi-provider tracking — use
    the canonical vendor/community identity from the registry; never invent a local
    attribution domain.
