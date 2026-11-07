---
id: evidence:root/20261107075243
title: Python Ruby adapters shared Node compilation
status: active
created: 2026-11-07T07:52:43Z
updated: 2026-11-07T08:28:03Z
rel:
  governs: [crates/node-adapter]
files:
  - {path: summary-001.json, sha256: 09ba1cdf3e041fbbf16f6ff1546e49a4261a55a3bae2e8c187fd6f21524f89d9, bytes: 1306, store: repo}
  - {path: summary-002.json, sha256: cf779ccd005511ab728655fcb2f185eec98368ece59cf722a4d59390ce1055cf, bytes: 1839, store: repo}
---

# Python Ruby adapters shared Node compilation

## What was measured

Root explicitly assigned isolated local B target/node-native/build with
CARGO_BUILD_JOBS=1 and CARGO_INCREMENTAL=0 after its graph integration completed.
The Node all-targets host-pilot check passed in41.35s and scoped Clippy -Dwarnings
passed in3.60s. No actual Node/Bun/framework/authority host process was started.
Resource guard before check:34.79GiB available/82% committed; before Clippy:
34.26GiB/83%. B was returned immediately on success.

Owned source checkpoint78562be (including earlier Node migration44acbbf) was
clean at completion. HEAD observed after commands65166bb6e9b35caadac338978c593ceeb1a2a6ab.
This observed HEAD is not falsely presented as an immutable whole-tree execution
snapshot. Concurrent shared-dependency source mutation was not independently
monitored. Final hosted qualification still requires an immutable exact checkout.

Raw logs remain private; public sanitized receipt includes commands and hashes.
Cargo.lock SHA256 observed after completion:
06c2ed0db6d101a6e44c0e8186f550b2f565339ffe4a9ff849daddcf17146996.
Raw check SHA256 aed031ff295fba6331ec83bb31082e1aec9a64e20dcd897e46b1cfcf372e7caa;
raw Clippy SHA256324c091c91dbe0ae21097c52c0428902f4c93ac7ae2204d2d142f4af322b7b54.

## How to read it

Compilation/lint only. No unit execution, native loader/package verification,
actual runtime/framework/IPC/ABI/browser or whole-product qualification follows.
The retained older Windows52/Linux36 Node runtime receipts do not validate this
new shared-context source. All Python/Ruby native/package/runtime gates are
separately tracked; no default-empty-crate result counts as native success.

## Python native compilation follow-through

Root separately assigned the same isolated B target for Python all-targets
host-pilot. Check002 passed1.97s against source ba0c291. Clippy001 failed on two
collapsible-if diagnostics, then those two owned source expressions were changed
to equivalent let-chain/conjunctive forms and Clippy002 passed0.79s with-Dwarnings.
No lint was suppressed. This final Clippy invocation also type-checked the fixed
source. The initial E0277 check001 remains a retained failure, not overwritten.

Summary002 binds the final two changed source hashes and all three raw command
log hashes. HEAD observed after the commands5016be6a4a5702b77a7f586c2e948c645f75c883
is observation metadata, not a claimed immutable whole-tree execution snapshot.
No test/import/product/host process ran. In particular the required-feature
Python package-tool binary was excluded from this host-pilot build and remains
unverified. Ruby native and its new package driver are separate unfinished gates.
