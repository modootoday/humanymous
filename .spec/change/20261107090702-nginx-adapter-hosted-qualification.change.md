---
id: change:root/20261107090702
title: nginx adapter hosted qualification
status: active
created: 2026-11-07T09:07:02Z
updated: 2026-11-07T09:07:02Z
rel:
  depends_on: [sot:root/rust-migration]
---

# nginx adapter hosted qualification

## Change

Update the nginx integration documents and source-bound evidence after actual
hosted qualification of8132117e7bf1445939d7dee9066fcf45ccf0915b. Run42355168729,
job128358355202 passed35 scenario groups with zero failed/ignored/not_run,
cleanup0, source unchanged and immutable image identity. Evidence is
evidence:root/20261107052501, hosted-813-execution-summary.json and its canonical
Git blob proof. The earlier local14-unit/package-check/Clippy result remains a
separate denominator. This documentation change does not modify runtime code,
shared Cargo/lock/toolchain, detection policy or the frozen candidate.

The pinned successful image build enforces library tests and separate native,
pilot and package feature builds/Clippy. The published CI summary omits raw
per-case/build-stage reports and package manifest hash; those missing receipts
are recorded rather than reconstructed. Positive ALLOW remains an explicit
synthetic fixture through actual authority/WAL. Full product, sealed startup,
human/AT, sustained backpressure and future profiles remain unqualified.

## Migration

No API or data migration. Historical source-only NOT_RUN records are preserved;
the exact813 component receipt supersedes them only within its source scope.
Root retains integration and remote dispatch ownership. The owned watcher was
stopped after nginx success, without cancelling the remote workflow. Observe the
new benchmark quiet window; no additional local build/runtime commands are needed
for these evidence and documentation updates.
