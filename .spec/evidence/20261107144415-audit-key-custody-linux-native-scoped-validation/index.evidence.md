---
id: evidence:root/20261107144415
title: Audit key custody Linux native scoped validation
status: active
created: 2026-11-07T14:44:15Z
updated: 2026-11-07T14:48:36Z
rel:
  governs: [crates/audit]
  depends_on: [plan:root/20261107135332]
files:
  - {path: facts.json, sha256: de4c6c451f7a04ca2192a68583626e13c270cb93a366a7e61ba32ace013fbb1f, bytes: 7811, store: repo}
---

# Audit key custody Linux native scoped validation

## What was measured

Exact immutable3d325bdbc2c182d9805688d74fe1087bd8851de2 on ordinary
REA-Ubuntu WSL/Linux x86_64, UID1000, native GNU Rust/Cargo1.99.1. Root-allocated
new isolated A target, jobs1/incremental0, locked/offline, single test thread;
source/fixture0700 and observed ext-family magic0xef53. Native API admission
required same-held-descriptor statx mount ID. No install/fetch, graph change,
privilege/account/service or global umask change.

Fmt0(0.202s), all-target check0(13.263s), strict Clippy0(2.925s). SIX Rust tests
actually passed in SIX individually selected single-summary harness invocations:

| Selected test | Passed | Failed / ignored / measured | Filtered |
| --- | --- | --- | --- |
| Descriptor inventory/other-reservation admission | 1 | 0 / 0 / 0 | 104 |
| Cancellation/admission before open | 1 | 0 / 0 / 0 | 104 |
| Absolute-path alias refusal | 1 | 0 / 0 / 0 | 104 |
| Actual new Unix WAL modes/rotation/restart | 1 | 0 / 0 / 0 | 104 |
| Existing0644 modes/dev/inode preserved | 1 | 0 / 0 / 0 | 104 |
| Controlled retained Source/crypto/fault case, explicitly --ignored | 1 | 0 / 0 / 0 | 104 |

The controlled case used actual signed2-record/2-checkpoint WAL, two complete
passes, independent positioned readers and last-reader lock retention/release.
Its15 deterministic mutation/admission subcases include same-size file and root
replacement, truncate/observed regrow/append, sidecar remove/add, cancel, wrong
mode, symlink/hardlink/FIFO, gap/unknown member and final barrier4 mutation after
both complete passes.15 subcases are ONE Rust test, not15 Rust tests. Historical
five subprocess helpers were not executed/countable in this selection. Missing
venue/Unsupported cannot return a counted healthy controlled result.

Actual exec session5480 terminal0; owned-target process observation no match
(pgrep1); A returned before read-only closing hashes. All2569 archive files,
67 package-scoped resolved registry packages/3181 files and72 selected Rust tool/
host/standard-library files matched hashes AND inventory before/after. All16
config/credential candidates remained absent; ambient Cargo/Rust variable count0
before/after. Explicit argv binds qualified tools/target/jobs/incremental/offline
and controlled fixture. Selected Rust tool closure does not attest external OS/
system linker files. Facts and exact named footers attached; raw logs/argv/
closures reside privately as `evidence:root/20261107142917`.

## How to read it

Original full-workspace metadata failed offline on asn1-rs0.7.2 and was preserved
in [[evidence:root/20261107143416]]. Root renewed allocation for package-scoped
offline cargo tree; it succeeded, resolving only audit's GNU normal/build/dev
graph. No source fix or network fetch was needed. No earlier failed attempt is
retroactively qualified. Archive SHA256 is recorded in attached facts.

This qualifies the scoped private Linux backend and prospective Unix0600 change
at this source/venue. Public native entrypoint still Unsupported; scanner remains
test-compiled, with no production startup/repair/healthy owner. No full audit/
workspace suite, independent witness transport, hosted service/Docker product,
FD exhaustion/actual peak, memory/RSS or hard-stall supervisor proof implied.
Parent/namespace trust and cooperative quiescence remain premises; metadata/
transcripts are not atomic snapshots or ABA oracles. Observed regrow starts from
an old sampled mtime. Ext-family is shared ext2/3/4, not independent proof of
exact ext4/physical media/persistent UUID.
