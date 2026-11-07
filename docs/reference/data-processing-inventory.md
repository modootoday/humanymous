---
layout: doc
title: Data-processing review inventory
permalink: /reference/data-processing-inventory.html
reviewed: 2026-11-07
---
# Data-processing review inventory

**Diátaxis quadrant:** Reference. **Audience:** privacy reviewers and operators.

This is an inventory template for review, not an attestation of a deployed Rust
service. Populate it from actual configuration and capture evidence.

| Data family | Required review |
| --- | --- |
| Connection observations | Provenance, minimization, retention and shared-network effects |
| Request integrity | Exact byte layer and digest scope; avoid retaining raw payloads |
| Browser aggregates | Necessity, lifetime and assistive-technology impact |
| Credentials and pseudonyms | Purpose, scope, rotation, access and erasure |
| Audit commitments | Durability, continuity, authorized review and deletion design |
| Analytics | Sampling/loss, bounded dimensions and independent retention |

Raw passwords, forms, clipboard data and screenshots are excluded from routine
collection. Hashing an identifier does not make it anonymous. See the
[privacy companion](../explanation/dpia-companion.md).

## Transient transport capture candidate

The [same-transport capture candidate](supported-topologies.md#same-transport-capture-candidate)
can retain handshake metadata and ordered decrypted HTTP/2 header bytes in its
connection/stream owners. Those headers can contain Authorization, cookies or
other linkable application data. Transient buffering is not permission to export
them into analytics, logs or public conformance fixtures. Review necessity,
minimization and access at the consuming runtime boundary before deployment.
Dropping an owner or closing a socket is not proof of physical memory erasure or
removal of application, operating-system or diagnostic copies. Use synthetic
inputs for public examples and preserve independent operational retention rules.

## Retention classification versus storage action

The audit library's `RetentionPolicy` classifies supplied ages into HOT, WARM,
COLD and EXPIRED. Its compatibility defaults are cumulative 90, 365 and 2555
fixed-day boundaries, inclusive at each boundary; they are not a record of the
retention configured for a deployment. Input windows use nanoseconds, and display
days truncate rather than substitute calendar years or leap-year arithmetic.

An EXPIRED label does not delete files, erase backups or authorize retirement.
COLD does not create WORM storage. Negative ages remain HOT; report clock skew
separately rather than treating future records as eligible for deletion. Actual
retirement needs checkpoint-preserving storage transitions, authorized handling
and evidence of residual copies. See the [erasure pilot](../runbooks/erasure-crypto-shred.md).

## Subject linkage and secret snapshots

The reference vault uses random per-subject keys for its provisioned HMAC or
scrypt pseudonym mode. Its snapshot contains secret keys and reverse-link data;
seal and restrict that snapshot rather than treating it as a public validation
artifact. Historical snapshots do not encode the complete trusted scope/mode
policy, so restore them only with the independently provisioned settings.

Scoped erasure removes the subject key and reverse links and retains a stable
non-remint tombstone. That marker can remain linkable; neither it nor a keyed
pseudonym is an anonymity certificate. Tombstones consume bounded capacity and
are not evicted to create fresh subject identities. Runtime authentication and
audit of reverse lookup, durable snapshot persistence, backup retirement and
KMS separation remain distinct from this in-process object.
