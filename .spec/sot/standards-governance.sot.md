---
id: sot:root/standards-governance
title: Standards draft governance and preparation
status: active
created: 2026-11-06T21:41:01Z
updated: 2026-11-06T21:48:59Z
verify: [manual]
rel:
  depends_on: [sot:root/rust-migration]
---

# Standards draft governance and preparation

## Rule

Version 0.2.0. Accepted authoring/preparation rules, not external standards
adoption or proof that the proposed technical profile is complete.

### Continuous source reconciliation and experiment boundary

The docs session owns Pages and standards together. Review actual Rust sources,
public API surfaces and source-bound pilots after producer/authority/collector/
CLI/browser changes. Keep an explicit change manifest and implementation map;
never substitute an old chat completion or component green for current acceptance.
The persistent goal remains active while integration and standardization gates
are unresolved. No external communication or submission follows from this rule.

The next safe implementability slice is informative: public experimental adapter
wire documentation, P-01..P-10 implementation traceability and synthetic exact-
byte parser vectors. They describe humanymous.adapter/0.1 separately from the
generic proposed Draft v0. Reuse the actual humanymous-integrity parser in the
Rust Pages tool; do not copy validation, policy or request-integrity algorithms.
Fixture authoring and read-only validation are separate commands; CI cannot
regenerate expectations. Store exact bytes (hex), per-case digests, trusted
profile/clock and fixed expected acceptance, with all failures in the denominator.
Acceptance is not an ALLOW verdict, peer authentication, audit or origin effect.

Design panel covered blue/evaluator and red/privacy/SRE perspectives. Critique
reflected: public origin docs now state the runtime Authorization reservation;
IPC authority and session runtime capability matrices are separate; nginx parser
support is not executable authority profile selection. Generic wire encoding,
authentication/signature commitments, duplicate/retry reservation/retention/
restart epochs, streaming and independent implementations remain blockers.
Keep real human/screen-reader/Pass recovery not_run until actual evidence.
Synthetic zero-motor fixtures and automated accessibility scans do not replace it.

### Public document authority

Author the project Draft v0 manually in docs/standards/draft-v0/, using contextual
protocol/, reviews/, validation/ and process/ directories. These pages are public
GitHub Pages content, not an internal notebook. The existing migration delivery
contract governs pinned remote_theme, design tokens, Pages build and release
freshness. Never copy private .spec.local traces, secrets, raw personal data or
internal specification IDs into public pages. Do not generate normative prose
with documentation scripts or maintain divergent normative Markdown/RFCXML twins.

Each page declares layout: doc, title, description, stable permalink, project
draft status, version and review date. Use ordinary Markdown and baseurl-aware
links compatible with the pinned modootoday theme. Validate both apex and
repository-subpath builds, navigation, anchors, citations and accessible output.
The current legacy docs/theme is not completion authority for the new Pages site.
An authored source page is not claimed rendered/published until that build passes.

Public v0 is a developing project draft, not an IETF Internet-Draft, W3C report,
WHATWG standard or assigned registry entry. Local v0 and a future submitted -00
are different identifiers. Once a revision snapshot is declared published/frozen,
never replace its normative contents: add a new revision and migration note.
Record editorial vs wire/behavior changes and source revision in the release
manifest, not an invented commit hash in authored text.

### Technical authoring rules

Write an English-first abstract, user problem, scenarios, goals/non-goals,
alternatives and prior-art gap. Separate informative explanation from proposed
normative requirements. Each requirement has an ID, implementing actor and
observable pass/fail result; BCP 14 RFC 2119/8174 applies only to deliberate
uppercase requirement words. Define actors, trust boundaries, evidence source,
connection/request/stream identity, states, errors, limits, retries and replay.

Before submission-ready status, define exact types, byte/Unicode/encoding layer,
absent/null/empty/duplicate/unknown-field behavior, canonicalization/signature
scope, version negotiation and extension/deprecation. Trace every requirement to
schema, positive/negative vectors, implementation, pilot and review disposition.
Open wire questions are explicit blockers, never silent implementer assumptions.

Include security, privacy, accessibility, internationalization, deployment,
compatibility, resource bounds, conformance classes, IANA Considerations and
normative/informative references. Research states facts/provider claims/external
observations/inferences/unconfirmed items separately, with primary URL, version,
maturity and checked date. No undocumented detection policy becomes a standard.

### Venue selection and preparation gates

| Gate | Required evidence and allowed status |
| --- | --- |
| G0 scope | User problem, existing standards/WG gap map, alternatives and minimal new interoperability need |
| G1 project v0 | Public authored draft, explicit unadopted status, terms, requirement IDs and open issues |
| G2 implementability | Exact schema/bytes/error/limit/version rules and independent positive/negative vectors |
| G3 interoperability | Independent implementations/adapters, native/browser/Docker pilots and public-safe reproducible reports |
| G4 horizontal review | Security/privacy/a11y/i18n self-review and real external-review opportunity/disposition |
| G5 venue/rights | Appropriate community scope, editors/contribution authority, applicable IPR review and registry need |
| G6 submission package | Fixed source revision, venue format/RFCXML v3, boilerplate/format/nits and rendered human review |
| G7 external activity | Actual submission/review request and authoritative resulting URL/state, only within explicitly authorized external communication scope |

