---
id: evidence:root/20261116110546
title: Real Rust product e2e with a real browser running the Rust WASM collector, and witness-loss readiness (unit 35 increment 3)
status: active
created: 2026-11-16T11:05:46Z
updated: 2026-11-16T11:05:46Z
rel:
  depends_on: [evidence:root/20261116103304, evidence:root/20261116095026, plan:root/20261116060213, sot:root/container-test-environments, sot:root/client-collector, sot:root/runtime-application]
files:
  - {path: receipt.json, sha256: fc9cfa261ed506be7e9444895ac3d413c7e33de08e5fd93ae026a2c57b3fb5a2, bytes: 77479, store: repo}
---

# Real Rust product e2e with a real browser running the Rust WASM collector, and witness-loss readiness (unit 35 increment 3)

## What was measured

One run of `humanymous-lab-runner product-e2e --venue cloud-container` at
source commit `a7354c8047235c08c4f98f95bc3e38b56b7f1dee` (branch
`claude/product-e2e-3`, clean tree). The runner built two targets of
`build/rust/product-e2e.Dockerfile` and ran three topologies from
`deployments/lab/compose.product-e2e.yaml` as project
`humanymous-lab-product-e2e`. It then tore everything down. `receipt.json` is
that receipt. Its status is `partial_topology_measured`.

- **Venue**: the cloud session's own Linux engine, endpoint
  `unix:///var/run/docker.sock`, Docker 29.9.2 client and server, engine ID
  `c7f5a196-202c-4d3d-b816-145fcff2acd0`. The ID was unchanged after
  teardown. This is not Windows `desktop-linux` and not a GitHub-hosted
  runner (rule 96).
- **Build egress**: build containers in this venue had no DNS. The builds used
  the host network and the venue's host-loopback HTTPS proxy as a build-only
  argument. They also used the venue CA bundle as a build-only secret. Debian
  sources were switched to https for the apt steps. No lab container used the
  host network.
- **Base images**: Docker Hub answered 429. The pinned `debian:bookworm-slim`
  and `rust:1.99.1-slim-bookworm` digests were pulled from `mirror.gcr.io` by
  digest and tagged locally. The builds resolved them by those digests.
- **Images**:
  - Gate, origin, driver and witness image:
    `sha256:75ad5d0aa717c9e6d3df81b90d708fbc0941a46a5d347466f3a2b213ec4893bc`
    (132 MB).
  - Browser image (`--target browser`):
    `sha256:97e293bbd272e512674b3c4dd8c6274ff68fea49b87685c04ec63c0e28d7a28d`
    (839 MB).
  - Both builds were cache hits (`build_seconds` 1 and 0). They reused layers
    from builds of the same file content shortly before the commit.
- **Cleanup**: compose down for all profiles, all run containers removed, zero
  project containers left, and both built images removed.

### Main topology and origin loss (increment 1 and 2 assertions)

The main topology ran the same 27 driver assertions as before, and 27
passed. Every case's status and verdict matched increment 2 exactly. The
`origin_socket_reachable_from_attacker_namespace` observation was again true.
The health, dependency-edge and topology assertions all passed. Origin loss
after readiness passed again, and the shipped-binary `gate` stayed `healthy`
with TCP liveness only.

### Witnessed topology with a readiness probe

The runtime now exposes `ServiceHost::epoch_health()`, a read-only view of
the existing session-epoch poison latch. It turns false, permanently for the
process, once an audit append fails or the clock rolls back. The witnessed
lab Gate mirrors it into a private tmpfs marker. The `wgate` exec probe
(`--product-health gate-witnessed`) requires that marker and a TCP connect.

- Before the driver, both `witness` and `wgate` were `healthy`. `worigin`
  came up through `up --wait`.
- The witnessed driver passed its 5 cases with outcomes identical to
  increment 2. The broker relayed 3 `signed` replies and injected one typed
  `unavailable` after the loss. It fabricated no signature.
- The runner then stopped `witness` (exit 143). Before any further request,
  `wgate` health was still `healthy`, because the runtime learns of the loss
  only on its next append.
- After the driver's pre-signed effect got `503 durable_audit`, the latch
  failed. `wgate` became `unhealthy` while staying `running`. The check
  `wgate_readiness_withdrawn_after_failed_append` is true.

### Browser topology: real browser and real Rust WASM collector

The `browser` profile runs three containers on its own internal network.

- **`bwitness`**: runs `--product-witness`, with UID 65533 and network
  `none`.
- **`bgate`**: runs `--browser-gate`. This is the runtime library
  `ServiceHost::bind_with_witness` with a required external witness, plus an
  owned lab frontend proxy on `:8080`. The gateway itself stays loopback-only.
  Writer keys and the WAL are on tmpfs owned by UID 65532 with mode 0700.
