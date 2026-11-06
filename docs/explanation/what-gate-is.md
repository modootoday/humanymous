---
layout: doc
title: What Gate does
permalink: /explanation/what-gate-is.html
reviewed: 2026-11-07
---
# What Gate does

**Diátaxis quadrant:** Explanation. **Audience:** prospective adopters.

Gate is the enforcement boundary between a client and an origin. It consumes
request-bound evidence and decisions, serves supported recovery routes and
prevents an origin effect when required conditions are not established.

That position matters: a detector can compute a result without stopping a request.
An edge must also preserve application semantics, isolate administration and
handle outages safely. Original connection facts depend on where TLS terminates.
Coherent real-browser automation remains a detection limit.

The Rust Gate is still being integrated and qualified. See
[implementation status](../reference/production-vs-reference.md) and
[where it fits](where-gate-fits.md).
