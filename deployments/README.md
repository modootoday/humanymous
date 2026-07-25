# Local Docker environment — the detector vs the bots

A complete humanymous stack in containers, defined as **modular compose fragments**.
The detection stack (Core engine + reverse-proxy Gate + a demo origin) runs long and
is viewable from the host; the bots (the automation catalog) run on demand and are
**network-sandboxed**.

Running everything in containers — rather than all on one loopback host, as the
project originally did — is what finally exercises the **network layer** of
detection: a real container IP is flagged `l5.ip.datacenter_asn`, and one
fingerprint appearing across three real subnets raises
`l5.correlation.proxy_rotation`. Neither can fire on 127.0.0.1.

## Layout (golang-standards)

```
build/                      # Dockerfiles (+ per-file .dockerignore overrides)
  core.Dockerfile           #   cmd/server  (also the public demo image)
  gate.Dockerfile       #   cmd/gate
  bots.Dockerfile           #   Playwright + Go attack binaries
configs/dev.env             # dev-only env (admin tokens, targets) — env_file'd
deployments/
  compose.yaml              # top-level; `include:`s the fragments
  compose/networks.yaml     #   web + lab-a/b/c (SRP: topology)
  compose/defenders.yaml    #   core + origin + Gate
  compose/bots.yaml         #   attack / conformance / swarm
  k8s/                      #   Kubernetes manifests for the published Gate image
  systemd/                  #   systemd unit to run the Gate binary on a host
  origin/                   # nginx shop the Gate protects
  bots/                     # bots orchestration scripts
  artifacts/                # run outputs land here
```

## Topology

```
  host :8443 (core)   host :8444/:8445 (gate)
        │                         │
  ┌─── web (bridge, host-visible) ────────────────────────┐
  │  core   gate ──proxies──▶ origin                 │
  └────────────────────────────────────────────────────────┘
  ┌─ lab-a 172.30 ────┐ ┌─ lab-b 172.31 ────┐ ┌─ lab-c 172.32 ────┐
  │ (internal: no      │ │ internet route)   │ │                   │
  │  bots / bot-swarm-a│ │  bot-swarm-b      │ │  bot-swarm-c      │
  └──────────▲─────────┘ └────────▲──────────┘ └────────▲──────────┘
             └── all attack core / gate, which join every subnet
```

**Safety by construction:** the `lab-*` networks are `internal: true` (no route
off-box) and the bots containers attach to *only* a lab network — so the automation
can physically reach nothing but the detector. The mandate ("local target only") is
enforced by the network, not by convention.

## Quick start

```bash
# from repo root — Make targets wrap compose (where `make` is available):
make up             # build + start the detection stack (core :8443/demo, gate :8444, admin :8445)
make attack         # run the bots (automation catalog) vs the engine (47 profiles)
make e2e-assert     # assert last attack artifact (all bots blocked, no FP)
make swarm          # multi-subnet correlation swarm (proxy_rotation across 3 subnets)
make gate-e2e       # Gate proxy-layer conformance (34 checks)
make e2e            # full Docker e2e suite (attack + assert + gate-e2e + swarm + overlays)
make e2e-quick      # Docker e2e without swarm/overlays (faster iteration)
make down           # tear everything down

# or the one-shot script (same authority as CI):
bash scripts/e2e-docker.sh

# or drive compose directly from deployments/ (cross-platform; no make needed):
docker compose up -d --build core origin gate
docker compose run --rm bots
docker compose --profile swarm up --abort-on-container-exit bot-swarm-a bot-swarm-b bot-swarm-c
docker compose run --rm gate-e2e
docker compose down -v
```

> **E2E policy:** completion authority for end-to-end detection/Gate validation is **Docker only**. Host `node test/e2e/runner.mjs` against a loopback binary is for profile authoring, not for claiming the suite passed.

The authoritative Linux runner prints every Docker command directly, marks every
phase `START`/`PASS`/`FAIL`, emits a heartbeat every 30 seconds, and applies a hard
timeout to each phase. Output is also mirrored to
`.agent-runs/e2e/<run-id>/e2e.log`; the machine-readable current phase is in the
adjacent `status.json`.

Follow the newest run from another terminal without waiting for its command to
return:

```bash
sh scripts/e2e-watch.sh
```

