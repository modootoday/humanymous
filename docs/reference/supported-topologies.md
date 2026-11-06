---
layout: doc
title: Deployment capability boundaries
permalink: /reference/supported-topologies.html
reviewed: 2026-11-07
---
# Deployment capability boundaries

**Diátaxis quadrant:** Reference. **Audience:** infrastructure integrators.

These are qualification profiles, not a list of released Rust deployments.

| Profile | Boundary |
| --- | --- |
| Direct TLS edge | Original handshake and protocol observations require capture before normalization |
| CDN or load-balancer origin | Upstream TLS is not the client's TLS; original facts are unavailable without an authenticated supported channel |
| Framework middleware | Request lifecycle hooks do not imply raw network or response-transformation capability |
| Managed edge hook | Provider observations, body access and execution limits require explicit qualification |
| Native server module | Worker, reload, application binary interface (ABI) and ownership behavior need independent pilots |

Required unavailable capabilities prevent route readiness. A deliberately limited
profile must declare its gaps. HTTP/3 capture is not qualified by an HTTP/2
implementation. Keep management local and authenticated in every profile.
See the [integrator guide](../start-here/integrator.md).
