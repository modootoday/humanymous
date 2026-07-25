---
name: red-blue-validate
description: >
  Use when finishing detection/Gate work or before merge: run the Docker-only
  e2e suite (attack catalog, assert, gate-e2e, swarm). Prefer for full gate,
  make e2e, make attack, red-blue, or detector-vs-bots verification.
---

# Red/Blue validate (Docker-only e2e)

## Policy

**All e2e completion authority is Docker compose.** Host/loopback catalog or
`node test/gate/e2e.mjs` alone is **not** done. See `.agents/rules/60-e2e-docker-only.md`.

## Preconditions

- Docker Engine / Desktop available
- Defense-only (lab networks are internal by construction)

## Preferred one-shot

```bash
# Full suite matching CI intent
bash scripts/e2e-docker.sh
# or
make e2e
```

Windows:

```powershell
pwsh -File scripts/e2e-docker.ps1
```

Faster local iteration (still Docker; skips swarm + overlays):

```bash
make e2e-quick
# E2E_SKIP_SWARM=1 E2E_SKIP_OVERLAYS=1 bash scripts/e2e-docker.sh
```

## Checklist (if running steps manually)

Progress:

- [ ] Unit: `go test ./...` (and WASM build if client path touched)
- [ ] `docker compose -f deployments/compose.yaml config -q`
- [ ] `make up` — core + origin + gate
- [ ] `make attack` — bots catalog → `deployments/artifacts/core-results.json`
- [ ] `make e2e-assert` — `node scripts/assert-attack.mjs …`
- [ ] `make gate-e2e` — proxy conformance (34)
- [ ] `make swarm` — assert `l5.correlation.proxy_rotation` (unless out of scope)
- [ ] Optional: `bash scripts/ci-overlays.sh` for PLAN-08 overlays

## Report

- Pass/fail per Docker gate
- Human baseline not DENY
- No guarantee language; reference-measured only

## Gotchas

- Stale host `bin/server` on :8443 can confuse operators — use compose stack.
- Swarm log must contain `l5.correlation.proxy_rotation`.
- Do not mark network-plane work complete on loopback-only runner output.
