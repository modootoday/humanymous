---
layout: doc
title: Review request-integrity route requirements
permalink: /how-to/configure-attested-routes.html
reviewed: 2026-11-07
---
# Review request-integrity route requirements

**Diátaxis quadrant:** How-to. **Audience:** application integrators.

The Rust route configuration is not release-qualified. First identify the exact
body representation, method, target and session that an integrity decision must
bind. Complete verification before any origin effect on a route requiring it.

Set explicit size, time and storage limits. Test body mutation, replay,
cross-route use, cancellation and partial upload. Streaming routes require a
separate supported contract rather than silently omitting verification.
See the [origin contract](../reference/gate-origin-contract.md).
