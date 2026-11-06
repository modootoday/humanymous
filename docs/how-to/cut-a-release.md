---
layout: doc
title: Review release readiness
permalink: /how-to/cut-a-release.html
reviewed: 2026-11-07
---
# Review release readiness

**Diátaxis quadrant:** How-to. **Audience:** release maintainers.

The Rust replacement is not ready for final release. Do not publish a partial
migration as the completed replacement or deploy these draft docs as that release.

Require exact-version locked builds and native pilots, full isolated Docker
evaluation, browser/WASM checks, fresh dependency scans, package-content review
and current documentation evidence. Preserve failures and missing prerequisites.
Check artifact/version identity and scoped trusted publication credentials.

Publishing remains a separate authorized action after qualification. This page
does not grant registry or Pages publication permission. See [status](../reference/production-vs-reference.md).