No external submission, message or review request is authorized merely by this
preparation task. Prepare concrete packages first. Do not claim an RFC, adoption,
review response, implementer endorsement or rights clearance without evidence.
Project Apache-2.0 licensing alone does not establish venue contribution/IPR
compliance; record that review separately without offering a legal certification.

| Venue | Appropriate scope and procedure boundary |
| --- | --- |
| IETF | HTTP evidence/decision interoperability: individual draft submission, community/WG discussion and possible adoption, reviews/Last Calls/IESG and RFC publication are separate steps. Anyone may prepare an individual draft; it is not automatically a standard |
| W3C/WICG/CG | Browser/API incubation: explainer/prototype/public feedback, appropriate chartered WG and Recommendation transition conditions; CG report is not Recommendation |
| WHATWG | Necessary HTML/Fetch changes through relevant issues, incubation, implementer support/tests and consensus; not a registry for an entire product |
| IANA | Only actual identifier needs, using each registry's policy. Registration does not endorse the complete technology |
| ISO/IEC or Ecma | Conditional longer-term committee/industry route with its own proposal, review/vote and rights procedures; not a shortcut for initial HTTP/browser profile acceptance |

WEBBOTAUTH concerns cryptographic automated-client authentication to
human-facing websites; its charter excludes HTTP API/agent-to-agent, end-user
authentication, nonparticipating bot detection and reputation. AIPREF concerns
content-use preferences and excludes enforcement/authentication/training audit.
The proposed profile must identify a concrete gap instead of duplicating these
activities or RFC 9421/9530/9309. No new browser API is proposed without a
demonstrated platform gap. Product scoring and vendor blocklists are outside
the proposed interoperability contract.

### Current procedural sources

Checked 2026-11-07 KST. W3C operative Process was the 2025-08-18 edition; a
2026 proposed Process draft is not substituted for the operative policy.
Recheck live procedures at submission time. Internet-Drafts expire within six
months and can be renewed; project v0 has not been submitted.

- [IETF draft preparation](https://www.ietf.org/participate/ids/), [standards process](https://www.ietf.org/process/process/), [submission instructions](https://datatracker.ietf.org/submit/tool-instructions/), [required content](https://authors.ietf.org/en/required-content), [IPR](https://www.ietf.org/process/ipr/).
- [W3C Process](https://www.w3.org/policies/process/), [incubation](https://www.w3.org/guide/incubation.html), [review](https://www.w3.org/guide/documentreview/), [Patent Policy](https://www.w3.org/policies/patent-policy/).
- [WHATWG Working Mode](https://whatwg.org/working-mode), [stages](https://whatwg.org/stages), [IANA requests](https://www.iana.org/protocols/apply), [registry guidance](https://www.iana.org/help/protocol-registration).
- [ISO directives](https://www.iso.org/sites/directives/current/consolidated/index.html), [Ecma bylaws](https://ecma-international.org/policies/by-laws/).
- [BCP 14](https://www.rfc-editor.org/rfc/rfc8174.html), [RFC style](https://www.rfc-editor.org/info/rfc7322/), [specification guidelines](https://www.w3.org/TR/qaframe-spec/), [explainer template](https://github.com/w3ctag/explainer-explainer/blob/main/template.md), [TAG reviews](https://tag.w3.org/workmode/design-reviews/).
- [Security/privacy questionnaire](https://www.w3.org/TR/security-privacy-questionnaire/), [internationalization](https://www.w3.org/TR/international-specs/), [WEBBOTAUTH charter](https://datatracker.ietf.org/wg/webbotauth/about/), [AIPREF charter](https://datatracker.ietf.org/wg/aipref/about/).
- [Pages source/publication](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site), [Jekyll Pages](https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/about-github-pages-and-jekyll), [baseurl filter](https://jekyllrb.com/docs/liquid/filters/), [theme](https://github.com/modootoday/jekyll-theme-modootoday/blob/v1.5.0/README.md).

## Why

The user requested subagent research, systematic canonical preparation and an
incremental public Draft v0 under the GitHub Pages documentation tree. Standards
have venue-specific processes, not a universal registration shortcut. Clear
maturity and implementability gates prevent project research being overstated.

## Verification

The supplemental adapter-parser internationalization corpus keeps the original
24-case fixture unchanged. It authors10cases: binary body00ff80; composed and
decomposed distinct exact-byte body commitments; visually similar digest mismatch;
Unicode JSON header/target; percent UTF8; invalid decoded UTF8/NUL/dot-segment
rejections. Current shared-parser outcome6accepted/4rejected is not independent
interoperability, generic normalization selection or downstream transport fidelity.
Fixture generation is explicit create-new Rust authoring; CI/build never refresh
inputs or expectations. Public vectors explain both JSON input and body byte layers.

Horizontal self-review records observable keyboard/focus, actual screen-reader,
sparse/slow-user, motion/zoom, recovery-loop and machine-client outcomes with
source/tuple and explicit passed/failed/blocked/not_run states. These are
informative review cases, not new generic wire requirements. Pages rendering
does not qualify the product challenge. Internationalization vectors must state
reject/preserve/named-normalization behavior and exact commitment preimages;
visual equivalence is not byte equivalence. Generic wire remains unfrozen.

Official-source read-only standards research completed. Current state is
authoring/preparation design only. Public draft content, rendered Pages checks,
wire conformance, external review and actual submission are separate gates.
