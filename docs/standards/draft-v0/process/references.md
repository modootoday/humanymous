---
layout: doc
title: References and dependency maturity
description: Existing standards, procedural sources and developing work relevant to the project proposal.
permalink: /standards/draft-v0/process/references/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---
# References and dependency maturity

**Diátaxis quadrant:** Reference. **Audience:** protocol reviewers and draft editors.

**Status:** reference inventory for an independent project draft. Listing a
source is not adoption, implementation conformance or endorsement. The generic
transport/authentication profile has not selected its complete normative
dependency set.

## Existing standards considered as building blocks

| Source | Relevant boundary |
| --- | --- |
| [BCP 14: RFC 2119](https://www.rfc-editor.org/rfc/rfc2119.html) and [RFC 8174](https://www.rfc-editor.org/rfc/rfc8174.html) | Declared uppercase requirement words; ordinary lowercase prose remains ordinary language |
| [HTTP semantics, RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html) | HTTP meaning; does not define this project's evidence envelope |
| [HTTP Message Signatures, RFC 9421](https://www.rfc-editor.org/rfc/rfc9421.html) | Authenticated covered components; selected coverage, key trust and replay rules remain profile responsibilities |
| [Digest fields, RFC 9530](https://www.rfc-editor.org/rfc/rfc9530.html) | Specified content integrity; actual received bytes and encoding layer still need verification |
| [Robots Exclusion Protocol, RFC 9309](https://www.rfc-editor.org/rfc/rfc9309.html) | Crawler preference publication, not authorization or enforcement |
| [Trace Context](https://www.w3.org/TR/trace-context/) | Correlation; external trace IDs/sampling flags do not establish security authority |

## Informative technical and horizontal review

- [DOM trusted events](https://dom.spec.whatwg.org/#dom-event-istrusted): user-agent dispatch is not natural-person attestation.
- [Security/privacy questionnaire](https://www.w3.org/TR/security-privacy-questionnaire/) and [Privacy Principles](https://www.w3.org/TR/privacy-principles/): review aids, not project compliance certification.
- [Internationalization guidance](https://www.w3.org/TR/international-specs/) and [specification guidelines](https://www.w3.org/TR/qaframe-spec/): terminology, explicit conformance and unambiguous byte/locale rules.
- [Web Bot Auth work](https://datatracker.ietf.org/wg/webbotauth/about/) and [content-preference work](https://datatracker.ietf.org/wg/aipref/about/): recheck charter, draft version and scope before proposing overlap or an extension; a moving work item is not an RFC.

## Preparation procedures

Use [IETF author guidance](https://authors.ietf.org/),
[W3C Process](https://www.w3.org/policies/process/),
[WHATWG Working Mode](https://whatwg.org/working-mode) and the relevant
[IANA registry policy](https://www.iana.org/protocols/apply) at the time of an
actual preparation/submission action. Proposed future procedures do not replace
operative rules. Rights/contribution clearance and editor authority are separate
from this repository's software license.

These references support the [preparation checklist](preparation.md); they do
not report external review responses or an authorized submission.
