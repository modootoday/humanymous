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

The Rust Node-API bridge now has experimental Express/Hono interfaces; its
release-qualified platform list is still empty. Node/Bun and nginx integration
are not general support promises. Middleware behind a CDN cannot reconstruct the original
client TLS handshake. Review [topology boundaries](../reference/supported-topologies.md)
and the [origin contract](../reference/gate-origin-contract.md).

The current loopback runtime and local adapter decision service are distinct
profiles. Request-integrity support in the runtime does not make it available
on the IPC authority's routes. Likewise a parser for a producer does not prove
that the executable authority selects and authenticates that producer profile.
Use the [implementation map](../standards/draft-v0/validation/implementation-map.md)
to distinguish parser, service, browser and origin-effect evidence.
