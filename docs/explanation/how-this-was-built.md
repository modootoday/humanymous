---
layout: doc
title: Development and evidence method
permalink: /explanation/how-this-was-built.html
reviewed: 2026-11-07
---
# Development and evidence method

**Diátaxis quadrant:** Explanation. **Audience:** contributors and technical reviewers.

Development separates a written contract, implementation, verification and
release. The Rust migration compares observable behavior with the existing
implementation and preserves frozen detection policy. One successful crate
test is not a completed product migration.

Pure byte transformations receive boundary vectors. Integration tests exercise
request lifecycles and failure handling. Native OS/browser pilots and the full
isolated Docker lab establish the deployment evidence that library tests cannot.

Public documentation is authored Markdown using a pinned upstream theme. Checks
validate content and rendering without generating the prose. Private execution
records remain separate from public synthetic examples. See
[review and versioning](../reference/versioning-derivation-index.md).
