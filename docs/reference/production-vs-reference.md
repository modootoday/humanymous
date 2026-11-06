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
| Rust libraries | Work in progress; crate tests do not qualify a deployed system |
| Gate, browser WASM and installer | Full integration and deployment acceptance pending |
| Node/Bun and nginx adapters | Target integrations; support requires exact-version pilots |
| Cross-platform packages | Native OS validation and release acceptance pending |
| Documentation | Authored source; build evidence is separate from runtime evidence |
| Standards Draft v0 | Independent developing proposal; no external submission or adoption |

Historical Go behavior is a migration comparison baseline, not evidence that the
new runtime is ready. A future release must name exact supported tuples,
capabilities, artifact digests and completed checks. Until then, use an isolated
evaluation environment and retain your existing production controls.
