---
id: evidence:root/20261107052815
title: Pages Rust verification and standards experiment status
status: active
created: 2026-11-07T05:28:15Z
updated: 2026-11-07T05:36:57Z
rel:
  governs: [docs]
files:
  - {path: status.json, sha256: 08db59f7c4b992ab7fa8d7a2ee859002c5b31270629f85a38a5cf0864c397d34, bytes: 866, store: repo}
---

# Pages Rust verification and standards experiment status

## What was measured

The authored Pages tree contains 85 reader pages. Actual pinned-theme Jekyll
builds and upstream strict checks succeeded for both the empty baseurl and
`/humanymous`; the Rust DOM/link/anchor validator reported no errors. A later
apex rebuild includes the explicit collector guard-hook compatibility text.
The current in-app preview was visually observed at localhost port 8766.

The Rust Pages package passed 11 unit tests with no failures or ignored tests,
and strict all-target Clippy. The fixed experimental adapter parser corpus
passed 4 accepted and 20 rejected cases. Its SHA-256 is
`b1fc0069ef858ce36cf9cb6810a5852cfed73559a44fc6f2797f50232b11ed92`.
These are parser outcomes, not enforcement verdicts or Draft v0 conformance.

After those passes, a small change moves invalid-baseurl rejection before any
builder process. Its initial rerun was blocked by concurrent common workspace
manifest/lock drift under `--locked`; the failure is retained. After the common
lock was synchronized, all 11 tests, strict Clippy, source checks and 24 vectors
passed again. No dependency or lock changes were made by the Pages owner.

## How to read it

Private raw build/test logs are retained in the current Pages verification
bundle. Earlier failed invocations remain retained rather than overwritten as
successes. The source-review receipt remains stale and publication remains
blocked. Build success is not an authored review of every changing Rust source.

The public `P-01` through `P-10` identifiers are explicitly defined as this
project draft's profile requirements, with descriptive headings and validation
links. They are not priority levels or externally assigned standard numbers.
Generic wire negotiation, authentication and replay semantics remain open;
the implemented adapter experiment does not resolve them by implication.

No new complete accessibility, ordinary-human, screen-reader, deployment or
cross-implementation conformance result is claimed. The current browser check
observed the homepage and protocol-profile rendering only.
