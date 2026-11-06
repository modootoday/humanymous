---
layout: doc
title: Application compatibility boundaries
permalink: /explanation/will-this-break-my-app.html
reviewed: 2026-11-07
---
# Application compatibility boundaries

**Diátaxis quadrant:** Explanation. **Audience:** application developers.

Protection can change how a request reaches an application. Route reservation,
body verification, browser insertion and challenge responses need explicit
compatibility tests; no universal transparent-integration guarantee is made.

Test signed requests, large uploads, compression, cookies, redirects, caching,
content security policy and cancellation. Streaming, WebSocket and gRPC require
their own lifecycle contract. HTML insertion must not modify API bodies,
immutable responses or unsupported content. Complete-body verification must
finish before application side effects.

The Rust adapters are not release-qualified. Start with a disposable application
fixture and the [edge-to-origin contract](../reference/gate-origin-contract.md).
