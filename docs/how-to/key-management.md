---
layout: doc
title: Prepare key lifecycle tests
permalink: /how-to/key-management.html
reviewed: 2026-11-07
---
# Prepare key lifecycle tests

**Diátaxis quadrant:** How-to. **Audience:** security operators.

Inventory key purposes and keep signing, administration and subject-erasure
authority separate. Store no operational keys in the repository or public
examples. Use synthetic keys for migration evidence.

Define rotation overlap, revocation, restart continuity and rollback behavior.
Test old/new credential boundaries and loss of the key store. Preserve exact
cryptographic domain separation and byte encoding during migration. Rust key
lifecycle operations remain subject to deployment qualification.
See [erasure preparation](../runbooks/erasure-crypto-shred.md).

## Separate envelope rotation from identity rotation

The current sealed compatibility envelope uses scrypt and AES-256-GCM with fresh
salt/nonce on sealing. This preserves the historical format; it is not a new KDF
policy, generic standards wire or signing-identity rotation mechanism. `seal`
returns encrypted bytes, not an atomic durable file replacement or ACL check.
Owned-buffer zeroization does not establish erasure from OS snapshots, allocator
copies or historical backups.

For a synthetic native custody pilot, prepare these observations:

| Case | Result to verify |
| --- | --- |
| Provisioning | The private absolute directory is already provisioned and a lifetime exclusive guard is retained; no automatic permission repair |
| Existing or pending state | Wrong password/corruption never calls fresh-key creation; any pending envelope requires explicit recovery |
| Passphrase change | Writer/HMAC/witness identity and complete vault snapshot remain unchanged; old/new credentials have the expected access boundary |
| Approved rotation | Approval binds the exact current ciphertext generation before consumption; the trusted caller selects the new secret |
| Interrupted replacement | Authenticate committed and pending envelopes, preserve identity and tombstones, inspect actual reported sync strength and retain failed-attempt evidence |
| Vault mutation or erasure | Ordinary snapshot persistence cannot introduce an unauthorized tombstone; exact-subject erasure and backup retirement remain distinct |

Approvers in the current rotation API do not sign the new passphrase or prepared
new ciphertext. Do not describe it as fully destination/body-bound approval.
A future signing-key or witness-identity rollover needs its own accepted migration
and recovery contract. Normal seal/open reject empty passphrases; the explicit
legacy import path exists only for migration and is not normal provisioning.
Do not discard a persistent lock inode or pending state to force a successful
startup. Unix directory sync and Windows namespace-unconfirmed outcomes remain
different durability claims. See the [audit boundaries](../reference/security-audit.md).
