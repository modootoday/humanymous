---
name: pass-wargame-round
description: >
  Use for one humanymous Pass challenge wargame round: raise automation cost
  without a11y regressions. Trigger on Pass wargame, SoT-36, or challenge hardening.
when_to_use: >
  "pass wargame", "challenge round", "SoT-36", "harden Pass", "wargame KPI".
---

# Pass wargame round

## Non-negotiables

- No motor-richness/speed gate; keyboard + SR path; crypto/attestation alternative.
- Pass solve must not launder hard-rule DENY bots.
- Prefer short self-clearing windows over multi-minute lockouts.

## Round loop

1. Name attack class / residual.
2. Smallest defense axis that raises cost without a11y breach.
3. Wargame strategy + unit tests.
4. Verify: `go test ./...` (unit). Any live engine / catalog / Gate path → **Docker** (`make e2e` or compose `make up` + attack), not host loopback alone.
5. CHANGELOG: blocked class, residual, human floor.

Canon: `sots/36-humanymous-pass.md`. Template: `assets/round-log.md`.
