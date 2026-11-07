---
id: evidence:root/20261107091756
title: Audit key custody bounded helper source
status: active
created: 2026-11-07T09:17:56Z
updated: 2026-11-07T09:17:56Z
rel:
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody bounded helper source

## Historical scope

Original governing path: `crates/audit/tests/key_custody.rs` in the former
humanymous-audit package. Evidence/source-recording commit:
de33864b0130b34f1b7cd7f43a839a56701d4b1a. The original source-only/quiet-window
scope and comparison with the older8132117 contract remain unchanged below.
After Runtime relocation64f74c7a this retained evidence has no active path
governance; it does not qualify the moved Runtime or create new test results.

## What was measured

Source-only repair of key_custody's two existing actual native subprocess
parents. Shared capture has60second/64KiB bounds, exit0/stderr0 and exact child
markers inside the captured transcript. Leading/trailing newline and explicit
flush guarantee marker framing. Original parent state checks remain, with
explicit pending presence/removal and unchanged signing/HMAC identity checks.
No new test, ignored entrypoint, dependency or production API was introduced.

## How to read it

No formatting, compilation or test execution occurred during the user-directed
benchmark quiet window. The old8132117 source uses an explicit legacy nested-
summary contract; this future source requires one outer summary. Runner owner
acknowledged the paired exact-helper/both-parent contract and authored rejection
fixtures for nested summaries, unknown ignored tests and leaked markers.
Both sides need fresh qualification together. Rawignored2 in this target and
rawignored4 across audit remain excluded from success counts. This source does
not retroactively qualify older probes or the full product.
