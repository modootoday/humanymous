---
id: plan:root/20261107041500
title: Rust guard coverage and owned instrumentation
status: active
created: 2026-11-07T04:15:00Z
updated: 2026-11-07T04:56:14Z
rel:
  depends_on: [sot:root/client-collector, sot:root/program-roadmap]
---

# Guard integration

- [x] Compare original injector.js and shared Guard; document direct-eval compatibility finding.
- [x] Implement pure bounded counter/coverage/prototype/native lifecycle; native17 includes4guard cases with32native masks.
- [x] Implement actual baseline capture in BrowserCollector, exact legacy native marker,
  read-time prototype counts and frozen partial guard on stop; native19/browser34 pass.
- [x] Implement owned MutationObserver with exact direct-node/marker semantics;
  native19 and actualDockerFirefox40 pass, including pending drain, stop/drop,
  malformed/oversized input, reentrancy and failed-disconnect inert callback.
- [ ] Implement explicit forwarding hooks, preserving return/throw/this/construction and documenting direct-eval change.
- [ ] Test partial setup, reentrancy, stop/drop, own-reference-only restoration and callback lifetime.
- [ ] Port bounded active CDP probes without claiming universal automation identity.
- [ ] Verify actual browser application/lifecycle tests and full collector bootstrap ordering.
- [ ] Integrate qualified report with snapshot/session/RIT and actual Docker product e2e.

Pure state does not install browser hooks or prove runtime observations. Uncovered
planes cannot emit a fully reported guard. Scoring freeze/a11y/T4 remain unchanged.

Observer browser evidence uses Windows-compiled target-independent WASM mounted
read-only into the existing pinned Linux Firefox image, with networknone,
read-onlyroot, UID65532,capdropALL,no-new-privileges,pids512. It is not a freshly
built source image or product e2e/full sandbox attestation. An earlier build was
blocked by concurrent lock changes and accidentally reran the prior36-test
artifact; its raw log is preserved as prior36-stale-artifact and not current
source validation. The final source built successfully and executed40/0/0.