- **`browser`**: runs `--browser-driver` in its own network namespace. The
  actor drives Firefox ESR through geckodriver under the Rust
  `wasm-bindgen-test-runner` 0.2.130. The test runner loads the
  `humanymous-collector` WASM test build, built with feature `runtime-pilot`
  for `wasm32-unknown-unknown`.

Readiness and hardening:

- `bgate` readiness reported `audit_external_witness=true` and
  `audit_witness_required_durability=file_and_directory`.
- Its hardening reported UID 65532, effective capabilities 0,
  `no_new_privs`, a read-only root and no Docker socket.

Test result:

- The actor's test inventory matched: 67 tests, exactly one selected,
  `browser::transport::runtime_pilot::actual_runtime_session_report_and_required_capability_fail_closed`.
- The runner printed `test result: ok. 1 passed; 0 failed; 0 ignored;
  66 filtered out` with exit 0.

Wire observations: both the browser-local proxy and the Gate frontend proxy
counted the same requests, as follows.

| Request | Count | Response |
|---|---|---|
| Session | 1 | 200, no ACK |
| Collect | 1 | 200, ACK |
| `/effect` | 2 | 403, ACK each |
| `/requires-collector` | 1 | 422, no ACK |

- The Origin header matched the owned origin on all 5 API requests, with zero
  absent or `null` Origins.
- The Referer header was the exact owned origin-root on all 5.
- This is the first actual execution of the reviewed origin-root referrer
  source. It did not reproduce the earlier
  `browser_bootstrap_origin_browser_proxy_null` observation in this Firefox
  ESR.

Gate audit and drain:

- The Gate's authenticated offline audit was CHALLENGE, CHALLENGE, then
  COLLECTED (sequence 3), which equals the fixture expectations.
- There were zero origin effects.
- The audit was fully witnessed, with no ambiguous tail. The last checkpoint
  equals the witness head pinned by the broker, and no checkpoint repair was
  performed.
- 3 witness counter-signatures were relayed.
- The Gate drained cleanly after `--browser-stop` with the latest pinned head.

Namespace controls: all 10 browser controls held.

- The runtime and backend loopback and bridge ports were unreachable.
- Metadata egress was refused.
- There was no default route.
- The deny-only local DNS was observed, and the outside UDP route was refused.
- The foreign-authority and unknown-API proxy requests were refused.

Verdicts are observations of the frozen engine, not tuned outcomes. The
pre-report CHALLENGE is the fixture's expected fail-closed result.

## What is still absent, and why

- **Authority**: absent. The Gate decides in-process
  (`crates/sdk/humanymous-runtime/src/gateway/service.rs:287`,
  `gateway/application.rs:474`).
- **Shipped-binary witness and readiness**: neither the witnessed Gate nor the
  browser Gate is the shipped binary.
  - `gateway/main.rs:16` calls `ServiceHost::bind` only.
  - `gateway/main.rs:21` writes readiness once and gives its supervisor no
    live latch. The shipped-binary `gate` probe therefore stays liveness only.
- **Readiness timing**: readiness is withdrawn only after a failed append, not
  when the witness stops. The latch is not the typed observer and readiness
  API pending in `sot:root/runtime-application`.
- **Browser path provenance**: the browser reaches the Gate through owned lab
  proxies. That path is not original client TLS/H2 or raw-header provenance.
- **DNS canary**: no canary container ran. The browser was given an unrouted
  private address, so only UDP route denial was observed, not zero arrival at
  an outside resolver.
- **Collect reports in other topologies**: in the main and witnessed
  topologies, collect reports are still client-asserted fixtures.
- **Not qualified here**: human or assistive-technology support, browser
  sandbox completeness, collector provenance qualification, witness
  independence, power loss and clean room.
- **Origin isolation**: the main topology still shares the Gate namespace
  (`gateway/configuration.rs:127-128`, `gateway/application.rs:352`, `374`).

## Exploratory attempts before this one

- An earlier browser-image build failed because the collector WASM test build
  imports three read-only legacy oracle files that the dockerignore excluded.
  They are now included.
- One runner invocation refused to start because its `--emit` path already
  held an older receipt. No receipt was produced, and the earlier file was not
  overwritten.

## Sanitization

Two strings differ from the runner's original output:

- the build `--secret` CA source path now reads `<venue-ca-bundle>`;
- the build-only proxy URL now reads `<venue-proxy>`.

A final newline was also appended. The original receipt's SHA-256 was
`265a5fc2eb342b19f1e5007f0f2cdeb047ac69b61295cee76ae5d5b80f892c4f`. The
receipt has no session IDs, keys, tokens, passphrases, witness frames or
checkpoint contents.

## How to read it

- `status` is `partial_topology_measured`, with
  `pass_product_clean_room=false`, `product_complete=false` and
  `clean_room=false`.
- `browser.pass=true` and `components.browser.present=true` mean that the
  real browser and the real Rust WASM collector test ran and passed against a
  witnessed Gate. They do not qualify the full product.
