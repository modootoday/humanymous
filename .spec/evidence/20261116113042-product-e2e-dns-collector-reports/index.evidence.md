---
id: evidence:root/20261116113042
title: Real Rust product e2e with an owned DNS canary and real collector reports in every driver (unit 35 increment 4)
status: active
created: 2026-11-16T11:30:42Z
updated: 2026-11-16T11:30:42Z
rel:
  depends_on: [evidence:root/20261116110546, evidence:root/20261116103304, evidence:root/20261116095026, plan:root/20261116060213, sot:root/container-test-environments, sot:root/client-collector, sot:root/runtime-application]
files:
  - {path: receipt.json, sha256: ca6c7c53171bb8bed04e3956c39225948c242ad114c5e98a9304653578081bf4, bytes: 94001, store: repo}
---

# Real Rust product e2e with an owned DNS canary and real collector reports in every driver (unit 35 increment 4)

## What was measured

One run of `humanymous-lab-runner product-e2e --venue cloud-container` at
source commit `d4f90b04264b7bad20f90500b2be5b3018ef8b06` (branch
`claude/product-e2e-4`, clean tree). The runner built two targets of
`build/rust/product-e2e.Dockerfile` and ran three topologies from
`deployments/lab/compose.product-e2e.yaml` as project
`humanymous-lab-product-e2e`. It then tore everything down. `receipt.json` is
that receipt. Its status is `partial_topology_measured`.

This increment closes two gaps from the increment 3 bundle
([[evidence:root/20261116110546]]): there was no DNS canary, and the main and
witnessed drivers submitted client-asserted fixture reports.

- **Venue**: the cloud session's own Linux engine, endpoint
  `unix:///var/run/docker.sock`, Docker 29.9.2 client and server, engine ID
  `c7f5a196-202c-4d3d-b816-145fcff2acd0`. The ID was unchanged after
  teardown. This is not Windows `desktop-linux` and not a GitHub-hosted
  runner (rule 96).
- **Build egress**: as in increment 3. Builds used the host network, the
  venue's host-loopback HTTPS proxy as a build-only argument, and the venue CA
  bundle as a build-only secret. No lab container used the host network.
- **Base images**: the pinned `debian:bookworm-slim` and
  `rust:1.99.1-slim-bookworm` digests were pulled from `mirror.gcr.io` by
  digest and tagged locally.
- **Images**:
  - Gate, origin, driver, witness and DNS canary image:
    `sha256:cdb594c0539234373dc377c9cbbb5d577328ca541b43c91c48bcff40ec28d42e`
    (132 MB, `build_seconds` 107).
  - Browser image (`--target browser`):
    `sha256:0aaad83c55ef5a7708d988599b3bc2d27229208d4949cf15dcc21eab5f0ab2a5`
    (839 MB, `browser_build_seconds` 32). Both builds reused layers from the
    exploratory run below.
- **Order**: the browser topology now runs first. Its captured collector
  report is the one every later collected session submits.

## DNS canary

The `browser` profile gains two services:

- **`dnscanary`** runs `--product-dns-canary`. It is a real UDP :53 listener
  in its own container, on its own internal network `dnscanary`. The browser
  is not on that network. It answers each well-formed query with NXDOMAIN. It
  logs one line per arrival: the source address, byte count and the SHA-256
  of the question. It never logs the name.
- **`dnsprobe`** runs `--product-dns-probe` from the canary's network. It is
  the positive control, and it ran once before and once after the browser.

The canary's address was given to the browser actor as its outside resolver
control, replacing the increment 3 unrouted address.

| Assertion | Result |
|---|---|
| Canary announced `listening` | true |
| Both positive controls answered (rcode 3) | true |
| Arrivals are exactly the two controls (2 of 2, both from the probe address) | true |
| Zero arrivals from the browser network `172.18.0.0/16` | true |
| Canary is on a network the browser is not on | true |
| Browser lookup failed at its local deny resolver | true |
| Browser UDP route to the canary refused (`NetworkUnreachable`, os error 101) | true |

The browser's local deny resolver received 21 datagrams: 2 well-formed owned
queries and 19 that were not owned `.invalid` questions. It forwarded none.

What this adds over increment 3: zero arrival is now observed at a live
resolver that answered queries from its own network before and after the
browser ran. It is still a routing observation. The canary sits on a separate
network, so it does not test a resolver that the browser could reach at L3.

## Real collector reports in the main, origin-loss and witnessed drivers

The browser Gate frontend proxy now keeps the exact collect entity that the
real collector posted to `/_humanymous/browser/collect`. That collector is
`BrowserCollector::snapshot_json`, built for wasm32 with feature
`runtime-pilot`, running in Firefox ESR. The runner passes that entity to each
driver as one stdin line. The driver checks the following before it uses the
report:

- the entity's SHA-256 and length;
- an `engineVersion` that starts with `rust-wasm-`;
- an empty `sessionId`.

