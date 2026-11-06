---
layout: doc
title: Why exchange evidence provenance
description: The deployment problem behind the independent automation evidence proposal.
permalink: /standards/draft-v0/explainer/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---

# Why exchange evidence provenance

**Diátaxis quadrant:** Explanation. **Audience:** application owners, browser engineers and protocol reviewers.

**Status:** independent developing project draft, not an adopted standard.
This proposal does not solve the distinction between a person and automation
using an internally consistent real browser at a human pace.

## The deployment problem

A service may receive traffic through a content delivery network, ingress proxy,
application middleware and a separate decision service. Each sees different
facts. A middleware object may contain header names after normalization. A TLS
fingerprint at an origin may describe the proxy rather than the visitor.
A partial body may omit the bytes on which a decision depends.

If these differences disappear during exchange, a decision can appear better
supported than it is. The proposed profile makes the source, capture boundary,
completeness and request binding visible to every consumer.

## Scenarios

- A directly exposed edge observes a client connection and passes request-bound
  evidence to an application without transferring management authority.
- A TLS-terminating proxy forwards authenticated facts. The origin distinguishes
  those facts from its own connection to that proxy and from client assertions.
- A legitimate machine client calls a protected API. It uses the application's
  authentication, authorization and resource budget instead of browser gestures.
- An adapter verifies required complete-body integrity before the origin can
  store an upload or execute a transaction.
- A telemetry queue overflows. The operator sees observation loss and total
  attempts rather than interpreting a sampled subset as complete security audit.

## Existing work and the proposed gap

[HTTP Message Signatures](https://www.rfc-editor.org/rfc/rfc9421.html) and
[digest fields](https://www.rfc-editor.org/rfc/rfc9530.html) provide building
blocks for selected components and body commitments. This profile must specify
what is bound and how the receiver verifies actual bytes; it does not invent a
new signature algorithm.

[Robots Exclusion Protocol](https://www.rfc-editor.org/rfc/rfc9309.html)
publishes crawler preferences, not access authentication.
[Trace Context](https://www.w3.org/TR/trace-context/) supports correlation,
not authorization or trusted evidence attribution.

The [WEBBOTAUTH charter](https://datatracker.ietf.org/wg/webbotauth/about/)
addresses participating automated-client authentication to human-facing websites,
with explicit exclusions. The [AIPREF charter](https://datatracker.ietf.org/wg/aipref/about/)
addresses content-use preferences. This proposal investigates the remaining
collector-to-adapter provenance and decision-binding problem; it does not claim
those groups have accepted that problem or this solution.

## Alternatives

| Alternative | Consequence |
| --- | --- |
| Keep evidence inside one process | Fewer transport boundaries; still needs explicit capture completeness and request attribution |
| Forward ordinary headers only | Simple integration, but untrusted claims can be confused with authenticated observations |
| Reimplement decisions in every framework | More deployment choices, but policy and cryptographic behavior can diverge |
| Exchange a versioned evidence contract | Enables shared authority and visible limits, but adds binding, replay, compatibility and privacy obligations |

No new browser API is proposed here. Existing browser events, including trusted
events, do not establish natural-person identity. [DOM](https://dom.spec.whatwg.org/#dom-event-istrusted)
and the [WebDriver working draft](https://www.w3.org/TR/2026/WD-webdriver2-20260702/)
describe why trusted automation events must not become a human-presence guarantee.

## Open questions

The exact serialization, canonicalization, authenticated transport profiles,
streaming integrity, extension negotiation and independent implementation plan
remain unresolved. They block a submission-ready or interoperable status.
