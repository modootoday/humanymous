# Hard-won lessons (always-on summary)

Before large detection, Gate, Pass, or honesty changes, read
`.agents/lessons/HARD-WON.md`.

**Top five (non-negotiable):**

1. Do not gate server hard-rules on client-forgeable flags.
2. Shared scoring ≠ same premises on Core vs Gate — assert per plane.
3. Docker-only e2e for completion (`make e2e`).
4. Pass: no multi-minute lockouts; no motor/speed gates.
5. No speculative over-engineering; verify claims against source (SoT-38 if present).