On each collected session, the driver sets only `sessionId`, because the
native collect route binds the report by that field. There is no fixture
fallback. Without a captured report, every collected session fails.

The captured report was 2139 bytes, SHA-256
`7a5c81e5159cc0e20785c1496576949c2e801fd2dd96855d54427fc495e0db6d`.

- **Signals**: `l1.navigator.webdriver` BOT, `l1.window.outer_eq_inner`
  SUSPICIOUS and `l2.screen.metrics` SUSPICIOUS, all collected `wasm`.
- **Guard**: `guard.reported` is false.
- **User agent**: Firefox 153 on Linux.

Expectations were not changed. Every verdict below is an observation of the
frozen engine:

- **Main topology**: 27 of 27 assertions passed. Every collect returned 200
  COLLECTED (11 of 11). Every case's status and origin delta equal increment 3
  exactly. Signed effects from sessions that submitted this report were
  forwarded with 200, even though it carries a webdriver BOT signal. Combined
  `signed_forged_trust_headers` and `signed_forged_cf_connecting_ip` stayed
  CHALLENGE, as before.
- **Exact-request check**: `origin_observed_exact_request` is now always
  recorded. It fails, rather than disappearing, when nothing was forwarded.
  In this run it was exercised and passed.
- **Origin loss after readiness**: passed. The collect returned 200
  COLLECTED, and the signed effect got the same 5xx as before.
- **Witnessed topology**: passed with outcomes identical to increment 3.
  Both collects returned 200 COLLECTED. 3 counter-signatures were relayed, and
  1 `unavailable` was injected after the loss. `wgate` went from `healthy` to
  `unhealthy` after the first failed append.
- **Browser topology**: passed as in increment 3. The audit, proxy request
  counts, Origin and Referer, witness coverage and clean drain all match.

The drivers' HTTP request headers are unchanged and still synthetic
(Chrome-style `User-Agent` and `sec-ch-ua`). They do not match the replayed
Firefox report. That mismatch is part of what the engine scored, and it was
not adjusted.

## What is still absent, and why

- **Authority**: absent. The Gate decides in-process.
- **Shipped-binary witness and live readiness**: as in increment 3.
- **Browser path provenance**: the browser reaches the Gate through owned lab
  proxies, not original client TLS/H2.
- **DNS reachability**: the canary is on a separate network. A resolver that
  the browser namespace could reach at L3 but must not use was not tested.
- **Report replay**: the replayed report is real collector output. It is
  replayed into other sessions, though, and is not produced in them:
  - the main, origin-loss and witnessed drivers are not browsers;
  - their request headers are synthetic;
  - `sessionId` is rebound.
  It is one captured report, not a qualified producer.
- **Not qualified here**: human or assistive-technology support, browser
  sandbox completeness, collector provenance qualification, witness
  independence, power loss and clean room.
- **Origin isolation**: the main topology still shares the Gate namespace.

## Exploratory attempts before this one

- One invocation refused to start because its `--emit` path in a shared
  scratch directory already held an unrelated older receipt. There was no
  Docker activity, and no file was overwritten.
- One complete run at `bcdcea9` reached `partial_topology_measured`. Its
  receipt SHA-256 was
  `dc7a218c0ae2db45d15d5f5792ba3f4699feafade6b2bf51e21364986fa2fe27`, and
  all its other outcomes matched this run. However, it read the browser
  container's address after the container had exited, and got an empty
  string. Its `dns_canary_zero_arrivals_from_browser` check therefore passed
  vacuously. Commit `d4f90b0` judges arrivals against the browser network's
  subnet and fails when the subnet is unknown. That run is not this evidence.

## Sanitization

Two strings differ from the runner's original output:

- the build `--secret` CA source path now reads `<venue-ca-bundle>`;
- the build-only proxy URL now reads `<venue-proxy>`.

A final newline was also appended. The original receipt's SHA-256 was
`f713c1e86eb04d7ddd6859204a3bb03a9c6af8d193ae785a8bdc21eaa5105bc8`.

The receipt holds only the digest and length of the captured report, not its
bytes. It has no session IDs, keys, tokens, passphrases, witness frames or
checkpoint contents. Canary arrival lines carry lab-internal addresses and
question digests only.

## Cleanup

- Compose down for all profiles; zero project containers left.
- Both built images removed by the runner.
- After the run, the base tags, mirror images and build cache were removed
  from the engine. The engine was left with zero images, containers, volumes
  and build cache.

## How to read it

- `status` is `partial_topology_measured`, with
  `pass_product_clean_room=false`, `product_complete=false` and
  `clean_room=false`.
- `browser.dns_canary.pass=true` means a live canary saw no browser query.
- `collect_report_source.real_collector_report_captured=true` means the
  drivers submitted collector bytes, not fixtures.
- Neither result qualifies the full product.
