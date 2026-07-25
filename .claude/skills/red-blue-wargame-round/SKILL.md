---
name: red-blue-wargame-round
description: >
  Use for sequential multi-round red/blue wargame series: red writes attack
  in test/redteam or test/gate/e2e, Docker e2e measures, then blue guards.
  Trigger on red/blue wargame, 50-round series, residual gap hunt, attack
  then defend. Not Pass-only (use pass-wargame-round) or measure-only
  (use red-blue-validate).
when_to_use: >
  "red blue wargame", "50-round wargame", "sequential red/blue",
  "attack then blue guard", "residual gap hunt", "redteam then e2e then fix".
---

# Sequential red/blue wargame round

Always-on constraints: rule `61-redblue-wargame`.  
Docker authority: rule `60` + skill `red-blue-validate`.  
Lanes: skill `coordinate-sessions`.  
Pass-only challenge KPI: skill `pass-wargame-round` (SoT-36).

## Preconditions

1. `session-board list` → claim plane lane (`gate-edge`, `red-catalog`, `detection-core`, …).
2. Confirm series mode with user if ambiguous: **no commits during series** (default) vs commit-after-batch.
3. Private series tree (gitignored `plan/` is fine):

   ```text
   plan/<series-id>/PROTOCOL.md   # optional series notes; copy from assets/protocol.md
   plan/<series-id>/LEDGER.md
   plan/<series-id>/BACKLOG.md
   SCRATCH/<series-id>/rounds/rNN-evidence.md
   ```

4. Do not invent `test/wargame/`. Stay on existing harnesses.

## Round loop (one attack class)

### 1. RED — research + attack artifact

| Plane | Write here |
|-------|------------|
| Core L1–L7 | `test/redteam/<name>.mjs` (`run`, `label`=`bot:…`, `needsBrowser`); raw TLS/RIT → `cmd/redteam` + `_bin.mjs` wrap |
| Gate edge/admin | extend `test/gate/e2e.mjs` (and package tests under `internal/gate` / `internal/audit` as needed) |
| Pass | `test/redteam/pass_*.mjs` only with `pass` lane |

**Core profile registration (all three or silent skip):**

1. `test/e2e/runner.mjs` `PROFILES`
2. `cmd/server/launch.go` `launchProfiles`
3. `web/playground.html`

Authoring aids (not done): `node test/redteam/run-one.mjs …`, host gate e2e.

Code research: file:line. Web research when technique class needs grounding (note URL one-liner). Defense-only / own deployment.

### 2. MEASURE — Docker for that plane

| Plane | Command | Artifact |
|-------|---------|----------|
| Core | `make up` → `make attack` → `make e2e-assert` | `deployments/artifacts/core-results.json` |
| Gate | `make gate-e2e` | harness PASS/FAIL + `deployments/artifacts/gate-e2e.log` |
| Pass | compose `pass-e2e` / `pass-wargame` | service logs |
| After risky Blue | `make e2e-quick` or `make e2e` | `.agent-runs/e2e/*/status.json` |

If Docker is blocked: LEDGER `e2e-pending` + host note — **never** claim completion on host alone.

Copy cmd, exit code, verdict/check id into `SCRATCH/.../rNN-evidence.md` (template: `assets/evidence-template.md`).

### 3. BLUE — disposition from measure only

| Code | Meaning | Action |
|------|---------|--------|
| `fix` | FN / real gap / false green | Smallest freeze-safe product change + test that fails without it |
| `hold` | Already blocked (TP / gate PASS) | No product change; cite evidence |
| `fp` | Not a real bypass | Document; no defense churn |
| `defer` | Freeze / lane / Docker / SoT | Residual + owner; no silent drop |

Forbidden: tune red to ALLOW; freeze-spend without user OK; empty-commit theater.

### 4. LEDGER

Update series `LEDGER.md` (template: `assets/ledger-template.md`).  
**Default: no git commit for the round.**

## Series end (only when user allows)

1. Review all `fix` WIP as logical batches (not 50 empty commits).
2. `git-coord` preflight → claim → stage **product+test only** (not SCRATCH noise).
3. Conventional commit + provider `Co-Authored-By` (rule `92`).
4. Promote durable process lessons to `.agents/lessons/HARD-WON.md` if new.

## Relationship to other skills

| Skill | Role |
|-------|------|
| `red-blue-wargame-round` | Multi-round series loop (this file) |
| `red-blue-validate` | One-shot Docker verification / pre-merge gate |
| `pass-wargame-round` | Pass challenge cost/a11y round (SoT-36) |
| `review-changes` | Pre-merge freeze/security review of WIP |
| `handover-pack` | Pause series / provider switch |

## Assets

- `assets/protocol.md` — committed protocol digest (agent-readable)
- `assets/ledger-template.md`
- `assets/evidence-template.md`
- `assets/backlog-template.md`

## References

- `docs/how-to/write-a-red-profile.md`
- `docs/how-to/self-validation-red-team.md`
- `docs/explanation/red-catalog-architecture.md`
- `plan/04-e2e-redblue-plan.md`
- `deployments/README.md`, `scripts/e2e-docker.sh`, `scripts/assert-attack.mjs`
- `test/AGENTS.md`
