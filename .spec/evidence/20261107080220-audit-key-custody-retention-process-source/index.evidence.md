---
id: evidence:root/20261107080220
title: Audit key custody retention process source
status: active
created: 2026-11-07T08:02:20Z
updated: 2026-11-07T08:04:29Z
rel:
  governs: [crates/audit/tests/control_operations.rs]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: validation-facts.json, sha256: b7d5656f7f9d6306136e30cb99461d6ccd9a2b8ce9d3f0110e1ae86df54c35b7, bytes: 2613, store: repo}
---

# Audit key custody retention process source

## What was measured

One normal hosted-only test was authored for the actual audit-control-retainer
binary, using the existing bounded child capture API with60second/64KiB bounds.
The three child invocations cover new provision and signed successor persistence,
exact latest-pin restart and stale-pin refusal. Checks include actual OS exit,
bounded stdout/stderr, exact reply-frame counts, shared-client verification of
captured signed anchors and final actual-store reopen after child exit.

Root then assigned A/shared target, jobs1/incremental0/testthreads1. All seven
pure retention/client/rotation units passed, fail0/ignored0. Scoped library
Clippy with warnings denied and retainer-binary cargo check passed. The full
control_operations integration target compiled/linked with --no-run; no native
test or service process executed. All captured source, Cargo manifest/lock and
toolchain hashes remained unchanged before/after checks. Slot returned promptly.
Start free34.38GiB/commit82%; after free34.5GiB/commit82%. Formatting and scoped
public Specfold checks passed. Exact sanitized facts are attached; raw logs and
before/after inputs remain in private evidence:root/20261107075026.

## How to read it

This is source/pure/static evidence, not a process or product result. fa327b7 remains the
previous immutable source checkpoint; its seven-pure/seven-native facts are not
rewritten to describe this eighth native test. No helper/ignored exception was
added, no raw transcript is published and no workflow dispatch occurred.
Captured reply validation does not establish live interactive timeouts, different
OS ownership/volume, host power-loss behavior or product Docker qualification.
