---
layout: doc
title: Security, privacy and inclusive access
description: Review boundaries for the independent evidence interoperability draft.
permalink: /standards/draft-v0/reviews/security-privacy/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---

# Security, privacy and inclusive access

**Diátaxis quadrant:** Reference. **Audience:** security, privacy, accessibility and internationalization reviewers.

**Status:** independent project draft and initial self-review. No external review,
legal compliance certification or standards-body approval is claimed.
The profile cannot prove that every authenticated request was made by a person
or that a real-browser user has benign intent.

This page is an informative review checklist. Candidate protocol requirements
belong to the [proposed profile](../protocol/profile.md#requirement-language);
this review does not add a separate set of conformance requirements.

## Threat boundaries

| Boundary | Required review and negative test |
| --- | --- |
| Client to collector | Forged provenance, solved flags, headers, trace IDs and malformed/oversized telemetry |
| Collector to decision service | Producer impersonation, replay, wrong tenant/body/stream/route, expiry and key rotation |
| Decision to origin effect | Route mutation, double execution, body validation after a side effect, retries after partial forwarding |
| Key discovery | Private/loopback/link-local targets, redirects, DNS changes, large/churning key documents and resource exhaustion |
| Public to administration | Separate listeners/credentials, forwarding spoof, application server-side request forgery (SSRF), browser localhost attacks and unauthorized local processes |
| Audit to analytics | Sampling, dropped queues, disk failure and misleading incomplete measurement denominators |

Review that administration is absent from the public adapter namespace, as
described by [decision containment](../protocol/profile.md#p-09-decision-containment-and-revalidation). Locality is
not authentication: shared container loopback and default named-pipe permissions
can cross unintended trust boundaries. Service identity, restrictive permissions,
remote-client rejection where needed, authorization and sensitive-operation
dual control remain separate requirements.

Sources: [Windows pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights),
[Unix sockets](https://www.man7.org/linux/man-pages/man7/unix.7.html),
[Kubernetes Pods](https://kubernetes.io/docs/concepts/workloads/pods/).

## Data minimization

Raw keystrokes, passwords, form values, clipboard contents, full document trees
and screenshots are outside default collection. Browser observations should be
purpose-limited aggregates with explicit retention and access controls.
Connection or body commitments can still be linkable or personal data; an
ordinary hash of a small input space is not anonymity.

Separate tenant/purpose pseudonyms, key lifetimes, deletion and audit retention.
Document what must survive to verify integrity and what can be erased. Do not
publish operational visitor traces or identifiers as conformance examples.
The [W3C Privacy Principles](https://www.w3.org/TR/privacy-principles/) are a
Statement, not a compliance certificate for an implementation.

## Accessibility and human recovery

Verify that accessible recovery does not depend on pointer richness, typing
speed or rapid completion. See the [accessible recovery contract](../../../help/challenge-accessibility.md).
Include keyboard-only, assistive
technology, reduced-motion, mobile and slow-user controls. A low event count
does not establish abuse. Authentication and resource controls must provide
legitimate API-client paths without injecting browser challenge markup into API
or streaming responses.

Recovery must explain the outcome, preserve safe user progress and avoid an
unredeemable challenge loop. Screen-reader and keyboard testing must be measured
in actual supported browsers; a written accessibility intention is insufficient.

## Internationalization

Define exact byte/Unicode normalization boundaries before hashing or signing.
Include decomposed/composed characters, non-Latin text, right-to-left content,
Unicode paths and internationalized host names in vectors. Locale-dependent
number/time formatting must not change canonical bytes. Display text and machine
identifiers have different normalization and escaping responsibilities.

## Review status

This is an initial self-review checklist. External security/privacy/accessibility/
internationalization review requests, responses and resolutions remain pending.
Use the [security/privacy questionnaire](https://www.w3.org/TR/security-privacy-questionnaire/)
and [international specification guidance](https://www.w3.org/TR/international-specs/)
when preparing that review; do not mark those reviews complete from this page.
