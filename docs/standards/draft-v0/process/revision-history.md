---
layout: doc
title: Draft revision and change control
description: Editorial, experimental implementation and future normative wire changes have distinct acceptance gates.
permalink: /standards/draft-v0/process/revision-history/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---
# Draft revision and change control

**Diátaxis quadrant:** Reference. **Audience:** draft editors and implementers.

**Status:** developing project draft. No immutable normative snapshot, external
submission, registry assignment or adopted standard is claimed. Project `v0`
is distinct from an Internet-Draft submission revision such as `-00`.

| Change | Current record | Effect |
| --- | --- | --- |
| Initial authored draft | Problem, actors, provenance and candidate requirements | Incomplete generic model, not a complete wire profile |
| Public terminology clarification | Named P-01 through P-10 headings, defined profile identifiers, individual validation links and informative review scope | Editorial clarity; no detection-policy or candidate requirement semantics changed |
| Current implementation reconciliation | Experimental adapter wire, implementation map, exact-byte parser corpus and current runtime/collector/CLI boundaries | Documents concrete implementation separately; does not select a generic Draft v0 encoding |

The machine-readable [change manifest](../changes.json) lists the affected public
surfaces and change classes. Source-bound build/review reports record actual
source/fixture hashes separately. A date, a prose review or an older build does
not certify a new working tree.

## Before freezing a revision

Resolve generic wire types and limits, missing/null/duplicate behavior,
canonical commitments, authentication/discovery, version negotiation, replay
retention/epochs and streaming. Link every requirement to positive/negative
vectors and actual implementation/deployment evidence. Record unresolved cases,
independent interpretation and horizontal-review dispositions.

Name a fixed source revision and validation report before declaring an immutable
snapshot. Later normative wire/behavior changes require a new revision and
migration/compatibility note; an editor must not silently replace frozen normative
content. Experimental implementation corrections may be documented without
claiming that the generic profile has become complete.

Preparation and public-site rendering do not authorize an external submission.
Follow the [submission preparation](preparation.md) gates and retain actual
resulting URLs/states only after an explicitly authorized maintainer action.
