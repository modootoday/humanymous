---
id: sot:root/audit-key-custody
title: Audit key custody
status: active
created: 2026-11-07T00:20:05Z
updated: 2026-11-07T03:30:26Z
verify: [cargo test -p humanymous-audit --locked]
rel:
  governs: [crates/audit/src/keys/**, crates/audit/src/storage/exclusive_file.rs, crates/audit/src/storage/wal.rs]
  depends_on: [sot:root/rust-migration]
---

# Audit key custody

## Rule

The guard, not the last duplicate OS file description, defines the writer lock's
lifetime. A temporary inherited/duplicated description must not retain custody
after its owner guard is dropped. Keep the lock inode and all owner/ACL checks;
do not unlink, retry away WouldBlock, or relax exclusivity. Verify acquisition
is denied while each live guard exists and that closing a stale duplicate cannot
unlock a successor. Use a deterministic duplicate-description regression before
attributing the intermittent remote failure to process-spawn inheritance.

Keep the frozen sealed AES256-GCM/scrypt envelope and existing vault API.
Custody owns a canonical, already provisioned private service directory. Every
component rejects symlinks/reparse points; the supervisor must keep ancestors
and the directory stable against hostile same-owner replacement. Unix requires
current effective UID ownership, 0700 directory and 0600 regular single-link
files. Windows requires configured owner SID and only simple owner allow ACEs;
directories must propagate owner-only file and directory inheritance. Unknown,
NULL or foreign grants fail closed. No automatic permission widening/repair.

Hold a nonblocking OS exclusive lock on a persistent lock inode for the custody
object lifetime. Cooperating processes cannot load stale state or generate two
identities. Check/open via no-follow native flags and verify handle metadata and
ACL. Locks are local-filesystem/cooperating-writer guarantees, not distributed
locking or protection against privileged/same-owner attackers.

Creation occurs only when both committed and pending files are absent. Pending
always means explicit recovery required. Wrong passwords, corruption, partial
files, permissions and I/O errors never call the creation callback. Writes use
exclusive pending creation, complete encrypted write, file sync, same-directory
rename and Unix directory sync. A failed commit poisons the object until reopen.
Errors after rename mean uncertain durability; never acknowledge mutation.
Windows reports file-synced namespace-unconfirmed rather than promising power
loss durable replacement: std exposes no equivalent verified directory flush.

Explicit pending promotion authenticates it and (when a committed file exists)
authenticates the old file and requires identical writer/HMAC/witness identity.
Promotion is an operator recovery action; no automatic timestamp/size guessing.
Recovery also preserves existing subject keys and tombstones; pending erasure
requires future permit-aware recovery and currently fails closed. General vault
persistence cannot introduce new tombstones, remove existing keys or replace key
bytes. A separate persist_erasure requires the same unconstructible production
ErasurePermit as shred; no boolean enables it at the public boundary. Runtime
must serialize snapshot persistence against responses to concurrent mutations.
Vault snapshots persist only inside the encrypted envelope. Rotation retains
identity and tombstones; old passwords cease opening the current envelope after
successful replacement, but old backups and storage snapshots remain residuals.
Only the latest committed snapshot is authoritative; external rollback defense
and backup deletion are not implemented. No ErasurePermit issuer is added.

## Critique reflected

Native writer ownership is implemented once by a crate-private ExclusiveFile in
storage and consumed by both custody and WAL. It owns an already validated File,
acquires the standard-library nonblocking exclusive lock and attempts explicit
unlock on Drop before the File closes. An OS unlock error is not hidden by retry,
unlink or a fabricated success receipt; File close remains the final fallback
and reacquisition may fail closed. The lock filename/inode must remain unchanged.
There is no public clone/unlock API. A cfg(test)-only duplicate-description hook
enables deterministic guard-lifetime regressions for both consumers. Remove the
now redundant fs4 dependency rather than keeping parallel native lock mechanisms.

The standard File contract confirms that duplicate/inherited descriptions can
extend a lock until explicit unlock or all descriptions close. This motivates
the ownership fix; it does not independently identify which concurrent spawn
occurred in the earlier CI process. [Rust File locking contract](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).

Blue: reuse seal/open, never fork crypto. Red: stale snapshots under separate
locks lose tombstones; lifetime lock and serialized replacement are required.
SRE: rename success followed by flush failure has unknown outcome; poison and
reopen, preserving pending when present. DPO: encrypted backup residuals preclude
claims of durable global erasure. Evaluator: native subprocess/restart/fault tests
do not replace Docker product e2e or independent witness/admin qualification.

## Why

## Verification

Native tests exercise restart identity, vault/tombstone persistence, rotation,
parallel-process lock refusal, permission/alias rejection and interrupted pending
recovery. Actual workspace Cargo integration remains root-owned. Raw execution
logs remain private and only sanitized facts/hashes enter public Specfold records.
