---
id: sot:root/audit-key-custody
title: Audit key custody
status: active
created: 2026-11-07T00:20:05Z
updated: 2026-11-07T00:31:45Z
verify: [cargo test -p humanymous-audit --locked]
rel:
  governs: [crates/audit/src/keys/**]
  depends_on: [sot:root/rust-migration]
---

# Audit key custody

## Rule

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
