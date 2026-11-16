---
id: evidence:root/20261116095026
title: Real Rust product e2e, first partial topology run (unit 35 increment 1)
status: active
created: 2026-11-16T09:50:26Z
updated: 2026-11-16T09:50:26Z
rel:
  depends_on: [plan:root/20261116060213, evidence:root/20261116062411, sot:root/container-test-environments]
files:
  - {path: receipt.json, sha256: 5cfb9e1fd1b469cd515a34b6ed620f32a6ad733e931aeadbcf1b5d24b9c47e95, bytes: 24033, store: repo}
---

# Real Rust product e2e, first partial topology run (unit 35 increment 1)

## What was measured

One run of `humanymous-lab-runner product-e2e --venue cloud-container` at
source commit `528b77ab2a4deb380236724e8845f8e89460646f` (branch
`claude/product-e2e`, clean tree). The runner built
`build/rust/product-e2e.Dockerfile` from source (152 s), started
`deployments/lab/compose.product-e2e.yaml` as project
`humanymous-lab-product-e2e`, ran the driver, inspected the containers and
network, and tore everything down. `receipt.json` is that receipt.

- **Venue**: a cloud session's own Linux container engine, endpoint
  `unix:///var/run/docker.sock`, Docker 29.9.2 client and server, engine ID
  `c7f5a196-202c-4d3d-b816-145fcff2acd0`, unchanged after teardown. It is not
  Windows `desktop-linux` and not a GitHub-hosted runner (rule 96).
- **Image**: `sha256:905f574fa8a9d33933cf7400b929d7d9dd64962999b0a5e5890cbc01f1438254`
  (132 MB). Built from `rust:1.99.1-slim-bookworm` and run on
  `debian:bookworm-slim`, both digest-pinned.
- **Topology**: three containers from that image, as follows.
  - `gate` runs the real `humanymous-runtime` binary. It gets private stdin
    configuration and synthetic sealed custody on tmpfs.
  - `origin` is a Rust recording origin.
  - `attacker` is the Rust driver.
  - All three are on one `internal: true` network with no published ports.
    They are read-only, run as uid 65532 and use `init`.
- **Driver**: 27 assertions, 27 passed. They cover:
  - refusal without a session (401);
  - forged trust headers without a session (401);
  - a route outside the inventory (409);
  - the reserved admin route (403);
  - a foreign Host (403);
  - CL+TE, duplicate CL, identical duplicate CL, a lone TE and obs-fold
    framings (400 each);
  - missing raw-TLS and collector capabilities (422 each);
  - a signed session, collect and effect forwarded exactly once, with the
    exact request observed at the origin;
  - replay and body mutation (403);
  - an effect from an uncollected session (403 CHALLENGE);
  - no Authorization, `x-humanymous-*` or forged trust header ever at the
    origin.
- **Origin delta**: every refused case left the origin record count unchanged.

The scoring outcomes are observed facts, not policy changes:

- Each signed request with exactly one of `X-Forwarded-For`, `Forwarded`,
  `X-Real-Ip`, `X-Hmny-Origin-Auth` or `X-Hmn-Verdict` was ALLOWed. The Gate
  stripped the header before the origin, so the strip path really ran.
- A lone `Cf-Connecting-Ip` produced CHALLENGE.
- All six headers together also produced CHALLENGE. That request never
  reached the origin.

## Sanitization

One string differs from the runner's original output. The build `--secret`
source path, a venue-local CA bundle location, reads `<venue-ca-bundle>`.
The original receipt's SHA-256 was
`a1fdfb3a4b85ad70b7ae5f1b747abf5fc4412a965f429dbd36a47bb585a76f02`. The
receipt has no session IDs, keys, tokens or the custody passphrase.

## How to read it

- `status` is `partial_topology_measured`, `pass_product_clean_room=false`,
  `product_complete=false` and `clean_room=false`.
- The gateway validates loopback-only listen and origin addresses and drops
  non-loopback peers. Origin and attacker therefore share the gate's network
  namespace. The driver observed the origin socket reachable from the attacker
  namespace, so origin isolation is absent. This violates the canonical L5
  boundary and is recorded as a gap.
- Authority, an independent audit service or witness, and the browser and
  collector are absent. Collect reports are client-asserted fixtures. Only the
  gate's embedded durable WAL ran.
- The scaffold product labs are unchanged; unit 36 retires them.
