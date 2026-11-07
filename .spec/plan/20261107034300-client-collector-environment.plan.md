---
id: plan:root/20261107034300
title: Rust first-party environment collection
status: active
created: 2026-11-07T03:43:00Z
updated: 2026-11-07T03:56:02Z
rel:
  depends_on: [sot:root/client-collector]
---

# Environment probe

- [x] Compare original env.js, shared Environment wire contract and frozen scorer.
- [x] Specify owned bait/timers/abort, weak failure interpretation and same-origin limits.
- [x] Implement explicit Rust asynchronous API with generated WASM glue only.
- [x] Verify native13, WASM release and warnings-denied WASM Clippy locally.
- [x] Execute actual Docker Firefox17tests for bait/privacy/fetch/timeout/cleanup, including actual exported API and owned-origin HTTP.
- [x] Register final scoped source/image/test manifests and private execution logs.
- [x] Verify f6d2893 exact-source remote component17; preserve downloaded image/log receipts.
- [ ] Verify f6d2893 exact-source native matrix separately; no component result replaces it.
- [ ] Integrate measured environment with remaining collector/session transport.

Scoring is unchanged. The separate environment API is not a qualified collector,
human/AT baseline, server attestation or full product Docker e2e. Advanced/guard,
bootstrap/distribution, recovery and Pass are required remaining product work.
