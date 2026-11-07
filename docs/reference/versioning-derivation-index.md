---
layout: doc
title: Documentation review and versioning
permalink: /reference/versioning-derivation-index.html
reviewed: 2026-11-07
---
# Documentation review and versioning

**Diátaxis quadrant:** Reference. **Audience:** contributors and release reviewers.

These pages are edited directly. A review date records a content review, not a
runtime certification. Public documentation must match the exact artifact and
capabilities described by a release.

The maintainer review ledger maps governed source files to reader pages and records
source digests, review reasoning and required check identifiers. It is excluded
from the rendered site; that does not make repository metadata confidential.
Actual test evidence must identify the executed source, environment and outcome;
naming a check in the ledger does not prove it ran or passed. Changed source requires
a renewed review even when the prose remains correct. Missing mappings and stale
digests block release; a nearby Markdown edit does not repair the ledger.

Reviews can be renewed for a substantively checked subset while other source
changes remain open. A partial renewal does not approve the full inventory or
qualify a release. Preserve failures, filtered and ignored tests, and distinguish
read-only command/process checks from actual native service activation.

Versioned drafts retain their maturity statement and change record. A project
version does not imply standards-body publication. See [documentation style](../style-guide.md).
