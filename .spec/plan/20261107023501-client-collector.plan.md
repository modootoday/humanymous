---
id: plan:root/20261107023501
title: Rust passive behavior collection and lifecycle
status: active
created: 2026-11-07T02:35:01Z
updated: 2026-11-07T02:59:58Z
rel:
  depends_on: [sot:root/client-collector, sot:root/program-roadmap]
---

# Bounded L4 feature collection

- [x] Port pure pointer/key/cadence reductions and exact first600 caps.
- [x] Verify analytic boundaries, negative repeat-key dwell and privacy shape.
- [x] Implement passive owned Rust listeners, stop/drop and partial-start cleanup.
- [x] Execute unchanged JS reducers as read-only differential test oracles.
- [x] Run actual owned browser dispatch and cleanup/error tests in Docker.
- [x] Verify native/WASM/Clippy and register source-manifest evidence.
- [ ] Verify the final committed behavior source in remote component/native CI.
- [ ] Connect remaining environment/guard/advanced, distribution and session/RIT.
- [ ] Retire read-only JS oracle imports only after equivalent validated goldens
  and final foreign-source disposition; no legacy JS becomes product glue.

No scoring or a11y policy changes. This is event-to-summary collection, not a
human/automation attestation or full product e2e. The two referenced JS reducers
are test-only read-only migration oracles until their retirement gate is met;
other legacy implementations remain separate migration debt.
