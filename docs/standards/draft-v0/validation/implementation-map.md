---
layout: doc
title: Requirements and implementation evidence map
description: Trace candidate requirements to current components without conflating their scopes.
permalink: /standards/draft-v0/validation/implementation-map/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---
# Requirements and implementation evidence map

**Diátaxis quadrant:** Reference. **Audience:** implementers and independent evaluators.

**Status:** informative traceability map for a developing project draft.
No row claims complete Draft v0 conformance. Source and fixture digests belong
in the source-bound execution report; an older passing component run does not
qualify a changed binary.

| Candidate requirement | Current subset | Evidence boundary still needed |
| --- | --- | --- |
| [Observation provenance (P-01)](../protocol/profile.md#p-01-observation-provenance) | Shared adapter parser validates declared header mode and rejects fabricated capture; collector reads actual browser APIs | Authenticated original-client capture and report-to-session provenance, not a client assertion |
| [Capability availability (P-02)](../protocol/profile.md#p-02-required-capability-availability) | Authority/runtime reject required unavailable planes; their capability profiles differ | Every claimed deployment tuple and actual producer selection; no inference from a parser or framework name |
| [Request binding (P-03)](../protocol/profile.md#p-03-request-and-decision-binding) | IPC scope/route checks and separate runtime session/integrity canonicalization exist | Generic Draft v0 byte/authentication/negotiation contract and cross-implementation binding |
| [Connection isolation (P-04)](../protocol/profile.md#p-04-connection-and-stream-isolation) | Adapter connection IDs remain assertions; normalized profiles declare losses | Original raw connection/stream capture and multiplexed tenant-bound pilots |
| [Idempotent delivery (P-05)](../protocol/profile.md#p-05-idempotent-delivery) | Authority refuses duplicate nonces; runtime counters/audit recovery have bounded pilots; pure collector Session allows one pending request and verifies consumed-counter ACKs | Actual Fetch/body/abort/server-wire integration plus generic retry/reservation/retention/epoch rules; lost response and ambiguous-origin evidence |
| [Decisions and errors (P-06)](../protocol/profile.md#p-06-decisions-and-errors) | Typed service errors are distinct from verdicts; unsupported inputs fail closed | Independent error compatibility and complete negotiated-version behavior |
| [Before-effect integrity (P-07)](../protocol/profile.md#p-07-complete-body-verification-before-effects) | Complete-body parser validation and bounded runtime audit-before-origin path exist | Full qualified collector-to-origin flow, origin-bypass isolation and actual human controls |
| [Streaming lifecycle (P-08)](../protocol/profile.md#p-08-streaming-integrity-lifecycle) | Experimental adapter capture/stream fields remain null; arbitrary upload/upgrade integrity is unsupported | A separately specified streaming protocol and cancellation/backpressure/side-effect evidence |
| [Containment (P-09)](../protocol/profile.md#p-09-decision-containment-and-revalidation) | Decision response has a strict small field set; public namespaces do not authorize administration | Full operational management, independent dual approval and final-route mutation pilots |
| [Visible loss (P-10)](../protocol/profile.md#p-10-audit-independence-and-visible-loss) | Bounded buffers, authenticated WAL/log, local export and witness components exist; native verification scope distinguishes omitted HMAC and absent anchors | Generic loss/quality wire model, qualified sink/process-isolation evidence and unsampled full denominators; component proof scope does not establish pre-ingestion completeness |

The [wire experiment](../protocol/implementation-experiment.md) and
[parser vectors](vectors.md) document concrete existing interfaces separately
from the unresolved generic profile. Reuse of one shared Rust parser is good
implementation consistency, but is not independent interpretation evidence.

Working browser transport source now declares `BrowserSession`/`BrowserResponse`
and an explicitly selected cookie-free runtime profile. It adds same-origin
Fetch, bounded complete-body processing and status-bound consumed-counter ACK
handling, with application Authorization forwarding in that selected profile.
These changed paths have not been built or exercised in the current review;
source wiring does not close the P-03, P-05 or P-07 evidence gaps. The
[browser API reference](../../../reference/browser-collector.md#browser-transport-follow-through-under-source-review)
records the current bounds and cancellation semantics. This implementation
experiment does not freeze a generic Draft v0 browser wire profile. An ACK
records token consumption; it does not authenticate response-body bytes,
prove an ALLOW verdict or establish that an origin effect committed.

The laboratory browser and Firefox collector checks are different components.
Neither a native window nor a WASM build demonstrates full browser readiness,
authenticated observation receipts, a complete browser sandbox policy or natural-
person presence. Actual keyboard, screen-reader and accessible recovery outcomes
remain separately measured gates; synthetic zero-motor reports are narrower.

Independent implementation, external horizontal review, contribution/rights
clearance and submission-ready venue packaging remain pending. No review request,
submission, registry assignment or adoption is implied by this map.
