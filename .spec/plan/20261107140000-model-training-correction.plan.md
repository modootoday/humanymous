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

Subsequent root allocations qualified the corrected first slice at exact source
a0ebc9ae4da4c52020f6e39bb614d1e14d4e0b2e: fmt,13unit+1fixture, all-target check
and strict Clippy passed, including an actual owned-file parser test. First-source
68898522 fmt failure remains historical. This supersedes the execution-pending
statement only for that parser slice; kernel/lifecycle remain unallocated.

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

Next lowest L1 sub-slice authorized by root: legacy Standardize only, source and
fmt/spec work. standardize.rs plus training/mod.rs export and4module regression
tests are prepared; all4tests are NOT_RUN. This does not implement Train or resolve
RNG/holdout/lifecycle choices. Next allocation needs scoped fmt first, model units,
check/strict Clippy on an immutable source; no slot has been retained or requested
by source authorship itself. The prior parser14-pass receipt remains exact-source.

Owner6 later qualified e7d8f9a3 with17library+1fixture passed, including4new
standardization tests; fmt/check/strict Clippy passed. No target remains reserved.
This supersedes NOT_RUN for Standardize only, not Train/RNG/evaluation or whole
legacy rows230/231. Next lowest kernel candidates remain evaluation primitives
with explicitly separate gate policy, or RNG after sequence compatibility decision.

Root next authorized source-only Evaluate reduction: evaluate.rs uses existing
Prediction values, exact class counts and float32 rates with absent denominators
unavailable. Four new tests are NOT_RUN; prior18-pass receipt is not inherited.
RNG targets exact Go single-stream compatibility; source/fixture sequence freeze
is required before RNG implementation. No new workload has been started.

Each slice needs Rust 1.99.1/edition2024, scoped tests/fmt/Clippy, allocated actual
pilot and source-bound receipts. No GPU, model download, training, inference,
Cargo or Docker execution is authorized by this plan. No docs/** changes.
