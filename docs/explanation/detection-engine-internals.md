---
layout: doc
title: Detection computation during migration
permalink: /explanation/detection-engine-internals.html
reviewed: 2026-11-07
---
# Detection computation during migration

**Diátaxis quadrant:** Explanation. **Audience:** implementation reviewers.

The migration preserves scoring behavior rather than introducing new detection
policy. Equivalent arithmetic alone is insufficient: signal provenance,
ordering, missing-input handling, exact boundaries and traces also need parity.

Invalid or non-finite input must produce an error that callers handle safely.
Required missing evidence cannot be converted into a convenient allow. Adapters
consume the shared decision contract instead of calculating a second score.

Partial Rust tests do not establish full-engine equivalence. Compare the
complete oracle corpus and deployment outcomes before making that claim. Even
an equivalent engine retains the coherent-browser ceiling. See
[decision limits](../reference/hard-rules-verdicts.md).
