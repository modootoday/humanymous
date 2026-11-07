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
- [ ] Implement owned MutationObserver with script-marker semantics and callback lifecycle.
- [ ] Implement explicit forwarding hooks, preserving return/throw/this/construction and documenting direct-eval change.
- [ ] Test partial setup, reentrancy, stop/drop, own-reference-only restoration and callback lifetime.
- [ ] Port bounded active CDP probes without claiming universal automation identity.
- [ ] Verify actual browser application/lifecycle tests and full collector bootstrap ordering.
- [ ] Integrate qualified report with snapshot/session/RIT and actual Docker product e2e.

Pure state does not install browser hooks or prove runtime observations. Uncovered
planes cannot emit a fully reported guard. Scoring freeze/a11y/T4 remain unchanged.
