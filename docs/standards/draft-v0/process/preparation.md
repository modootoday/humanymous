---
layout: doc
title: Prepare a standards discussion
description: A preparation procedure for an independent project proposal, without claiming adoption or submission.
permalink: /standards/draft-v0/process/preparation/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---

# Prepare a standards discussion

**Diátaxis quadrant:** How-to. **Audience:** draft editors and maintainers preparing external review.

**Status:** independent project draft. This procedure has not produced an external
submission, adopted standard or registry assignment. There is no universal web
technology registration service; choose the community responsible for the
specific interoperability problem.

1. Define the user problem and compare existing specifications. Keep product
   detection policy, reputation and vendor blocklists outside the interoperable
   message contract. Explain why an existing signature or preference mechanism
   does not already solve the proposed gap.
2. Resolve the [protocol model's](../protocol/profile.md) open wire questions.
   Assign requirement IDs and connect them to exact schema, errors and positive/
   negative vectors. Keep informative examples distinct from complete messages.
3. Obtain independent implementation and [validation evidence](../validation/conformance.md).
   Publish only synthetic or appropriately reviewed public-safe artifacts.
4. Complete the [horizontal self-review](../reviews/security-privacy.md), then
   prepare concrete security, privacy, accessibility and internationalization review
   requests and a disposition record. A self-review is not an external response.
5. Select the venue and review its current scope and contribution policies.
   Record editor/contributor authority and rights review separately from the
   software's license. Do not assume Apache-2.0 satisfies every venue's obligations.
6. Freeze a dated source revision and render the public Pages document. Validate
   links, anchors, status/version, navigation, accessibility, sensitive-data
   exclusion and both apex and repository-subpath URLs.
   Bind the document and validation reports to the same candidate, recording
   relevant dependency inputs and any dirty-source commitments. Keep expected
   versus executed cases visible, including cases not reached after a runner
   failure. A successful earlier build cannot qualify changed documents.
7. Prepare the venue-specific submission package. If preparing an IETF draft,
   use the RFCXML version 3 XML document format, required boilerplate and
   formatting and editorial checks (often called nits checks),
   and read the rendered result. Avoid two divergent normative sources.
8. Submit or request external review only through an authorized maintainer action.
   Record the actual resulting URL/state. Preparation is not submission; an
   individual submission is not working-group adoption or a standard.

## Choose the relevant venue

Keep a dependency record with each reviewed source's title, dated edition,
publication status, stable URL, selected sections and intended normative or
informative role. Use the [checked reference editions](references.md#checked-editions-and-document-status)
as review inputs, then recheck operative venue rules and unresolved wire choices
before freezing a submission package. Referencing an external Recommendation
does not give this project draft the same maturity or endorsement.

| Venue | Applicable preparation |
| --- | --- |
| IETF | HTTP protocol profile: individual Internet-Draft, relevant community discussion, possible working-group adoption and later standards-process review |
| W3C or incubation community | Browser capability proposal: user-centered explainer, prototype, wide review and an appropriate chartered working group if pursuing Recommendation |
| WHATWG | Necessary change to HTML/Fetch: relevant issue, incubation, implementer support, tests and consensus |
| IANA | A real identifier need under the specific registry's policy; not endorsement of the entire protocol |
| ISO/IEC or Ecma | A separately justified committee/industry route, not a shortcut around protocol review |

An IETF Internet-Draft is time-limited and can be updated; a local project v0 is
not a submitted revision `-00`. A W3C Community Group report is not a W3C
Recommendation. WHATWG stages describe progress rather than product certification.
Recheck operative procedures before submission: proposed future process drafts
must not be substituted for current policy.

## Official preparation references

- [IETF drafts](https://www.ietf.org/participate/ids/), [process](https://www.ietf.org/process/process/), [submission tool](https://datatracker.ietf.org/submit/tool-instructions/), [required content](https://authors.ietf.org/en/required-content), [intellectual property rights (IPR)](https://www.ietf.org/process/ipr/).
- [W3C Process](https://www.w3.org/policies/process/), [incubation](https://www.w3.org/guide/incubation.html), [document review](https://www.w3.org/guide/documentreview/), [Patent Policy](https://www.w3.org/policies/patent-policy/), [Technical Architecture Group (TAG) explainer](https://github.com/w3ctag/explainer-explainer/blob/main/template.md).
- [WHATWG Working Mode](https://whatwg.org/working-mode), [stages](https://whatwg.org/stages), [IANA requests](https://www.iana.org/protocols/apply).
- [ISO directives](https://www.iso.org/sites/directives/current/consolidated/index.html), [Ecma bylaws](https://ecma-international.org/policies/by-laws/).

## Pages publication boundary

The authored Markdown in this collection is public-site content. Use the pinned
remote theme's document layout and ordinary relative links. Keep confidential
review notes and private validation traces outside the Pages source. Building or
deploying these pages does not submit the proposal to a standards organization.
