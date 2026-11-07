---
id: evidence:root/20261107052001
title: Exact-source full CI failure and Rust delivery follow-up
status: active
created: 2026-11-07T05:20:01Z
updated: 2026-11-07T05:30:59Z
files:
  - {path: summary.json, sha256: 753f1e86267897755bf84e5fd5d64157ca92e2c33b6dca661629686c9fa1557b, bytes: 5330, store: repo}
---

# Exact-source full CI failure and Rust delivery follow-up

## Measured boundary

Full run42339951733 belongs only to
f32042af9136560b41c1765f565c2e04fe373a22 and completed failure. The attached
sanitized summary preserves all16job outcomes:4success and12failure, including
the required aggregate. Policy, Pages build, legacy Node checks and pinned
toolchain tests passed. All three native jobs reached actual compilation/unit
execution/release build and then refused the missing full native runner.
That is not a completed native product deployment or cross-OS qualification.

The same-source failures include missing browser/Docker runners, Go shipping
image rejection, publish=false packaging, stale documentation inventory, Unix
native-cli test-helper Clippy dead-code errors and frozen Go dependency findings
in the root filesystem scan. No later source's component success replaces this
run. Raw logs, Windows actor reports, exact image/build metadata and artifact
payloads remain private and are not attached publicly.

## Follow-up implementation scope

The new Rust delivery tool separates source/working-byte commitments, workflow
authorization, package/registry barriers, owning-runner integration, dependency
closure and image provenance verification. It does not implement missing product
Core/Gate/Pass/human/AT functionality or external registry account setup.
Initial Rust boundary tests and own preliminary audit are component evidence;
final source-bound checks, shared Cargo integration and real owning-runner
pilots must be recorded independently after implementation settles.

No publish, tag, main merge, external registration or broad detection claim was
performed. The delivery plan remains active and full_product_qualified=false.
