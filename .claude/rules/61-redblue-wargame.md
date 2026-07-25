# Sequential red/blue wargame (agent series)

When the user runs a multi-round red/blue **wargame series** (Gate/Core/audit residual
hunt, N-round ping-pong, “red puts attack then blue guards”), follow this rule.
**Pass-only challenge rounds** still use skill `pass-wargame-round` + SoT-36.

Full loop: skill `red-blue-wargame-round`. Measurement-only: skill `red-blue-validate`.
Session lanes: skill `coordinate-sessions`.

## Mandatory shape (every round)

```text
RED (attack artifact) → MEASURE (Docker e2e for that plane) → LEDGER + SCRATCH → BLUE (guard)
```

1. **Red first as executable attack** — write or extend a harness artifact before Blue code.
2. **Measure with Docker** for the plane under test (rule `60`). Host runners are authoring aids only.
3. **Blue only from measured outcome** — `fix` | `hold` | `fp` | `defer`. Do not invent defenses for unmeasured claims.
4. **No mid-series commit theater** — default: **do not commit each round**. No bulk `--allow-empty`. Product formalization commits only after the series (or when the user explicitly allows), via `git-coord` + attribution (rules `91`–`92`).
5. **No parallel harness trees** — do not invent `test/wargame/` (or similar). Use existing:
   - Core catalog: `test/redteam/*.mjs` (+ `cmd/redteam` for raw uTLS)
   - Gate edge/admin: `test/gate/e2e.mjs` and/or `internal/gate` / `internal/audit` package tests
   - Pass: `test/redteam/pass_*.mjs` (claim `pass` lane)

## Plane honesty

| Plane | Red artifact | Docker proof |
|-------|--------------|--------------|
| Core L1–L7 | `test/redteam` profile (3 registries) | `make attack` + `make e2e-assert` |
| Gate edge / admin | `test/gate/e2e.mjs` check | `make gate-e2e` |
| Pass | `pass_e2e` / `pass_wargame` | compose `pass-test` profile |

Do not claim Gate admin is proven by a Core `/api/collect` profile, or L5 multi-subnet by host loopback alone.

## Core catalog registration (if Red adds a profile)

A new `test/redteam/<name>.mjs` is silent until registered in **all three**:

1. `test/e2e/runner.mjs` `PROFILES`
2. `cmd/server/launch.go` `launchProfiles`
3. `web/playground.html` Observatory catalog

Update assert expected profile count when the catalog size changes. Never tune a red profile to hide a Blue gap (`docs/how-to/write-a-red-profile.md`).

## Detection freeze & a11y

- Verdict-altering score/weight/hard-rule edits still need explicit user freeze-spend (rule `20`).
- Prefer freeze-safe guards (admin SoD, token bind honesty, audit empty-chain, edge policy).
- Never raise cost by motor-richness/speed gates on Pass/CHALLENGE (rule Pass a11y).

## Multi-session

- Claim the correct work lane before edits; do not write another agent’s paths.
- Prefer private series state under gitignored `plan/<series>/` + `SCRATCH/<series>/` (templates in the skill assets).
- Defense-only: own deployment / compose lab only (HARD-WON / red-attacker Forbidden).

## Anti-patterns

- 50× empty commits as “progress”
- Hard-reset of shared history to erase concurrent sessions
- Unit-only “e2e done” for catalog / gate / swarm planes
- Weakening red until ALLOW instead of fixing Blue
