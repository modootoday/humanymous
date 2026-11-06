---
layout: doc
title: Plan TLS termination
permalink: /how-to/https-tls-certificates.html
reviewed: 2026-11-07
---
# Plan TLS termination

**Diátaxis quadrant:** How-to. **Audience:** infrastructure operators.

Draw every TLS hop and name the certificate/key owner at each endpoint. Original
client handshake observations require direct capture before normalization; a
CDN's connection to the origin is a different observation.

Keep private keys outside source and evidence records. Define renewal, reload,
expiry and failed-reload tests using synthetic lab certificates. Do not disable
verification to make a pilot succeed. Rust certificate configuration and reload
behavior await integration evidence. See [topology boundaries](../reference/supported-topologies.md).
