---
layout: doc
title: Automation evidence interoperability — Draft v0
description: An independent developing proposal for evidence provenance and request-bound decisions.
permalink: /standards/draft-v0/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---

# Automation evidence interoperability — Draft v0

**Diátaxis quadrant:** Reference. **Audience:** server, adapter and protocol implementers.

**Status:** independent project draft under development. Not an Internet-Draft,
RFC, W3C Recommendation, WHATWG standard or registered protocol. Reviewed
2026-11-07. No external submission or endorsement is claimed.

This proposal describes how cooperating security collectors and application
adapters could exchange the source, completeness and scope of observations and
bind decisions to the correct request. Authentication, authorization and a
person's presence remain different questions. The proposal does not promise to
identify all computer-use automation or prevent all abuse.

| Document | Purpose |
| --- | --- |
| [Explainer](explainer.md) | User problem, deployment scenarios, alternatives and open questions |
| [Protocol profile](protocol/profile.md) | Proposed requirements, actors and message model |
| [Retry and origin effects](protocol/retry-effects.md) | Candidate durable states and fail-closed treatment of uncertain effects; not implemented |
| [Security, privacy and accessibility](reviews/security-privacy.md) | Trust boundaries, data handling and human recovery |
| [Conformance and validation](validation/conformance.md) | Evidence required before an interoperability claim |
| [Standardization preparation](process/preparation.md) | Venue selection, contribution review and submission gates |
| [Experimental wire](protocol/implementation-experiment.md) | Concrete existing adapter protocol, separate from the generic proposal |
| [Implementation map](validation/implementation-map.md) | Candidate requirements versus actual component and deployment evidence |
| [Synthetic parser vectors](validation/vectors.md) | Exact-byte experimental inputs and their limited acceptance meaning |
| [Terminology](protocol/terminology.md) | Shared meanings without promoting assertions into verified observations |
| [Revision control](process/revision-history.md) | Editorial versus wire/behavior changes and future snapshot rules |
| [Reference maturity](process/references.md) | Existing standards, developing work and procedure boundaries |

The protocol model is incomplete until exact encodings, schema and test vectors
are reviewed. A conforming implementation cannot yet be claimed. All examples
are synthetic; there are no operational keys or visitor traces in this collection.

## Change record

Initial v0 introduces provenance, capability gaps, request-bound decisions and
before-origin integrity requirements. The collection remains a working draft.
An immutable revision will receive a source revision, dated change record and
validation report before it is described as a published snapshot.

The current editorial update adds named requirement links, concrete experimental
wire documentation and source-to-requirement mapping. Experimental parser
fixtures do not freeze a generic Draft v0 encoding or establish interoperability.
