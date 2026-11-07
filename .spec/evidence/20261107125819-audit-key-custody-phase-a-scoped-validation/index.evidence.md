---
id: evidence:root/20261107125819
title: Audit key custody phase A scoped validation
status: active
created: 2026-11-07T12:58:19Z
updated: 2026-11-07T13:01:19Z
rel:
  governs: [crates/audit]
  depends_on: [sot:root/audit-key-custody]
files:
  - {path: facts.json, sha256: 242db6b37af6d7272835f01fac603e4ef8ea8f1043447c4acca5cd835e1e0e88, bytes: 1718, store: repo}
---

# Audit key custody phase A scoped validation

## What was measured

Root-allocated A, immutable git archive of exact4b49e933b496b569867a4909fdce556a66cb7a20,
ordinary-user Windows/Rust1.99.1, jobs1/incremental0/thread1, locked+offline and
exclusive existing target/validation-audit. First-attempt all-target check and
strict Clippy passed, no source fixes or repeated commands. The primary worktree
and its later/uncommitted changes are completely excluded from this source.

Three harnesses passed21 unique Rust tests, failed/ignored/measured0: scan7
(82 intentionally filtered), framing13 (76 filtered) and verification_parity1
(0 filtered). One outer summary each. All seven newly authored Phase-A tests ran,
including segment/torn refusal, source/final cancellation, exact metadata pin,
budget-before-owned-decode, no partial finish, Unsupported native and elapsed
deadline checks. The unchanged Go verifier fixture covers840 wire reports.
These rows overlap earlier oracle runs and are not additional Rust tests.

All2434 files in the immutable snapshot have identical hashes and inventory
before/after the batch. Archive identity/hash, admission, full source snapshots,
toolchain, command arguments/timings and logs are attached to private
evidence:root/20261107125610. The67.8MB archive itself remains a reproducible local
private input, not an attachment: Specfold's4MiB file cap refused it with exit2.
Its git SHA/archive SHA256 and this disposition are attached; no store-policy or
upload change was made. Session64423 terminal exit0; A returned, no process
or further workload held. Primary-tree edits cannot drift this isolated proof.

## How to read it

Scoped private Phase-A/component compatibility qualification only. The Source
fixture is an in-memory oracle, not actual native file identity or allocation
measurement. Metadata intentionally admits invalid cryptographic fields and does
not return a crypto pass, trusted EOF token or healthy owner. No Phase B/native
provider, complete S1,128MiB/RSS, production S2/S3, native/protected/hosted or
product e2e qualification follows. Captured historical full140 is a different
source/scope, not an additional151 or161 unique test total.
