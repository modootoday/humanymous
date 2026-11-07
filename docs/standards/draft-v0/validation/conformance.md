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

Each row links to a named candidate requirement in the proposed profile.
The `P` identifiers refer to that profile, not test priority or external approval.

The [implementation map](implementation-map.md) records current partial coverage.
The [candidate retry/effect lifecycle](../protocol/retry-effects.md) adds proposed
crash-cut, concurrency, status and retention cases. They are acceptance
obligations, not an executed corpus or implemented generic wire.
[Synthetic parser vectors](vectors.md) exercise an existing experimental protocol;
they are not the complete corpus required for the generic Draft v0 profile.

| Requirement | Positive and negative evidence |
| --- | --- |
| [Observation provenance (P-01)](../protocol/profile.md#p-01-observation-provenance) | Direct, forwarded, asserted and absent evidence; forged source |
| [Required capability availability (P-02)](../protocol/profile.md#p-02-required-capability-availability) | Available, partial and missing required capture; route-readiness refusal |
| [Request and decision binding (P-03)](../protocol/profile.md#p-03-request-and-decision-binding) | Correct binding; wrong tenant, route or body, replay and expiry |
| [Connection and stream isolation (P-04)](../protocol/profile.md#p-04-connection-and-stream-isolation) | Correct multiplexing; cross-connection or cross-stream attribution and forged trace IDs |
| [Idempotent delivery (P-05)](../protocol/profile.md#p-05-idempotent-delivery) | Retries and duplicate delivery without double consumption or origin effects, including after restart |
| [Decisions and errors (P-06)](../protocol/profile.md#p-06-decisions-and-errors) | Separate verdicts from malformed input, timeout, unavailable dependency and unknown version |
| [Complete-body verification before effects (P-07)](../protocol/profile.md#p-07-complete-body-verification-before-effects) | Finite bodies before effects; partial, oversized, compressed and multipart inputs |
| [Streaming integrity lifecycle (P-08)](../protocol/profile.md#p-08-streaming-integrity-lifecycle) | Explicit stream authorization, partial-stream failure and lifecycle cases |
| [Decision containment and revalidation (P-09)](../protocol/profile.md#p-09-decision-containment-and-revalidation) | Mutation allowlists, final-route changes, no public administration and no credential leakage |
| [Audit independence and visible loss (P-10)](../protocol/profile.md#p-10-audit-independence-and-visible-loss) | Sampling, queue overflow, disk failure and unsampled outcome denominators |

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

## Evidence record boundaries

This is informative reporting guidance, not a selected generic wire schema or
an additional conformance class. A comparable result record identifies:

| Record group | Information needed to interpret the result |
| --- | --- |
| Claim | Requirement IDs, conformance class, exact claimed capability and explicit exclusions |
| Source and artifact | Source revision, dirty-source commitments if applicable, executable/image/package hashes and build-input identity |
| Execution | Exact run/job/attempt, command, OS/architecture, host/framework/browser versions and configured producer profile |
| Observation provenance | Actual production input versus test-only synthetic evidence, asserted versus authenticated observations, and any unavailable required plane |
| Outcomes | Attempts, verdicts, execution errors, skips, cancellations and observation loss, with the denominator and collection boundary stated |
| Origin effects | Observed application invocations and side effects, ambiguous outcomes and the mechanism used to observe them; not inferred from a decision ACK |
| Review and retention | Expected/observed result per case, evidence location/hash, reviewer scope and public-safe summary versus restricted raw evidence |

Separate parser acceptance, native host loading, actual authority decisions,
origin effects and human/assistive-technology outcomes. A synthetic ALLOW can
exercise an origin path without proving a production browser observation would
receive ALLOW. A CHALLENGE response does not show that recovery was available
or that a person completed it. Shared helper use does not establish independent
implementation, and a hosted-runner label does not establish actual assistive
technology or independent organizational custody.

Retain failed attempts when a rerun passes and bind both to their exact sources.
A suite report also needs a frozen expected inventory and an execution ledger.
Record expected, started, completed, passed, failed and not-run cases separately;
an early runner failure leaves later cases not run, even when every assertion
that actually executed passed. Compilation and test discovery do not count as
execution. Required host/process cases cannot become passing coverage by being
ignored, removed from the denominator or relabeled as helper tests.

A current-source claim identifies the complete relevant source and dependency
closure, not just the top-level manifest, lockfile or selected changed files.
When only a subset was hashed or inputs changed during validation, report the
limited scope and retain the component result without claiming an immutable
integrated candidate. Link separate build, execution and coverage outcomes;
one successful stage does not erase a later failure.

Name the byte representation of every source commitment. Exact executed working
bytes, canonical LF text and the stored Git blob are different observations.
For example, a CRLF checkout can have the same Git blob identity as an LF
checkout while their executed-file SHA-256 values differ. Record any explicit
normalization and verify its correspondence; do not silently replace an
executed-byte commitment with a canonical one. Binary inputs remain byte-exact.

Build evidence from the tested boundary upward: pure module, component contract,
real host integration and complete deployment. Each layer names its actual
inputs and observations. Passing a lower layer supports that boundary only;
shared helpers, simulated runtime completion and ignored process tests do not
supply the missing higher-layer execution. This reporting structure does not
require unrelated workspace or toolchain documentation to be hashed for a
pure-module claim, and does not waive relevant dependency or build inputs.

A skipped job is not a passing case. Distinguish a test assertion failure from
an environment, authorization or dependency failure without discarding either.
If raw evidence was not retained or cannot be retrieved, record that limitation;
a hash alone does not allow its content or asserted result to be inspected.
Public artifacts contain synthetic or reviewed sanitized data, never operational
credentials or private visitor traces. Pages rendering/link/access checks describe
the documentation artifact, not product protocol or recovery conformance.

## Current status

| Gate | State |
| --- | --- |
| Public v0 problem and requirements model | Authored; review in progress |
| Retry/effect lifecycle semantics | Proposed; concrete wire, durable implementation and crash-cut evidence pending |
| Experimental adapter parser bytes/vectors | Three fixed corpora, 59 cases matched in Windows component execution; single shared implementation, separate from Draft v0 wire |
| Exact schema and canonical wire vectors | Pending |
| Independent implementations and interoperability | Pending |
| Protocol-specific browser/native/Docker pilots | Pending |
| External horizontal reviews | Pending |
| Submission-ready package or external submission | Pending |

The final report must name exact source revisions, tested tuples, capabilities,
failures and residual limits. Passing a source-text checker alone does not
qualify an implementation or prove rendered GitHub Pages accessibility.
