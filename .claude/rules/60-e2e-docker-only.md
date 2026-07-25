# E2E is Docker-only (completion authority)

All end-to-end validation for detection and Gate **must** run through the Docker
compose stack under `deployments/`. Host/loopback runs are development aids only.

## Why

- Loopback (`127.0.0.1`) cannot exercise multi-subnet correlation or realistic L5 topology.
- Lab networks enforce defense-only (internal: true) — policy by construction.
- CI (`detector-vs-bots`) is the same path operators and agents must use.

## Authoritative commands

```bash
# Full suite (attack + assert + gate-e2e + swarm + overlays)
bash scripts/e2e-docker.sh
# or: make e2e

# Faster iteration (still Docker)
E2E_SKIP_SWARM=1 E2E_SKIP_OVERLAYS=1 bash scripts/e2e-docker.sh
# or: make e2e-quick

# Individual compose targets
make up && make attack && make e2e-assert
make gate-e2e
make swarm
```

Windows: `pwsh -File scripts/e2e-docker.ps1` (optional `-SkipSwarm` / `-SkipOverlays`).

## Forbidden as “done”

- `node test/e2e/runner.mjs` against a host-only `bin/server` as the sole gate
- `node test/gate/e2e.mjs` on the host as the sole Gate gate
- Claiming L5 / HR-network rules verified without Docker attack or swarm

## Allowed without Docker

- `go test ./...` unit tests
- WASM build/vet
- Authoring a red profile locally before packaging into the bots image
