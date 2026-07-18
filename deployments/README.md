# Local Docker environment — the detector vs the bots

A complete humanymous stack in containers, defined as **modular compose fragments**.
The detection stack (engine + reverse-proxy Sentinel + a demo origin) runs long and
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
  engine.Dockerfile         #   cmd/server  (also the public demo image)
  sentinel.Dockerfile       #   cmd/sentinel
  bots.Dockerfile           #   Playwright + Go attack binaries
configs/dev.env             # dev-only env (admin tokens, targets) — env_file'd
deployments/
  compose.yaml              # top-level; `include:`s the fragments
  compose/networks.yaml     #   web + lab-a/b/c (SRP: topology)
  compose/defenders.yaml    #   engine + origin + Sentinel
  compose/bots.yaml         #   attack / conformance / swarm
  origin/                   # nginx shop the Sentinel protects
  bots/                     # bots orchestration scripts
  artifacts/                # run outputs land here
```

## Topology

```
  host :8443 (engine)   host :8444/:8445 (sentinel)
        │                         │
  ┌─── web (bridge, host-visible) ────────────────────────┐
  │  engine   sentinel ──proxies──▶ origin                 │
  └────────────────────────────────────────────────────────┘
  ┌─ lab-a 172.30 ────┐ ┌─ lab-b 172.31 ────┐ ┌─ lab-c 172.32 ────┐
  │ (internal: no      │ │ internet route)   │ │                   │
  │  bots / bot-swarm-a│ │  bot-swarm-b      │ │  bot-swarm-c      │
  └──────────▲─────────┘ └────────▲──────────┘ └────────▲──────────┘
             └── all attack engine / sentinel, which join every subnet
```

**Safety by construction:** the `lab-*` networks are `internal: true` (no route
off-box) and the bots containers attach to *only* a lab network — so the automation
can physically reach nothing but the detector. The mandate ("local target only") is
enforced by the network, not by convention.

## Quick start

```bash
# from repo root — Make targets wrap compose (where `make` is available):
make up             # build + start the detection stack (engine :8443/demo, sentinel :8444, admin :8445)
make attack         # run the bots (automation catalog) vs the engine (26 profiles)
make swarm          # multi-subnet correlation swarm (proxy_rotation across 3 subnets)
make sentinel-e2e   # Sentinel proxy-layer conformance (34 checks)
make down           # tear everything down

# or drive compose directly from deployments/ (cross-platform; no make needed):
docker compose up -d --build engine origin sentinel
docker compose run --rm bots
docker compose --profile swarm up --abort-on-container-exit bot-swarm-a bot-swarm-b bot-swarm-c
docker compose run --rm sentinel-e2e
docker compose down -v
```

Admin console: `https://localhost:8445/__hmn/admin/console` — dev token
`operator:e2e-operator-token` (from `configs/dev.env`). Certs are self-signed;
accept the browser warning.

## Verified result

- **Attack run**: 25/25 bots blocked (TPR 100%), 0 false positives; the human
  baseline resolves to CHALLENGE (counted TN — a real user passes the check).
- **Swarm**: `l5.ip.datacenter_asn` on first contact, then
  `l5.correlation.proxy_rotation` once the shared fingerprint spans the three
  subnets (risk 59 → 75.4, DENY).
- **Sentinel conformance**: 34/34.

## Notes

- **Headful profiles** (human, nodriver, xvfb_headful, ai_agent, anti-detect) run
  under `xvfb-run`; headless profiles ignore it.
- **The Detection Observatory is loopback-only by design** and fail-closes on any
  non-loopback bind, so it is not containerized. Run it locally:
  `HMN_PLAYGROUND=1 make run` → https://localhost:8443/playground.
- **Windows**: put `C:\Program Files\Docker\Docker\resources\bin` on `PATH`, or
  `docker build` can't find `docker-credential-desktop`.
