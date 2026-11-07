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
| [Observation provenance (P-01)](../protocol/profile.md#p-01-observation-provenance) | Shared adapter parser validates declared header mode and rejects fabricated capture; collector reads browser APIs; a separate same-transport TLS/H2 capture candidate exists | Actual ingress qualification, original-client hop ownership and report-to-session provenance, not a client assertion |
| [Capability availability (P-02)](../protocol/profile.md#p-02-required-capability-availability) | Authority/runtime reject required unavailable planes; their capability profiles differ | Every claimed deployment tuple and actual producer selection; no inference from a parser or framework name |
| [Request binding (P-03)](../protocol/profile.md#p-03-request-and-decision-binding) | IPC scope/route checks and separate runtime session/integrity canonicalization exist | Generic Draft v0 byte/authentication/negotiation contract and cross-implementation binding |
| [Connection isolation (P-04)](../protocol/profile.md#p-04-connection-and-stream-isolation) | Adapter connection IDs remain assertions; normalized profiles declare losses; capture source keeps HTTP/2 state per connection and observations per stream | Actual capture/runtime request binding and multiplexed tenant-bound pilots; a pure parser is not authenticated ingress |
| [Idempotent delivery (P-05)](../protocol/profile.md#p-05-idempotent-delivery) | Authority refuses duplicate nonces; runtime counters/audit recovery have bounded pilots; pure collector Session allows one pending request and verifies consumed-counter ACKs | Actual Fetch/body/abort/server-wire integration plus generic retry/reservation/retention/epoch rules; lost response and ambiguous-origin evidence |
| [Decisions and errors (P-06)](../protocol/profile.md#p-06-decisions-and-errors) | Typed service errors are distinct from verdicts; unsupported inputs fail closed | Independent error compatibility and complete negotiated-version behavior |
| [Before-effect integrity (P-07)](../protocol/profile.md#p-07-complete-body-verification-before-effects) | Complete-body parser validation and bounded runtime audit-before-origin path exist | Full qualified collector-to-origin flow, origin-bypass isolation and actual human controls |
| [Streaming lifecycle (P-08)](../protocol/profile.md#p-08-streaming-integrity-lifecycle) | Experimental adapter capture/stream fields remain null; arbitrary upload/upgrade integrity is unsupported | A separately specified streaming protocol and cancellation/backpressure/side-effect evidence |
| [Containment (P-09)](../protocol/profile.md#p-09-decision-containment-and-revalidation) | Decision response has a strict small field set; public namespaces do not authorize administration; runtime source separates observer reads from an optional resource-only operator | Actual listener/peer/permissions, shutdown and resource-write pilots, independent dual approval and final-route mutation pilots |
| [Visible loss (P-10)](../protocol/profile.md#p-10-audit-independence-and-visible-loss) | Bounded buffers, authenticated WAL/log, local export and witness components exist; native verification scope distinguishes omitted HMAC and absent anchors | Generic loss/quality wire model, qualified sink/process-isolation evidence and unsampled full denominators; component proof scope does not establish pre-ingestion completeness |

The [wire experiment](../protocol/implementation-experiment.md) and
[parser vectors](vectors.md) document concrete existing interfaces separately
from the unresolved generic profile. Reuse of one shared Rust parser is good
implementation consistency, but is not independent interpretation evidence.

Runtime library source now provides an optional buffered TLS/H1 ingress through
`ServiceHost::bind_tls`. The captured request retains its handshaken connection
owner; the standalone executable still selects plaintext ingress. Browser
profiles and non-HTTP/1.1 ALPN are refused on this TLS path, and HTTP/2 or collector
requirements remain unready. This source change does not establish current native
ingress qualification or original-client visibility across a re-terminating proxy.

At source `8132117`, the [Linux MSRV job](https://github.com/modootoday/humanymous/actions/runs/42355168729/job/128359709010)
recorded an authority subset of 30
passing tests and two ignored entries: a required privileged other-user case
and its helper. Named results cover response binding, normalized producer
selection, controlled-parent sealed startup, native capacity and generic
cancellation/drain. This is authority component evidence, not actual Python,
Ruby, Apache or ASP.NET Core server/ABI qualification, independent custody or
complete workspace coverage. The separately authored
[25-case normalized corpus](vectors.md#normalized-host-profile-corpus-execution)
has since passed its Windows parser component execution. That result does not
qualify later authority changes or close the ignored privileged case.

The native authority test source also distinguishes ordinary current-user child
IPC from the explicit Linux other-user socket test. That required case and its
helper are ignored in default runs and need privileged, deployment-owned Docker
execution; a default green run cannot discharge them. Windows-only PID, pipe
DACL and nonreading-client cases require actual Windows execution. The normalized
producer loop uses constructed envelopes and the shared client, not real ASGI,
WSGI, Rack, httpd or Kestrel server callbacks. Its sealed-startup fixture checks
same-parent custody locking and key continuity, not independent witness custody.

Apache's safe request/replay component contributes a narrower P-07/P-08 subset:
exact-length bounded input, original-budget checks and cancellation-aware replay.
It does not authorize a request, specify streaming integrity or establish that
native server hooks run before every side effect. Generated ABI-input receipts
and default builds are separate from real httpd module execution. See the
[native host boundaries](../../../reference/supported-topologies.md#other-native-host-candidates).

The Node bridge now shares configuration, envelope projection and admission
with the native framework adapters. Their producer profiles still declare
different observation losses; shared implementation does not qualify each host
or establish interoperability between independent implementations. Earlier
component results do not cover this refactor. The
[Node reference](../../../reference/node-adapter.md) records the current interface
and optional host declarations.

The runtime management source exposes observer status/settings reads and an
optional separately credentialed resource operator, with native-peer and exact
installation-scope checks. The operator's source stages and journals resource-only
CAS changes and invalidates volatile sessions; it does not change scoring or add
destructive controls. A Unix component fixture has exercised native IPC,
credential separation, witnessed CAS, stale-revision refusal, session invalidation,
capacity and trusted-head restart. Its durable witness is a same-process fixture,
not an independent deployment; these Unix-only lifecycle cases do not establish
Windows service qualification. The
[operations reference](../../../reference/on-call-quick-reference.md#experimental-native-status-service)
separates those observations from the remaining deployment and full-product gates.

At runtime source `adb8e3ed`, ordinary-user component runs passed 46 library cases
plus one native-process case on Windows, and 50 library cases plus one
native-process case on Unix, with no failed, ignored or filtered cases. The
inventories include pure models and owned loopback/IPC/custody/process fixtures;
they are not 47 or 51 independent deployed product flows. The four additional
Unix cases do not establish Windows management or service behavior.

At audit source `174e5f21`, the Windows all-targets package run passed 110 cases
with zero failures. Five explicitly ignored child helpers and two zero-test
binaries were excluded from that count; the required parent cases passed.
This mixes pure, filesystem, parity and native-child components. It does not
establish a genuine hosted OS matrix, independent witness/retainer deployment,
production audit-before-apply or full-product qualification. These frozen results
do not automatically qualify later source changes or the generic Draft v0 wire.

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
