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
