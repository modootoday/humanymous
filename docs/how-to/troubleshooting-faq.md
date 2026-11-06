---
layout: doc
title: Diagnose an evaluation failure
permalink: /how-to/troubleshooting-faq.html
reviewed: 2026-11-07
---
# Diagnose an evaluation failure

**Diátaxis quadrant:** How-to. **Audience:** lab operators.

Record the exact artifact, configuration, topology and failing case first.
Separate unavailable capability, malformed input, dependency outage and policy
decision. Missing original TLS behind a CDN is a topology boundary, not a
browser bug to fix by trusting a header.

Check audit storage, route collisions, local management and origin bypass in the
owned lab. Preserve failures without copying secrets or raw visitor content.
Rust runtime troubleshooting procedures await deployment qualification.
See [incident preparation](../runbooks/incident-runbooks.md).
