---
id: plan:root/20261107031001
title: Custody guard lifetime and Unix validation repair
status: active
created: 2026-11-07T03:10:01Z
updated: 2026-11-07T03:30:26Z
rel:
  depends_on: [sot:root/audit-key-custody, sot:root/program-roadmap]
---

# Reproduce before repair

- [x] Reproduce guard-drop lock retention with a duplicate description in a real
  non-root network-none Linux Docker unit test; retain the failure.
- [x] Repair ownership lifetime once for custody and WAL without unlinking/retry/permission relaxation;remove fs4.
- [x] Prove successor exclusivity while the stale duplicate closes and preserve inode identity.
- [x] Fix Unix authority Clippy without suppressing the negative peer test.
- [x] Run audit/custody/authority and actual runtime process pilot regressions.
- [x] Register final scoped source/image manifests, before-failure and after-success receipts.
- [ ] Run exact-commit remote native validation; keep older cb5Linux failure intact.

The CI symptom is WouldBlock after reacquisition. Inheritance during concurrent
spawn remains an explanatory hypothesis until controlled descriptor retention
is reproduced; no unrelated crypto/identity policy or scoring change is allowed.
