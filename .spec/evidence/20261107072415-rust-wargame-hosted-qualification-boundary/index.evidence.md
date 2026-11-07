---
id: evidence:root/20261107072415
title: Rust wargame hosted qualification boundary
status: active
created: 2026-11-07T07:24:15Z
updated: 2026-11-07T07:24:41Z
rel:
  governs: [crates/attacker]
  depends_on: [sot:root/rust-wargame]
files:
  - {path: summary.json, sha256: 19d261e96272b3eafa153703ae647823c563168f21f4fa318433f82fb5c562b5, bytes: 755, store: repo}
---

# Rust wargame hosted qualification boundary

## What was measured

Read-only GitHub API inspection of run42348518353, Docker job128331815962,
on2026-11-07 at approximately07:23UTC. The Docker job is terminal failure;
its final owning summary reports lab-runner exit2, component-only qualification,
full_product_qualified=false and outer errors0. The delivery command deliberately
exits1 with `component/blocked pilot cannot qualify full product`. This is not
evidence of a Rust compilation failure or an attacker regression.

The recorded source is63f69ae9de703939b7b9091e2f215b12bea265de. The log exposes
only hashes of the private owning report/source manifest; the private report was
not recovered by this inspection. Therefore actual network19/7 denominators,
strict-decoder execution and case results remain unverified from this public log.

Source: [Docker job log](https://github.com/modootoday/humanymous/actions/runs/42348518353/job/128331815962).
Inspection used `gh run view 42348518353 --json status,conclusion,headSha,jobs`
and `gh api repos/modootoday/humanymous/actions/jobs/128331815962/logs`.
The CLI required --allow-escape-sequences to read the log; no new workflow was
dispatched. The overall run was still in_progress and is not asserted terminal.

## How to read it

This public receipt preserves an observed qualification boundary, not a passed
product round or a substitute for private raw evidence. It does not qualify
strict decoder671c439 or later step-attribution75d7c82. The latter remains
pure-tests/Clippy/hosted not_run, with fmt/scoped spec/byte checks only. Human/AT,
accessible Pass, coherent T4 and remaining producer requirements stay open.
