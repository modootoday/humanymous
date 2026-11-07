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
| Artifact and service provisioning | Original `doctor`, `install`, `rollback` manage an explicit artifact; separate signed `service-*` commands implement owned lab-service lifecycle with full runtime qualification still false |
| Native integration CLI | `humanymous-native init` emits an unqualified framework plan; explicit manifest writing currently requires Windows |
| Browser collector | Rust/WASM browser reads, passive behavior and separate environment/advanced APIs exist; authenticated runtime integration and recovery remain incomplete |
| Laboratory browser | Windows Meari/CEF prototype has actual native rendering and window controls; full browser readiness, external navigation and SDK qualification remain separate |
| Node/Bun and nginx adapters | Rust Node-API and Linux nginx implementations exist with bounded historical pilots; changed-source platform, packaging and deployment qualification remain separate |
| Cross-platform packages | Source-bound native component runs exist; they do not qualify every service, browser or framework on those operating systems |
| Documentation | Historical pinned-theme builds exist; the changed current source still needs its own build/link/access review and is separate from production deployment |
| Standards Draft v0 | Independent developing proposal; no external submission or adoption |

Historical Go behavior is a migration comparison baseline, not evidence that the
new runtime is ready. A future release must name exact supported tuples,
capabilities, artifact digests and completed checks. Until then, use an isolated
evaluation environment and retain your existing production controls.

The 2026-11-07 integration checkpoint registers 38 Rust workspace members.
That inventory is not a supported-platform matrix. Local component checks
include 10 Go-adapter tests, 36 maintenance-tool tests and 26 documentation-tool
tests; their separate check/lint results do not establish full-product parity
or an immutable combined dependency graph. Two mandatory CEF desktop process
tests are wired but have not executed at this checkpoint.

Hosted [run `42355168729`](https://github.com/modootoday/humanymous/actions/runs/42355168729)
targets source `8132117`, which precedes that local integration checkpoint.
It completed with failure: five of 19 jobs succeeded, including both-baseurl
Pages builds; source-review inventory and native/product gates still failed.
These separate outcomes cannot qualify later source changes.
The current documentation source still needs a fresh rendered-site build,
actual browser checks and source-review completion before preview acceptance.

The standalone HTTP runtime uses plaintext loopback HTTP/1.1. Library source
also exposes `ServiceHost::bind_tls` with installation-owned TLS configuration
and a same-connection captured request owner. This optional TLS/H1 path accepts
only buffered native routes and HTTP/1.1 ALPN; it rejects browser profiles and
does not enable HTTP/2 capture. Its source wiring has not been qualified by a
current native ingress run. A readiness declaration is not deployment evidence,
and CDN re-termination still hides the original client's TLS handshake.
Browser recovery, public administration and arbitrary streaming proxy
compatibility remain unavailable. Routes requiring unavailable capture or a
qualified collector stay unready.
See [runtime configuration](cli-config-policy.md), [collector API](browser-collector.md)
and [native browser boundaries](native-browser.md) for the exact implemented scope.
