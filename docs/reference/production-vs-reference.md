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
| Documentation | Fixed-source pinned-theme builds, rendered-link checks, archive round trips and representative browser observations exist; complete source review and accessibility qualification remain separate |
| Standards Draft v0 | Independent developing proposal; no external submission or adoption |

Historical Go behavior is a migration comparison baseline, not evidence that the
new runtime is ready. A future release must name exact supported tuples,
capabilities, artifact digests and completed checks. Until then, use an isolated
evaluation environment and retain your existing production controls.

Workspace inventories and component test counts belong to their exact source
checkpoints. Neither a workspace member count nor a passing component suite is
a supported-platform matrix or proof of full-product parity. Native browser
process tests and framework deployment pilots need their own execution evidence;
a documentation preview does not exercise them.

Hosted [run `42355168729`](https://github.com/modootoday/humanymous/actions/runs/42355168729)
targets source `8132117`, which precedes that local integration checkpoint.
It completed with failure: five of 19 jobs succeeded, including both-baseurl
Pages builds; source-review inventory and native/product gates still failed.
These separate outcomes cannot qualify later source changes.

At documentation source `269607366e4b2e37b47a974595b406b66642caae`, both the
site root and `/humanymous` builds produced 88 pages and 165 public files.
Rendered checks and two separate archive restorations passed for each build.
The restored files were then served on literal loopback addresses and inspected
in the in-app browser: representative desktop and 390-pixel mobile layouts,
navigation menus and keyboard skip-to-content behavior were observed.
The security-disclosure file matched its archived bytes over HTTP.
These observations apply to that source, not later edits. They are not a full
browser-page corpus, assistive-technology review, WCAG conformance result or
production deployment. Complete source-review reconciliation remains pending.

The standalone HTTP runtime uses plaintext loopback HTTP/1.1. Library source
also exposes `ServiceHost::bind_tls` with installation-owned TLS configuration
and a same-connection captured request owner. This optional TLS/H1 path accepts
only buffered native routes and HTTP/1.1 ALPN; it rejects browser profiles and
does not enable HTTP/2 capture. Six selected Windows TLS cases at runtime source
`1646c4821a5da2208a65d349656a10ec657ebeb5` passed, with 46 other library cases
filtered out. Both recorded identity cases returned CHALLENGE/403 and did not
invoke the origin. This establishes neither TLS ALLOW-to-origin behavior nor
complete ingress, browser or product qualification.
A readiness declaration is not deployment evidence,
and CDN re-termination still hides the original client's TLS handshake.
Browser recovery, public administration and arbitrary streaming proxy
compatibility remain unavailable. Routes requiring unavailable capture or a
qualified collector stay unready.
See [runtime configuration](cli-config-policy.md), [collector API](browser-collector.md)
and [native browser boundaries](native-browser.md) for the exact implemented scope.
