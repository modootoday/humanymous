---
name: pass-wargame-round
description: >
  Use for one humanymous Pass challenge (SoT-36) wargame round: raise
  automation cost without a11y regressions. Trigger on Pass wargame, SoT-36,
  or Pass challenge hardening. Not multi-round Core/Gate residual series
  (use red-blue-wargame-round) or Docker-only measure (use red-blue-validate).
when_to_use: >
  "pass wargame", "SoT-36", "harden Pass", "Pass challenge round", "pass KPI".
---

# Pass wargame round

**Scope:** Pass challenge plane only (`internal/pass`, `test/redteam/pass_*`).  
For sequential Core/Gate red→e2e→blue series see skill `red-blue-wargame-round` + rule `61`.

## Non-negotiables

- No motor-richness/speed gate; keyboard + SR path; crypto/attestation alternative.
- Pass solve must not launder hard-rule DENY bots.
- Prefer short self-clearing windows over multi-minute lockouts.

## Round loop

1. Name attack class / residual.
2. Smallest defense axis that raises cost without a11y breach.
3. Wargame strategy + unit tests.
4. Verify: `go test ./...` (unit). Any live engine / Pass path → **Docker** (compose `pass-e2e` / `pass-wargame` or full `make e2e`), not host loopback alone.
5. CHANGELOG: blocked class, residual, human floor.

Canon: `sots/36-humanymous-pass.md`. Template: `assets/round-log.md`.
