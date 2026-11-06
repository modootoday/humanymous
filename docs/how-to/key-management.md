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
