---
layout: doc
title: How request observations acquire meaning
permalink: /concepts/how-gate-sees-a-request.html
reviewed: 2026-11-07
---
# How request observations acquire meaning

**Diátaxis quadrant:** Explanation. **Audience:** protocol and application reviewers.

A request can carry connection facts, application context and browser assertions.
They have different authority. Direct capture is distinct from authenticated
forwarded evidence, and both differ from a field supplied by a visitor.

Observations need a capture boundary, completeness state and request binding.
Connection-wide HTTP/2 facts must not be attributed to another stream or tenant.
Parsed JSON is not the exact request body. An unavailable observation remains
unavailable rather than being inferred from a framework name.

The [project draft](../standards/draft-v0/protocol/profile.md) explores a common
model; exact interoperable wire details and Rust deployment evidence are pending.
