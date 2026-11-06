---
layout: doc
title: Why the evaluation lab is isolated
permalink: /explanation/red-catalog-architecture.html
reviewed: 2026-11-07
---
# Why the evaluation lab is isolated

**Diátaxis quadrant:** Explanation. **Audience:** defensive researchers.

The lab contains synthetic clients, enforcement and a disposable origin in a
controlled topology. Isolation bounds the target and makes capture provenance,
forwarding and origin effects observable together.

A host-only client can miss the network topology that a full enforcement claim
depends on. Docker runs are therefore the completion authority for full-system
evaluation. Real OS and browser pilots provide additional evidence, not a
substitute for the isolated suite.

Keep attack artifacts separate from public defender packages. The Rust lab
must still complete integration and parity runs. See [evaluation rules](../reference/red-team-rules-of-engagement.md).
