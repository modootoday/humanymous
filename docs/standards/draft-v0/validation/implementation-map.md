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

Public Rust facade source `e696e6a5` adds an explicit optional `adapter-go`
feature exposing `humanymous::adapters::go`; no adapter is enabled by default.
The namespace reuses existing owning contracts. Namespace/consumer checks do
not establish actual Go-host behavior, authenticated observation provenance,
full-product qualification or a generic Draft v0 wire profile. See the
[deployment boundaries](../../../reference/supported-topologies.md).

Delivery validation at source `c3d81a7c` records 107 passing pure tests and
successful local static workflow checks, retained at `86d8d68c`. Node 24 action
pins, Ubuntu 24.04 runner selection and release-note newline handling are
delivery changes; they add no protocol requirement or interoperability result.
Hosted execution and absence of historical annotations remain separate gates.

Core, Platform and Runtime consolidation at `c67d07c0`, `89715c04` and `64f74c7a`
changes package ownership, not the generic Draft v0 protocol. There are 29 workspace
packages plus four standalone qualification-tool packages at the Runtime checkpoint: 33 maintained
packages against an accepted eight-role target. The complete structural totals
are 45 → 39 → 38 → 33 → 21 → 10 → 8; these are package-count stages, not capability
or deployment milestones. Core contextual APIs live
under `crates/sdk/humanymous-core`; Platform's separate optional `testing` and
`windows-object` roles live under `crates/integrations/humanymous-platform`.
Neither a public package role nor a testing feature establishes authenticated
observations, provider availability, registry publication or conformance.

Runtime now groups `gateway`, `authority`, `audit`, `capture`, `network` and
`native_security` under `crates/sdk/humanymous-runtime`; its default feature is
`gateway`, with explicit role features and distinct executable/integration
targets. Its bounded Windows proof covers 177 selected pure cases from a
366-case inventory, check, strict Clippy and formatting. Unselected cases,
native ownership, public ingress and the full browser/product flow remain
separate gates. An older-source browser run is not proof for this consolidation.

Receipt `8e51e558` adds one positive and ten expected same-crate private-access
compiler refusals in the actual sibling modules, plus checks on four source-only
archives and an extracted temporary consumer. The 522 extracted files were
unchanged and the normal/build dependency tree excluded Platform `testing`.
Lock files were excluded and normal Cargo package verification was skipped;
this does not establish registry readiness, native authority or full-suite
execution. A compile-time privacy check does not mint an observation receipt.

Later bounded components retain their own scope: Core source `42ecee7d` records
160 library and 17 integration tests, including 15 SGD cases, without Go bit
parity or model-activation qualification. The two matched CLI check pairs at
`5d68cd39` measure dependency-related check cost, not full builds or Runtime
performance. Lab source `f8aec286` records 44 default Windows cases plus one
separate checkout case after a canonical-input correction; complete tool prehash
and hostile custody remain unqualified. WASM/static checks do not execute a
browser. The isolated Tools/Adapters assembly now has eight members, while the
live workspace remains at 29 plus four standalone tools with no cutover. Its
safe `apache`, `dotnet`, `framework`, `go`, `native-abi` and `nginx` profiles record
73 library units and strict all-target Clippy; four facade Go alias/profile tests
are separate. Neither the package count nor these Rust checks establish actual
hosts, completed exclusive export roles, loaders, packaging, native/cross-OS/CEF/
assistive behavior or full-product conformance. The isolated facade/collector
paths now match the [eight-package developer map](../../../start-here/developer.md#isolated-final-path-map);
live cutover and other integration gates remain open.

Node's isolated host-test build and strict Clippy are compile-only, with separate
positive/negative unsafe-boundary controls; no host, loader or capture producer
is qualified. Meari 0.3 is a separate CPU candidate with 32 model tests and static
checks, not the primary browser or an actual UI/accessibility/CEF result. Neither
checkpoint changes the generic wire model or supplies full conformance evidence.

The staged Tools assembly also implements an isolated offline Core API and CLI
training route whose outputs are unsigned data. Five new offline units, a
165-case Core library regression and two CLI parser units passed; a later Tools
checkpoint records 12 pure maintenance/materializer/result-parser units and
strict Clippy. This is not actual CLI
process execution, authenticated human provenance, Go/FMA parity, model signing,
promotion or activation. The later coordinator path-spelling change remains
separate from the earlier sealed source. Live workspace and Runtime qualification
do not follow from the isolated assembly, and no generic Draft v0 requirement or
new public readiness is established.

The later 2026-11-08 source `3137c905` provides a bounded actual-browser milestone:
49 isolated Lab assertions passed, including sensitive-document and browser/Gate
Referer-v2 checks. One actual browser test was selected from 67 inventoried tests;
66 were filtered and the selected process exited 0. This is not a full browser
corpus or Draft v0 conformance result. The report remained blocked with nine
required entries unavailable, owner exit 2 and full-product/release flags false;
the workflow failed. At that source the full assessor always refuses completion,
so repeating the same assertions cannot qualify it. The labels are not canonical
machine requirement IDs. No private ledger recovery, complete collector parity,
assistive recovery or production deployment is established.

The first pure public-role data parser is implemented at source `2e9934fe`, with
19 scoped Windows parser tests passing and 79 other library tests filtered out,
as retained at `e8f71220`. A separate source `be9eec6d` run, retained at `8c8fc504`,
passed one selected stronger-array-control test with 97 cases filtered out;
formatting, check and strict Clippy passed. This resolves that test-isolation gap
without rerunning the 19-case parser subset or the full 98-case library suite.
Its data declarations
are caller assertions, not
actual provider admission, native ownership, replay/freshness verification or a
Ready producer. This acceptance selects no public listener, endpoint, discovery
or generic Draft v0 wire contract. The existing private-loopback configuration
and historical execution reports keep their own scope.

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

Apache's safe request/replay component contributes a narrower subset of
[before-effect body verification](../protocol/profile.md#p-07-complete-body-verification-before-effects)
and [streaming lifecycle requirements](../protocol/profile.md#p-08-streaming-integrity-lifecycle):
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

Browser transport source declares `BrowserSession`/`BrowserResponse`
and an explicitly selected cookie-free runtime profile. It adds same-origin
Fetch, bounded complete-body processing and status-bound consumed-counter ACK
handling, with application Authorization forwarding in that selected profile.
The Firefox component suite at canonical source `667e98f` passed 66 cases,
including browser API tests with a synthetic Fetch fixture. Native TCP fixtures
at runtime source `adb8e3ed` separately exercised the shared client/server model.
Neither result establishes the positive browser-to-runtime-to-origin join or
qualifies later changes. Deployment evidence gaps remain for
[request binding](../protocol/profile.md#p-03-request-and-decision-binding),
[idempotent delivery](../protocol/profile.md#p-05-idempotent-delivery) and
[before-effect body verification](../protocol/profile.md#p-07-complete-body-verification-before-effects). The
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
