---
id: evidence:root/20261116103304
title: Real Rust product e2e with an independent audit witness, health checks and dependency loss (unit 35 increment 2)
status: active
created: 2026-11-16T10:33:04Z
updated: 2026-11-16T10:33:04Z
rel:
  depends_on: [evidence:root/20261116095026, plan:root/20261116060213, sot:root/container-test-environments]
files:
  - {path: receipt.json, sha256: 61349d96c7a9229a9e8fe04594edb80cae2eb6d02f11ac01471c5f0de3e33073, bytes: 42893, store: repo}
---

# Real Rust product e2e with an independent audit witness, health checks and dependency loss (unit 35 increment 2)

## What was measured

One run of `humanymous-lab-runner product-e2e --venue cloud-container` at
source commit `1809142a129aa07f05a189f0390d3dfdf48d0770` (branch
`claude/product-e2e-2`, clean tree). The runner built
`build/rust/product-e2e.Dockerfile` from source (97 s), ran two topologies
from `deployments/lab/compose.product-e2e.yaml` as project
`humanymous-lab-product-e2e`, and tore everything down. `receipt.json` is that
receipt. Its status is `partial_topology_measured`.

- **Venue**: the cloud session's own Linux engine, endpoint
  `unix:///var/run/docker.sock`, Docker 29.9.2 client and server, engine ID
  `c7f5a196-202c-4d3d-b816-145fcff2acd0`. The ID was unchanged after
  teardown. This is not Windows `desktop-linux` and not a GitHub-hosted runner
  (rule 96).
- **Image**: `sha256:f80f4df4419940dc383ca180f1ad130c9786fe864c6476f89602da4405ee3964`
  (132 MB), from the same digest-pinned `rust:1.99.1-slim-bookworm` and
  `debian:bookworm-slim` bases as increment 1. Docker Hub first answered the
  `debian` metadata request with 429. Both base images were then pulled by
  their pinned digests before the build.
- **Cleanup**: compose down, both attacker run containers removed, zero
  project containers left, the built image removed.

### Main topology: shipped gateway, health checks, origin loss

The `gate` container runs the shipped `humanymous-runtime` binary, as in
increment 1. The `origin` and `attacker` containers share its namespace.

- **Health checks**: `gate` and `origin` use exec-form probes
  (`CMD /usr/local/bin/humanymous-runtime-pilot --product-health <role>`).
  No shell runs. The gate probe is a TCP connect to the gate listener. The
  origin probe connects only to the origin's observation port, because every
  application-port contact is an origin record.
- **Dependency edges**: `origin` depends on `gate: service_healthy`, and
  `attacker` depends on both `gate` and `origin: service_healthy`. The engine
  recorded these edges in the compose labels. The runner started `gate` and
  `origin` with `up --wait`, and ran the attacker without `--no-deps`, so
  compose enforced the edges. Both were `healthy` before the driver ran.
- **Driver**: the same 27 assertions as increment 1, 27 passed. The scoring
  outcomes were the same as in increment 1.
- **Origin loss after readiness**: the runner stopped `origin` (state
  `exited`). A second driver confirmed that the origin and observation ports
  were unreachable. It then sent a signed, collected effect. The Gate answered
  `503 {"error":"origin_connect"}`. The `gate` container stayed `running` and
  its TCP health stayed `healthy`.

### Witnessed topology: independent audit witness and witness loss

The `witnessed` compose profile adds four containers.

- **`witness`**: runs `--product-witness`. It provisions its own sealed
  identity and head state on its own tmpfs, with a passphrase that never
  leaves the process. It then serves
  `humanymous_runtime::audit::checkpoint::service::serve`. It runs as UID
  65533 with network `none` and no volumes.
- **`wgate`**: runs `--product-gate-witnessed`. This is the runtime library
  `ServiceHost::bind_with_witness` with `require_external_witness=true`. It
  uses a `WitnessServiceClient` pinned to the witness public key. The
  container has its own internal network `humanymous-lab-product-e2e_witnessed`
  and no volumes.
- **`worigin`** and **`wdriver`**: share the `wgate` namespace.

The runner is the trusted broker. It attaches to the private stdio of
`witness` and `wgate` and exchanges the writer and witness public identities.
It then relays the typed frames. It records only frame kinds, never frame
contents.

