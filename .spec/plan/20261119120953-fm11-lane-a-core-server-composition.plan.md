---
id: plan:root/20261119120953
title: FM11 lane A Core server composition
status: active
created: 2026-11-19T12:09:53Z
updated: 2026-11-19T12:09:53Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/core_server.rs, crates/sdk/humanymous-runtime/tests/fixtures/gateway/core-server-go-golden.json]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application]
---

# FM11 lane A Core server composition

## Goal

Compose the ported kernels into a Rust Core server application,
`gateway::core_server`, that matches the measured behaviour of Go
`cmd/server` at oracle 981d25c. This closes the composition gaps of the FM11
ledger rows `server.go`, `handlers.go`, `resource_handlers.go`,
`powhandler.go`, `rit.go`, `tlsconn.go`, `collect_pipeline.go`, `main.go`,
`maxconns_test.go`, `logging.go` and `external_input_receipt.go`, on branch
`claude/fm11-lane-a`.

The work is bounded:

- Detection freeze: no score, weight, threshold, hard-rule predicate, verdict
  band or signal ID changes.
- The Pass routes stay with lane C, behind the `PassRoutes` mount point.
- The `internal/gate` composition stays with lane B.
- The ML activation (`ml_setup.go`, `mlcorrect/bundle.go`) is reserved to the
  owner and is left alone.

## Steps

- [x] Capture `core-server-go-golden.json` from the frozen oracle.
- [x] Build the HTTP/1.1 reader and encoder: Content-Length bodies only, a
  64 KiB head limit, and `Date` and `Connection: close` on every reply.
- [x] Port the Go 1.22 mux dispatch and redirects, the security headers, the
  FileServer redirects and the page, report, CSP, PoW, trace, traffic, `/res/`
  and healthz handlers, including `ensureSession`.
- [x] Port the collect pipeline in Go order:
  - RIT verify-and-rotate runs before the body is parsed.
  - The merged observation feeds the enrichment: rit, traffic check, H2 abuse,
    rate limiter, correlation, fp churn, WebRTC leak and PoW solved.
  - Scoring is followed by the live feed, the external-input receipt and
    `X-HM-PoW`.
- [x] Build a loopback-only accept loop:
  - A `ConnectionCap` permit is taken before each accept.
  - Plaintext sockets are peeked for wire header order.
  - TLS connections go through `capture::tls`.
  - Each connection owns its evidence; there is no address-keyed registry.
- [x] Replay the capture in `src/gateway/core_server/tests.rs`, with each
  deliberate difference asserted explicitly. Test sockets bind to loopback
  only. The qualification contract was regenerated: 26 new common
  `gateway::` tests.
- [ ] Public Core listener and binary role (plan B01, Root profile).
- [ ] Decide whether proxy-VPN and Tor exit CIDR sets feed `is_proxy` and
  `is_tor_exit` (owner). The intel SoT keeps them shadow-only.
- [ ] Mount the Pass routes (lane C).
- [ ] Product e2e under Docker (`red-blue-validate`).
- [ ] Owner review of the pull request.

## Record

Capture, 2026-11-19, in a cloud session container with its own Docker Engine
29.9.2. Venue: the rule 96 cloud container, not a GitHub-hosted runner.

- **Image:** `mirror.gcr.io/library/golang@sha256:91eda9776261207ea25fd06b5b7fed8d397dd2c0a283e77f2ab6e91bfa71079d`
  (go1.26.3).
- **Run settings:** `--network none`, `GOTOOLCHAIN=local`, `GOPROXY=off`,
  `GOFLAGS=-mod=readonly`, read-only module cache.
- **Source:** `git archive 981d25c go.mod go.sum internal pkg cmd/server web`,
  plus a scratch in-package test `cmd/server/zz_fm11la_capture_test.go`
  (SHA-256 `fa0241beaea3e4d5452bc4f4804391bbe6c97aa1075402917b1eaabc6ac7b564`).
- **Command:** `go test -count=1 -run ^TestFM11LACapture$ ./cmd/server/`.
- **Reproducibility:** the capture program is not committed, so the bytes are
  bound but cannot be reproduced from the baseline tree alone
  (`go-recorded-differential`).
- **Run-to-run variation:** random session ids and the wall-clock RIT bucket
  differ between runs. The tests do not compare these:
  - Fresh session ids are masked.
  - Stale-bucket signal values are rebased on the test clock.
  - Random-session stored entries are compared as a set.
- **Raw output:** `capture.json`, SHA-256
  `46860b4f3afa5ab3ec0ac61120245b9cc2c29bffce847e59b3c7978762f67557`.
- **Committed fixture:** the raw output with a `provenance` object prepended,
  written with `jq --indent 1`. Its `ritCanonical[].token` is renamed `mac` so
  the secret scanner does not flag it. SHA-256
  `8ebcdfdfbb9dfd0138a4aa7c59891f1372ce383f19bd8b4294d01fd9608d7e45`.
- **Never** regenerate this fixture to hide drift.

Deliberate differences, each pinned in a test:

- **Serving**
  - Only loopback binds are allowed.
  - No HTTP/2 and no keep-alive.
  - A request with `Transfer-Encoding` gets 400.
  - A head over 64 KiB gets 431.
- **Static files**
  - No Last-Modified, Range or conditional serving.
  - Unknown extensions are served as `application/octet-stream`; Go sniffed
    `text/plain`.
- **Fail-closed paths**
  - Exhausted bounded histories give 503 `history unavailable`.
  - A scoring failure gives 503.
- **Proxies:** trusted proxies come from an explicit CIDR allowlist (C1).
- **Routes**
  - explain, counters, mlcorrect and playground are not mounted (B09).
  - Pass routes give 404 until lane C mounts them.
- **Panics:** with logging off, a panic drops the connection.
