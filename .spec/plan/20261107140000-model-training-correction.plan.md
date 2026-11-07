---
id: plan:root/20261107140000
title: Model training correction migration slices
status: draft
created: 2026-11-07T14:00:00Z
updated: 2026-11-07T14:00:00Z
rel:
  depends_on: [sot:root/model-training-correction]
---

# Model training correction migration slices

Root authorized only the source record slice: training/record.rs, training/mod.rs,
minimal lib.rs export and module tests. This source is prepared; all execution,
kernel and subsequent lifecycle slices remain unallocated. No manifest changes.

1. Accept contracts and independent critique: module placement, RNG parity strategy,
   bounds, required cohorts, temporal ties, weak-label quarantine and lab lifecycle.
2. Preserve immutable baseline and existing inference fixture provenance. Link each
   legacy row/test to existing Rust evidence or an explicit unimplemented gate.
3. Implement shared record/kernel/evaluation and bounded file reader; unit gates
   include warm start, training intermediates, splits, cohorts and absent support.
4. Implement opt-in bounded sink and correction sensors with no production mutation.
5. Implement exact-byte stage/shadow/lab lifecycle and canary/controller snapshots.
6. Implement thin bootstrap/correction binaries after root manifest changes; actual
   file/process pilots must prove both accepted and rejected candidates.
7. Root/runtime/product-lab owners coordinate Docker integration, source closure and
   final row dispositions. Preserve every legacy responsibility until reviewed.

Each slice needs Rust 1.99.1/edition2024, scoped tests/fmt/Clippy, allocated actual
pilot and source-bound receipts. No GPU, model download, training, inference,
Cargo or Docker execution is authorized by this plan. No docs/** changes.