- **Readiness and health**: `wgate` readiness reported
  `audit_external_witness=true`, `audit_witness_required_durability=file_and_directory`
  and `audit_witness_independence_qualified=false`. Both `witness` (ready
  marker written after the audited Ready frame) and `wgate` were `healthy`.
  `worigin` came up through `up --wait`, gated on `wgate: service_healthy`.
- **Witnessed effect**: an unsigned request got 401, and a signed, collected
  effect got 200 with exactly one origin record. The broker relayed one
  `ready` and 3 `signed` witness replies, so the Gate's checkpoints were
  counter-signed by the separate container.
- **Witness loss after readiness**: the driver prepared a pre-signed effect
  from a collected session, then paused. The runner stopped `witness` (state
  `exited`, exit code 143). The broker then observed the witness stream close
  and answered the next counter-sign request with the typed `unavailable`
  refusal. That was one injected refusal, and no signature was fabricated.
  - The pre-signed effect got `503 {"error":"durable_audit"}` and created no
    origin record.
  - Session issue got 503 (observation only).
  - An unsigned request got `503 {"error":"session_epoch"}` and created no
    origin record: the failed append poisoned the session epoch.
  - `wgate` stayed `running`, and its TCP health stayed `healthy`.
- **Driver**: 5 cases, 5 passed. All seven witnessed topology assertions
  passed.

## Exploratory runs before this one

Two earlier runs used the uncommitted tree. Their receipts are not kept.

1. The first stopped at the build with the Docker Hub 429.
2. The second passed everything except one case. That case had guessed that
   an unsigned request after witness loss would still get 401, but the Gate
   answered `503 session_epoch`.

That case now records the status as observed. Like the pre-signed case, it
asserts the fail-closed property: no 2xx and no origin effect. No other
assertion changed.

## What is still absent, and why

- **Authority**: absent. The Gate cannot use a remote authority. The gateway
  builds `Application::new` with its own engine
  (`crates/sdk/humanymous-runtime/src/gateway/service.rs:287`). It decides
  in-process (`crates/sdk/humanymous-runtime/src/gateway/application.rs:474`)
  and has no authority client. `humanymous-authority` serves adapter
  decisions only over a restricted native IPC listener
  (`src/authority/transport/mod.rs:60`, no TCP fallback).
- **Shipped-binary witness**: the witnessed Gate is the runtime library
  composed by a lab supervisor. The shipped `humanymous-runtime` binary calls
  only `ServiceHost::bind`
  (`crates/sdk/humanymous-runtime/src/gateway/main.rs:16`). On Unix the CLI
  host passes no witness (`crates/cli/src/runtime_host/platform.rs:49`).
- **Witness qualification**: not qualified.
  - The identity is first-contact pinned through the broker.
  - The head pin is not persisted outside the witness.
  - tmpfs gives no power-loss durability.
  - The broker is the runner process.
- **Health probes**: they are liveness only, so neither probe observed the
  witness loss. Requests failed closed while health stayed `healthy`.
- **Clean room**: still blocked. `Configuration::validate` requires loopback
  listen and origin (`gateway/configuration.rs:127-128`), and the accept
  loop drops non-loopback peers (`gateway/application.rs:352`, `374`). Origin
  and attacker therefore still share the Gate namespace. The driver again
  observed that the origin socket is reachable from the attacker namespace.
  This increment did not relax either check.
- **Browser and collector**: absent. Collect reports are client-asserted
  fixtures.

## Sanitization

One string differs from the runner's original output. The build `--secret`
source path, a venue-local CA bundle location, reads `<venue-ca-bundle>`. A
final newline was also appended. The original receipt's SHA-256 was
`71aa7a23fe3271b128a8bb8ec3ae5b533af28828b3829ec454564e91d9b33a83`. The
receipt has no session IDs, keys, tokens, passphrases, witness frames or
public keys.

## How to read it

- `status` is `partial_topology_measured`, with
  `pass_product_clean_room=false`, `product_complete=false` and
  `clean_room=false`.
- `witnessed.pass=true` means the separate witness container counter-signed
  live Gate checkpoints, and that the Gate failed closed after it stopped. It
  does not qualify witness independence.