Use `sh scripts/e2e-watch.sh --once` (or PowerShell `-Once`) for a non-blocking
status snapshot.

On a Docker Desktop workstation, `pwsh -File scripts/e2e-watch.ps1` follows the
same log. `scripts/e2e-docker.ps1` is a troubleshooting convenience only; it is
not completion authority and intentionally does not claim the Linux overlay gate.
GitHub Actions uploads `.agent-runs/e2e/` on failure as well as success.

Admin console: `https://localhost:8445/__hmn/admin/console` — dev token
`operator:e2e-operator-token` (from `configs/dev.env`). Certs are self-signed;
accept the browser warning.

## Published images (pull-only production path)

The `compose.yaml` above is the **local-build lab**: it builds images from source
(`build/*.Dockerfile`) and wires the Core engine, Gate, a demo origin, and the bots
together for Red/Blue work. To run Gate in front of your *own* app you do not need to
build — two images are published to GitHub Container Registry (Apache-2.0, multi-arch
`linux/amd64` + `linux/arm64`, cosign-signed):

| Image | What it is |
|---|---|
| `ghcr.io/modootoday/humanymous-gate:latest` | The reverse-proxy enforcement layer — the product. Put it in front of your origin. |
| `ghcr.io/modootoday/humanymous-core:latest` | The standalone detection engine, for self-testing and the demo. |

`:latest` tracks the newest release.

`compose.release.yaml` is the **pull-only** counterpart to the lab file: it references
`ghcr.io/modootoday/humanymous-gate:${HMN_VERSION:-latest}` with **no `build:` and no
dev tokens**, and runs Gate hardened (distroless, non-root, read-only rootfs, dropped
capabilities) with ACME TLS on `:443`, a sealed keystore, and a durable, replay-verified
audit WAL. Adopt it in three steps:

```bash
cp .env.example .env            # HMN_UPSTREAM, HMN_DOMAIN, HMN_UNSEAL, HMN_ADMIN_TOKENS
cp routes.conf.example routes.conf
docker compose -f compose.release.yaml up -d
```

Gate joins your existing origin network and proxies to your app. The admin listener stays
host-loopback (`127.0.0.1:8445`) — keep it there or front it with mTLS/SSO. Liveness and
readiness are HTTP probes on the edge (`GET /__hmn/healthz`, `/__hmn/readyz`); the
distroless image has no shell, so there is **no Docker `HEALTHCHECK`** — point your
orchestrator's `httpGet` probes at those paths. For a quick local demo of the published
Gate image without the full production config, see
[Run from the published image](../docs/reference/install-requirements.md#run-from-the-published-image-no-build).

Beyond Compose, two more ways to run the same published Gate image/binary ship
here: **[`k8s/`](k8s/)** — `kubectl apply`-able Deployment/Service/PVC/Secret/
ConfigMap manifests (hardened, httpGet probes, graceful drain, admin plane kept
cluster-internal) — and **[`systemd/`](systemd/)** — a `humanymous-gate.service`
unit for running the `gate` binary directly on a host. Both mirror the flags and
hardening of `compose.release.yaml`.

## Verified result

- **Attack run**: all 45 bot profiles blocked (DENY/CHALLENGE) out of the
  47-profile catalog (45 bots + 1 detection-ceiling + 1 baseline); the baseline
  (a Playwright/CDP session, not a physical human) is not denied. Reference-
  measured on the maintainers' hardware, n=1 — not a guarantee.
- **Swarm**: `l5.ip.datacenter_asn` on first contact, then
  `l5.correlation.proxy_rotation` once the shared fingerprint spans the three
  subnets (risk 59 → 75.4, DENY).
- **Gate conformance**: 34/34.

## Notes

- **Headful profiles** (human, nodriver, xvfb_headful, ai_agent, anti-detect) run
  under `xvfb-run`; headless profiles ignore it.
- **The Detection Observatory is loopback-only by design** and fail-closes on any
  non-loopback bind, so it is not containerized. Run it locally:
  `HMN_PLAYGROUND=1 make run` → https://localhost:8443/playground.
- **Windows**: put `C:\Program Files\Docker\Docker\resources\bin` on `PATH`, or
  `docker build` can't find `docker-credential-desktop`.
