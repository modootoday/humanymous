# Sealed key compatibility integration

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
MSRV 1.88 selects aes 0.9.2 because newer 0.9.4 requires Rust 1.89; do not raise
the supported toolchain silently. Refer to execution evidence for scan results.

This is the frozen scrypt N=32768/r=8/p=1, 16-byte salt, 12-byte nonce,
AES256-GCM compatibility envelope. It is not a new stronger KDF policy or
versioned storage migration. Wrong password/tamper never creates fresh keys.
Normal seal/open reject empty passwords; explicit import_legacy exists only to
migrate historical stores. Null versus empty vault data is retained.

The vault also matches all 54 frozen HMAC/scrypt vectors, preserves
snapshot tombstones and removes reverse links on shred. Its erasure capability
has no production issuer until verified dual control is implemented.

Remaining required work: atomic durable file storage/native ACLs, load-or-create
under an exclusive lock, passphrase rotation, durably persisted vault/tombstones,
dual-control erasure and actual deployment key custody. A co-located
witness seed does not establish independence. Do not advertise these as complete.

Fixture generator temporarily compiles unmodified frozen Go key/vault sources.
All material in tests/fixtures/keys.json is synthetic public fixture data.
Retire migration-only Go tooling at final cutover, retaining audited vectors.
