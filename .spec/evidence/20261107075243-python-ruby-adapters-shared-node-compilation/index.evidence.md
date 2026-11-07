---
id: evidence:root/20261107075243
title: Python Ruby adapters shared Node compilation
status: active
created: 2026-11-07T07:52:43Z
updated: 2026-11-07T09:49:21Z
rel:
  governs: [crates/node-adapter]
files:
  - {path: summary-001.json, sha256: 09ba1cdf3e041fbbf16f6ff1546e49a4261a55a3bae2e8c187fd6f21524f89d9, bytes: 1306, store: repo}
  - {path: summary-002.json, sha256: cf779ccd005511ab728655fcb2f185eec98368ece59cf722a4d59390ce1055cf, bytes: 1839, store: repo}
  - {path: summary-003.json, sha256: b4b9498c9d6693ffa948a1fd10a3fa5ca85d87f6a1b0e21dfa7d3298c0145958, bytes: 3905, store: repo}
  - {path: summary-004.json, sha256: fa91c3441f1618771c7b8f34404347728e053cdc7b79202901771230e6c8abfc, bytes: 1194, store: repo}
  - {path: summary-005.json, sha256: 781917464064e470b1b1b88e6c1c7a8e3fc5a7505d4e9fa0a01a19bb15c257e5, bytes: 2887, store: repo}
  - {path: summary-006.json, sha256: f4397ca167766758ffe5ff19aff3dba7a735c5b188fdf1ba990003261d6c51b8, bytes: 4789, store: repo}
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

## Package drivers and shared Node fixture follow-through

Summary003 records six successful root-assigned B commands after the approved
graph window: Python package-tool all-targets check/Clippy, default Ruby Rust
driver all-targets check/Clippy, and Node host-pilot all-targets check/Clippy after
test fixture extraction. Selected184 source/common files changed0; excluded
transitive crates are named, so no full-graph stability claim. No tests, native
imports, frameworks or installers ran. Ruby default does not enable Magnus.
New Python Unix WSGI source was excluded by platform and feature cfg and has no
compile/runtime evidence from these Windows commands.

## Source813 hosted Node stage result

Summary004 retains final job128361689495 from run42355168729. Native build and
inspection stages exited0, then the required qualification.json was missing.
Exact813 package flow did not execute actual host_process tests before that gate;
native generation/loader inspection is distinct from framework/authority proof.
The LF-normalized API log remains private with its hash. Archive packing was not
reached; neither this historical result nor later source-only feature guards
qualify current packages or runtime tuples.

## Default Ruby pure tests and retained lint failure

Summary005 records a new assigned B batch: Ruby library2 and package-driver2
pure tests passed with zero failures/ignores/filters. All-targets Clippy failed
one items_after_test_module diagnostic. The batch stopped before Node commands
and B was returned. Conservative recursive closure tracking covered13packages/
251files and observed one changed test-support Docker source path; no full-graph
stability is claimed. Owned Rust stayed frozen. Moving the unchanged Drop impl
before tests is a later order-only fix whose retry remains NOT_RUN. Neither the
pure tests nor the default feature compile Ruby native callbacks, ABI or packages.

## Successful Ruby and Node retry with closure drift

Summary006 records the assigned retry: Ruby4pure and all-targets Clippy passed;
Node default and host-pilot all-targets check/Clippy passed, six command exits0.
The owned source was frozen. Conservative recursive13-package/252-file tracking
observed two audit paths change, with no additions/deletions. Therefore these
passes do not establish immutable whole-graph qualification. The earlier failed
Clippy attempt remains retained in summary005. No native Ruby, Node runtime/
installer/framework, or Unix Python WSGI execution occurred; Node pure parser
definitions were compiled by all-targets but not executed.
