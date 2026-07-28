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
21q. **Match the vendor gate to how widely the tell is shared** (R12, TLS certificate_compression
    absence, freeze-spend). compress_certificate (RFC 8879, ext 27) is sent by ALL modern browser
    engines (Chrome, Firefox, Safari) — unlike ALPS (R10), which is Chromium-only. So the gate is
    the BROADER `claimsBrowserUA` (Chrome/Firefox/Safari), not the Chromium-only `chrome/` token:
    a too-narrow gate would MISS a Firefox/Safari-UA impersonator that omits ext 27; a too-broad
    "any client" gate would FP the UA-less h2 health-checkers the Docker baseline actually
    contained. Keep the same precondition gate (h2-in-ALPN) and the same residual→HR-24 pattern
    (new class net.tls.certcomp, operator monitor override for a cert-compression-stripping
    middlebox). Measure: every real browser (Chrome/Firefox/Safari) carries ext 27, so human FP 0.
21p. **A multi-component fingerprint isn't closed until every component is checked; use a
    threshold with margin, not an exact value** (R11, HTTP/2 flow-control WINDOW_UPDATE,
    freeze-spend). The Akamai h2 fingerprint is SETTINGS|WINDOW_UPDATE|PRIORITY|pseudo-order;
    R6 closed SETTINGS and R7 pseudo-order, leaving a coherent-SETTINGS + Chrome-pseudo-order
    spoof that still shipped Go's 1 GiB connection window. Close the remaining component as its
    own residual under the SAME operator class (net.h2.spoof) — don't proliferate net-policy
    classes for one fingerprint family. FP-safety: browsers use bounded, slightly-varying windows
    (measured: Chrome 15663105, Firefox 12517377), so DON'T hardcode a browser value — use a
    floor (64 MiB) with a wide margin above the largest browser and far below the library value
    (1 GiB), and require the abusive direction only (>= floor; an absent/small window never
    fires). Gate on the browser CLASSIFICATION (isBrowserEngine) so only a client already
    mimicking a browser is judged. human FP 0 in the Docker gate, with a real Firefox in the
    baseline as extra headroom proof.
21o. **A vendor-specific "always-present" tell needs a vendor gate AND a precondition gate**
    (R10, ALPS application_settings absence, freeze-spend). ALPS is Chromium-only and is sent
    only when the ClientHello offers h2. Two independent FP traps: (a) Firefox/Safari legitimately
    never send it — so gate on the vendor (`chrome/` token), not "is a browser"; the Docker
    baseline literally contained a real Firefox with h2+no-ALPS that a naive check would have
    DENYed. (b) An http/1.1-only Chrome also omits it — so gate on the precondition (h2 in ALPN).
    Also make the presence check tolerant of a moving codepoint (ALPS migrated 17513→17613): match
    EITHER, or a codepoint bump silently turns the tell into a mass-FP. Measure the real vendor
    build (headless Chromium 149 sends 17613) to confirm before enforcing. HR-24 net.tls.alps,
    operator monitor override for a ClientHello-rewriting middlebox. Same residual→HR-24 pattern
    as R7/R8/R9; human FP 0 in the Docker gate is the proof.
21n. **Version-gate a "modern default is missing" tell, and MEASURE the default before shipping**
    (R9, post-quantum TLS enforcement, freeze-spend). Chrome shipped X25519MLKEM768 (0x11EC) on
    by default in M131 / Firefox 132; a scraper pinning an older TLS parrot claims a modern UA
    but omits the PQ group. The trap: flag it unconditionally and you FP every genuinely-older
    browser (which never sent PQ) and any UA that predates the default. The fix is a VERSION GATE
    (fire only when the UA claims >= the version that made it default) plus a real-baseline
    measurement — headless Chromium 149 sends 0x11EC, so the gated check cannot flag a real modern
    browser. Enforce via HR-24 net.tls.pq (operator monitor override for a PQ-stripping middlebox
    / TLS-inspecting proxy — the legitimate deployment-delta). Same score-exempt residual → HR-24
    NET-POLICY pattern as R7/R8; verdict-altering ⇒ SoT-37 policy event, `!`; human FP 0 in the
    Docker gate is the proof.
21m. **To enforce a structurally-unobservable signal, build the capture first, then MEASURE the
    FP baseline** (R8, R4 header-order enforcement). Go's net/http map destroys h1 wire order, so
    l5.header.order was dead — a raw h1 peek + replay (mirroring peekH2) makes it observable
    (OrderReliable). Do NOT guess the browser-order model: measure real headless Chromium (it
    always sends the sec-ch-ua cluster before user-agent, or omits it), then flag ONLY the
    measured inversion (user-agent before sec-ch-ua). Enforce via HR-24 net.header.order
    (operator-overridable). Human FP 0 in the Docker gate is the proof the capture didn't
    mis-order real browsers.
21l. **Enforce a detection residual via NET-POLICY, not a categorical block** (R7, user-authorized
    freeze-spend). To convert the R3/R6 h2 residuals to live CHALLENGE without an
    isDatacenterIP-class mass-FP, route them through HR-24 under a new operator-overridable
    class (net.h2.spoof, enforce default / monitor override). FP-safe because a DIRECT browser
    has a coherent h2; the only deployment-delta (an h2-reframing proxy) is what monitor is for.
    Verdict-altering ⇒ SoT-37 policy-version, `!` marker; MANDATORY Docker gate (human FP 0 is
    the decisive check — the real-Chromium baseline was byte-identical before/after).
21k. **A composite fingerprint checked on ONE component is spoofable on that component**
    (R6, web-researched h2 SETTINGS split). The 2026 h2 fingerprint = pseudo-order + SETTINGS +
    WINDOW_UPDATE, but `EngineFromH2` keyed the browsers on pseudo-order ALONE, so a raw framer
    sending Chrome's m,a,s,p with Go SETTINGS is misclassified Chrome and reaches ALLOW. Prove
    the classifier bug with a deterministic unit test; freeze-safe residual keys on a
    protocol-stable component the mimic omits (HEADER_TABLE_SIZE — all browsers send it, Go
    doesn't). Live h2 note: send SETTINGS + first HEADERS back-to-back (peekH2 captures the
    order before the server's SETTINGS; waiting deadlocks the fingerprint capture).
21j. **Verify WHICH layer actually defends — a comment can mis-attribute it** (R5, web-researched
    Pass token-reuse). The SoT-36 Pass anti-replay `traceDigest` claimed 1ms quantization was
    "coarser than any perturbation an attacker can hide below" — false: ≥1ms jitter is within
    human variance, so a captured placement-INDEPENDENT motor trace replays across brute-forced
    placements. The system still HOLDS, but via the per-solve COST axes (velocity + attestation
    + PoW), not the anti-replay. Fix the over-claim; do NOT add a motor/speed gate (rule 61 Pass
    a11y). Real anti-replay hardening (placement-bound proof) is a11y-sensitive → user decision.
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
