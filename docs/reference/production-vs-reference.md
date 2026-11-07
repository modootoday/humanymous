---
layout: doc
title: Implementation and release status
permalink: /reference/production-vs-reference.html
reviewed: 2026-11-07
---
# Implementation and release status

**Diátaxis quadrant:** Reference. **Audience:** adopters and reviewers.

Reviewed 2026-11-07. This branch is an incomplete Rust migration of a reference
project. No production-hardening, universal detection or completed replacement
release is claimed.

| Surface | Qualification state |
| --- | --- |
| Rust libraries | Shared detection, integrity, audit and state components exist; unit comparisons are narrower than deployed parity |
| HTTP runtime | Experimental loopback HTTP/1.1 with bounded complete requests, sessions, request integrity and durable audit before origin dispatch |
| Artifact provisioning | `humanymous doctor`, `install` and `rollback` validate/manage an explicit local artifact; they do not activate a service |
| Native integration CLI | `humanymous-native init` emits an unqualified framework plan; explicit manifest writing currently requires Windows |
| Browser collector | Rust/WASM browser reads, passive behavior and separate environment/advanced APIs exist; authenticated runtime integration and recovery remain incomplete |
| Laboratory browser | Windows Meari/CEF prototype has actual native rendering and window controls; full browser readiness, external navigation and SDK qualification remain separate |
| Node/Bun and nginx adapters | Experimental contracts and bounded pilots exist; Rust replacements and exact deployment qualification are tracked separately |
| Cross-platform packages | Source-bound native component runs exist; they do not qualify every service, browser or framework on those operating systems |
| Documentation | Authored source with pinned-theme builds; a checked Pages artifact is separate from production deployment |
| Standards Draft v0 | Independent developing proposal; no external submission or adoption |

Historical Go behavior is a migration comparison baseline, not evidence that the
new runtime is ready. A future release must name exact supported tuples,
capabilities, artifact digests and completed checks. Until then, use an isolated
evaluation environment and retain your existing production controls.

The HTTP runtime does not provide original-client TLS or HTTP/2 capture,
browser recovery, public administration or arbitrary streaming proxy compatibility.
Routes requiring unavailable capture or a qualified collector stay unready.
See [runtime configuration](cli-config-policy.md), [collector API](browser-collector.md)
and [native browser boundaries](native-browser.md) for the exact implemented scope.
