---
id: plan:root/20261107035500
title: Rust advanced observation lifecycle and parity
status: active
created: 2026-11-07T03:55:00Z
updated: 2026-11-07T03:56:02Z
rel:
  depends_on: [sot:root/client-collector, sot:root/program-roadmap]
---

# Full advanced plane

- [x] Compare original advanced.js and shared Advanced wire type; distinguish
  collectAdvanced from fingerprint-ID and ICE producers.
- [x] Reuse one timer owner with environment; distinguish fulfilled undefined,
  rejection and deadline while always cancelling owned timer.
- [x] Validate shared deadline and environment regression in actual isolated Firefox18;
  native13/release WASM/native Clippy pass. Register scoped source evidence.
- [ ] Port bounded device counts/kinds without labels/identifiers or permission prompts.
- [ ] Port voice count with owned listener,300ms fallback and cleanup without replacing global handlers.
- [ ] Port EME/GPU800ms optional fallback, preserving weak capability interpretation.
- [ ] Port WebGL/audio close, connection/battery, locale/display and memory semantics.
- [ ] Preserve original sentinel defaults and malformed/oversized observation handling.
- [ ] Exercise every surface/default/rejection/timeout and cleanup via actual browser tests;
  compare fixed vectors against unchanged legacy oracle where applicable.
- [ ] Verify exported API, release WASM, native/WASM Clippy and exact source CI.
- [ ] Integrate qualified advanced data with snapshot/session transport.
- [ ] Complete distinct fingerprint-ID and ICE/session responsibilities with privacy review.

No new score, threshold or DRM hard rule. Permission denial and missing APIs are
optional observations, not automation proof. Component tests never authorize
full Gate/browser/human/AT e2e or final retirement of original responsibilities.
