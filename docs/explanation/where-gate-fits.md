---
layout: doc
title: Where the enforcement edge fits
permalink: /explanation/where-gate-fits.html
reviewed: 2026-11-07
---
# Where the enforcement edge fits

**Diátaxis quadrant:** Explanation. **Audience:** infrastructure architects.

A useful topology makes the enforcement path unavoidable while keeping
administration off the public network. The origin accepts only the configured
edge identity or network path. Public routes never proxy management handlers.

If a CDN terminates TLS first, the next hop sees the CDN's connection rather
than the visitor's original handshake. Framework integration cannot reverse
that loss. Supported authenticated provenance may supply selected facts, but
an ordinary forwarding header is not that channel.

The Rust migration has not qualified these deployments. Review each
[capability profile](../reference/supported-topologies.md) before designing a pilot.
