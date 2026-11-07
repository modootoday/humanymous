---
id: plan:root/20261107020501
title: Rust browser collector frozen L1-L3 slice
status: active
created: 2026-11-07T02:05:01Z
updated: 2026-11-07T02:05:01Z
rel:
  depends_on: [sot:root/client-collector, sot:root/program-roadmap]
---

# Execution and residuals

- [x] Separate pure observations/evaluator from checked wasm32 browser I/O.
- [x] Reuse shared report/registry; preserve17frozen IDs/predicates/notes/verdicts.
- [x] Cover benign/absence/zero/case/Unicode/memory/fresh-realm boundaries.
- [x] Compile native and wasm32, run warnings-denied Clippy and workspace tests.
- [x] Run actual Firefox WASM component tests in non-root network-none Docker.
- [x] Prove inserted iframe removal when a Rust-defined getter throws.
- [ ] Verify exact committed source in remote native and collector component CI.
- [ ] Port active CDP producers, L4/environment/guard/advanced and generated loader.
- [ ] Bind collector/RIT transport and recovery to actual runtime sessions.
- [ ] Validate ordinary-browser/human/AT controls and full product Docker e2e.

Native6 and actual-browser3 tests pass in the recorded source snapshot. Whole
workspace tests, native/WASM warnings-denied Clippy pass. These prove this slice,
not all browser detection parity or human ALLOW. No scoring policy changed.
Docker headless Firefox is not the Meari+CEF product engine or ordinary baseline;
no complete sandbox attestation is asserted. BrowserReady/full_runtime_qualified
remain false. See crates/collector/INTEGRATION.md and source-bound evidence.
