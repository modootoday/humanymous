---
layout: doc
title: Plan audit continuity verification
permalink: /how-to/verify-audit-log.html
reviewed: 2026-11-07
---
# Plan audit continuity verification

**Diátaxis quadrant:** How-to. **Audience:** audit reviewers.

Rust audit CLI usage is not release-qualified. Prepare synthetic records with
known canonical bytes, roots and inclusion/consistency expectations. Retain
valid historical vectors as migration comparison evidence.

Test tampering, malformed proof lengths, truncated storage, restart/replay and
unavailable durable storage. Verify the claimed commitment scope; a valid proof
does not establish that an event was benign or that no event was omitted before
ingestion. Record tool/artifact identity and residual limits.
See [security evidence](../reference/security-audit.md).
