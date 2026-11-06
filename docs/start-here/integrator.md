---
layout: doc
title: Integrator guide
permalink: /start-here/integrator.html
reviewed: 2026-11-07
---
# Integrator guide

**Diátaxis quadrant:** How-to. **Audience:** application and infrastructure integrators.

The Rust replacement is not yet a qualified installation. Begin with a deployment
review instead of copying commands from an earlier Go release.

1. Draw the complete client-to-origin path and identify every TLS terminator.
2. Record where original connection facts, exact body bytes and browser hooks are available.
3. Choose required route capabilities. Treat missing required evidence as a readiness failure.
4. Reserve control routes and verify that they do not shadow application routes.
5. Keep administration local, authenticated and separate from adapter credentials.
6. Require a release manifest and deployment pilot for your exact combination before rollout.

Node/Bun middleware and nginx integration are migration targets, not current
support promises. Middleware behind a CDN cannot reconstruct the original
client TLS handshake. Review [topology boundaries](../reference/supported-topologies.md)
and the [origin contract](../reference/gate-origin-contract.md).
