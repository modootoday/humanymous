# Sealed key compatibility integration

## Native custody integration (2026-11-07)

Owner: rust-key-custody-next. Root has applied these target dependencies to audit:

```toml
[target.'cfg(unix)'.dependencies]
rustix = { version = "1.1", features = ["process", "fs"] }
[target.'cfg(windows)'.dependencies]
windows-permissions = "=0.2.4"
[target.'cfg(windows)'.dev-dependencies]
junction = { version = "=2.1.0", default-features = false }
```

Existing getrandom/zeroize suffice otherwise (custody uses standard-library OS
file locking, not an additional locking dependency). No membership or lib export
delta: `keys::custody` is declared by the owned keys/mod.rs. The safe dependency
owns Windows native FFI; workspace unsafe forbid remains unchanged. Actual
workspace native validation is now complete for this bounded persistence slice.

Additional exact delta above: junction is test-only, to create real NTFS reparse
points without relying on optional symlink privileges. It never enters custody
production dependencies. Root refreshed the lock; both actual workspace native
test commands passed with --locked. No further Cargo delta is requested.

Public API: `keys::custody::{Owner, Durability, Custody}`. Acquire a preprovisioned
private absolute directory and retain the guard for the full service lifetime.
`load_or_create` invokes the caller's provisioning closure only when committed
and pending files are both absent. `rotate` re-encrypts the complete current
identity/snapshot. `persist_vault` rejects key replacement/removal and tombstone
addition/loss. `persist_erasure` requires the existing ErasurePermit which has
no production issuer. `recover_pending` authenticates both envelopes when a
committed file exists and refuses identity changes and tombstone loss; new
erasure needs future permit-aware recovery. Recovery never runs automatically.

Paths within the service directory: keys.lock (persistent lifetime OS lock),
keys.sealed (committed legacy envelope), keys.pending (exclusive staged envelope).
No unilateral discard/quarantine operation exists. A partial pending file or
malformed committed file requires operator incident handling, never fresh keys.
Failure during commit poisons the object until reopen. Unix success reports
FileAndDirectory after both syncs. Windows reports
FileSyncedNamespaceUnconfirmed after file sync and rename. Do not convert that
status into a power-loss durable acknowledgement.

Validation: Rust1.99.1 actual workspace audit tests passed on Windows (49 cases)
and Linux local container (47 cases); native custody cases are 10 and 8 of those.
Two ignored subprocess entrypoints are invoked explicitly by parent tests and
are not counted as skipped successes. Workspace Clippy --all-targets -D warnings
passed on both OSes. Owned Rust formatting passes. Real NTFS junction and ACL
negatives, Unix modes/symlink/hardlink negatives, restart, child-process lock,
process-exit staged recovery and Windows rename-sharing failure are covered.
See evidence:root/20261107002839 for sanitized source and execution hashes.
Owned Specfold documents/bundles validate without errors or warnings. Full
repository spec check still reports pre-existing private analysis layout and
concurrently generated root-run attachments outside this lane; root must finish
that registration. Public owned files are LF and their working bytes match
Git-filtered blob bytes.

Integration premises: runtime serializes mutation/snapshot persistence before
acknowledging new pseudonyms and retains the guard. Ancestors remain supervisor
controlled; privileged/same-owner replacement, Windows hardlink aliases, remote
filesystems, actual power-loss/storage-controller semantics, cross-host rollback
and backup deletion are outside measured guarantees. No macOS run is claimed.
Windows permission API uses narrowly reviewed safe wrappers in
windows-permissions0.2.4 (native GetSecurityInfo with explicit SE_FILE_OBJECT,
SID parsing, basic owner allow ACE inspection). This is dependency trust, not
a proof of the dependency's entire unsafe implementation.

The source at `src/keys/mod.rs` is exported by humanymous-audit on native targets.
It was first verified directly as an isolated Cargo library, then connected to
the actual workspace after the authority owner handed back shared Cargo files.
WASM builds retain the pure audit APIs without host key-entropy dependencies.

Native target dependencies:

```toml
base64.workspace = true
aes-gcm = { version = "0.11.1", default-features = false, features = ["aes", "alloc", "zeroize"] }
scrypt = { version = "0.12.0", default-features = false }
zeroize = { version = "1.9.1", features = ["serde"] }
```

Run actual audit/workspace/MSRV/advisory/WASM checks after changes. The declared
historical MSRV 1.88 selected aes 0.9.2 because newer 0.9.4 requires Rust 1.89.
The user has now approved MSRV/toolchain1.99.1; the reviewed lock is retained
without an unrelated dependency update. Refer to execution evidence for scan results.

This is the frozen scrypt N=32768/r=8/p=1, 16-byte salt, 12-byte nonce,
AES256-GCM compatibility envelope. It is not a new stronger KDF policy or
versioned storage migration. Wrong password/tamper never creates fresh keys.
Normal seal/open reject empty passwords; explicit import_legacy exists only to
migrate historical stores. Null versus empty vault data is retained.

The vault also matches all 54 frozen HMAC/scrypt vectors, preserves
snapshot tombstones and removes reverse links on shred. Its erasure capability
has no production issuer until verified dual control is implemented.

Remaining required work: runtime/service startup wiring, independent witness
custody/anti-rollback storage, actual dual-control issuer/admin and permitted
erasure recovery, deployment provisioning, backup residual handling and Docker
whole-product e2e. A co-located witness seed does not establish independence.
Native persistence qualification is not product release qualification.

Fixture generator temporarily compiles unmodified frozen Go key/vault sources.
All material in tests/fixtures/keys.json is synthetic public fixture data.
Retire migration-only Go tooling at final cutover, retaining audited vectors.
