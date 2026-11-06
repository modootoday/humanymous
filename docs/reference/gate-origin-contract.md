---
layout: doc
title: Edge-to-origin contract
permalink: /reference/gate-origin-contract.html
reviewed: 2026-11-07
---
# Edge-to-origin contract

**Diátaxis quadrant:** Reference. **Audience:** application and proxy integrators.

The origin must reject direct bypass of the enforcement path. Strip or reject
externally supplied internal decision headers; only authenticated configured
peers may assert bound observations. Preserve legitimate application credentials.

Keep method, target/query semantics, duplicate header meaning and exact body
representation intact. Complete-body integrity finishes before any origin
effect. Bound buffering and timeouts; do not retry non-idempotent requests after
partial forwarding. Streaming needs its own tested contract.

Management is never an origin-facing public route. The Rust forwarding and
adapter implementations require conformance and actual deployment pilots before
these properties can be claimed for a release. See [compatibility](../explanation/will-this-break-my-app.md).
