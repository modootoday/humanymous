---
layout: doc
title: Conformance evidence and validation status
description: The tests and independent evidence required before this draft can claim interoperability.
permalink: /standards/draft-v0/validation/conformance/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---

# Conformance evidence and validation status

**Diátaxis quadrant:** Reference. **Audience:** implementers and independent evaluators.

**Status:** independent project draft. No implementation has been qualified
against a complete frozen version of this profile. Existing product primitive
tests are not proof of this proposed protocol's interoperability.

## Conformance classes

| Class | Obligation |
| --- | --- |
| Producer | Correct observation source, scope, completeness, version and limits |
| Decision service | Authenticated input binding, explicit errors, policy revision and request-bound decisions |
| Enforcing adapter | Final-route/origin binding, before-effect validation, bounded processing and no duplicate effect |
| Deployment profile | Actual TLS termination/capture boundary, protocol support and local-management isolation |

Optional capabilities must have declared prerequisites and tests. An unsupported
required capability blocks that profile's qualification. Success on one operating
system or runtime does not qualify every combination.

## Required corpus

| Requirement | Positive and negative evidence |
| --- | --- |
| P-01–P-02 | Direct, forwarded, asserted, partial and absent evidence; forged source and missing required capture |
| P-03–P-05 | Correct binding and duplicate delivery; wrong tenant/route/body/stream, replay, expiry and restart |
| P-06 | Separate verdicts from malformed input, timeout, unavailable dependency and unknown version |
| P-07–P-08 | Complete finite bodies before effects; partial/oversized/compressed/multipart and streaming lifecycle cases |
| P-09 | Mutation allowlists, final-route changes, no public administration and no credential leakage |
| P-10 | Sampling, queue overflow, disk failure and unsampled outcome denominators |

Vectors must specify byte-exact inputs/outputs, supported version, expected error
and source revision. They must include absent/null/empty/duplicate/unknown fields,
non-UTF8 body bytes, Unicode boundaries, clock skew and finite resource limits.
Two implementations that share the same faulty helper are insufficient evidence
of independent interpretation.

## Deployment and user controls

Run native Windows/macOS/Linux pilots for advertised native artifacts and the
isolated Docker full-deployment suite for end-to-end authority. Include real
Chrome/Firefox/WebKit, ordinary machine clients, webhooks, shared/cloud networks,
keyboard and assistive-technology controls. Test HTTP, supported remote procedure
calls, WebSocket and event streams for byte, trailer, cancellation and backpressure
preservation. No test may probe an unrelated third-party service.

Report attempts, ALLOW, CHALLENGE, DENY, execution errors, skips and observation
loss separately. CHALLENGE is not a successful human ALLOW. Do not omit errors
from detection denominators or present named-provider blocking as measured
without a corresponding authorized reproducible experiment.

## Current status

| Gate | State |
| --- | --- |
| Public v0 problem and requirements model | Authored; review in progress |
| Exact schema and canonical wire vectors | Pending |
| Independent implementations and interoperability | Pending |
| Protocol-specific browser/native/Docker pilots | Pending |
| External horizontal reviews | Pending |
| Submission-ready package or external submission | Pending |

The final report must name exact source revisions, tested tuples, capabilities,
failures and residual limits. Passing a source-text checker alone does not
qualify an implementation or prove rendered GitHub Pages accessibility.
