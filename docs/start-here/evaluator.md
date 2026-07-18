# Start here: Evaluator

> **How-to / navigation hub.** For the engineering leader or security buyer deciding whether humanymous Sentinel fits your stack.

humanymous Sentinel is a reverse proxy that sits in front of your origin app, scores each request across seven detection layers (static client, fingerprint, client integrity, behavior, network/protocol, cross-check, and scoring), and enforces a verdict at the edge before the request reaches your app. It does not emit a binary "bot / not-bot" flag: it produces a graded risk score from 0 to 100 and maps that score to one of three verdicts — ALLOW (pass to origin), CHALLENGE (serve an accessible proof-of-work interstitial; origin never contacted), or DENY (block; origin never contacted). Hard rules can override the score for high-confidence automation signatures. The honest limitation to weigh first: Sentinel raises the cost of automated traffic and challenges or blocks it, but it does not solve the T4 tier — anti-detect toolchains paired with real-human click-farms. That is an explicit design boundary, mitigated only by rate limiting and reputation, not eliminated. Everything here is a reference implementation for evaluation, not a production-hardened build; treat measured latency and false-positive figures as reference-measured, and expect a prod-delta (ACME certificates, bring-your-own keys, and operational hardening) before you ship.

## Next 3 reads

Read these in order. Each builds on the last, and together they take you from "what is this" to "running it against my own traffic."

1. **[What Sentinel Is (and Is Not)](../explanation/what-sentinel-is.md)** — The scope, the design principles (defense-in-depth, cross-check-first, low false-positive, deterministic), and the T0–T4 attacker tiers, including where the T4 ceiling sits. Start here to calibrate expectations before you look at mechanics.

2. **[Will This Break My App?](../explanation/will-this-break-my-app.md)** — The false-positive and rollout story: how privacy browsers, extensions, and old devices are handled without being scored as bots, how the safe-GET fail-open works on balanced routes, and why you deploy monitor-first (score and log, enforce nothing) before you enforce anything.

3. **[Quickstart (monitor mode, 30 min)](../tutorials/quickstart-monitor-mode.md)** — Stand up Sentinel in front of a test origin, watch it score real requests in the Audit Console, and enforce nothing while you build confidence. This is the fastest way to see the graded score and the ALLOW/CHALLENGE/DENY model against your own traffic.

> **Tip:** If you are evaluating with a security team, run monitor mode against a mirror of production traffic first. You get the full risk score and verdict stream in the audit log with zero enforcement risk to live users.
