---
id: sot:root/go-adapter
title: Go HTTP Gate compatibility
status: active
created: 2026-11-07T07:28:00Z
updated: 2026-11-07T07:28:00Z
verify: manual
rel:
  governs: [crates/go-adapter/**, adapters/go/**]
  depends_on: [sot:root/adapter-ecosystem, sot:root/runtime-application, sot:root/rust-migration]
---

# Go HTTP Gate compatibility

The existing lang.go-http decision governs: external Go HTTP origins consume
the shared Rust Gate; no authored Go SDK, foreign templates or unsafe ABI glue.
The integration root accepted this direction on 2026-11-07. No new producer
profile is needed: the actual Gate observes ingress and alone owns security.

## Public facade migration contract

Root approved optional feature `adapter-go` and public Rust access through
`humanymous::adapters::go::{Deployment, Host, Configuration, Rejection}`. The
internal target package is `humanymous-adapter-go`; its ordinary Rust library
artifact remains unique, with no Go SDK/module/cgo or common native DLL.
Configuration and Rejection are re-exported from the existing owning runtime,
without a second implementation or conversion layer.

Root's facade/closed manifest predecessor is e696e6a5. GO-FACADE-01 prepares the
leaf package identity `humanymous-adapter-go` and external test import
`humanymous_adapter_go` for Root's single atomic Cargo/lock/consumer commit.
The public namespace re-exports the whole existing leaf, including probe and
scenario APIs; `evaluate` remains `scenario::evaluate`. Lab owns the public
consumer imports and Delivery owns current workflow/policy selectors. No owner
may commit or compile the intermediate mixed graph. Source preparation and the
earlier facade component checks do not qualify the renamed runtime candidate.
The atomic graph is now committed at c858b1dc40d60c2d22f10411531a8b72b70aea63.
Local Windows leaf10+integration1 and scoped strict Clippy passed, alongside
facade component tests and Delivery checks. Evidence:root/20261107192637 records
the exact counts and limits:3004 snapshot records are2917 old target outputs
plus87 non-target files, not a full transitive/tool/read closure. The Windows
actor check excludes Unix Go code; that local result did not establish actual
Linux compilation or fresh hosted Docker Go8 at the renamed source.
Fresh scoped tests, strict Clippy, actual public consumer and hosted Docker Go8
now passed at renamed source a19e380218b87ea77c2f0d48d9fdd7edfa9fd8cb in genuine
hosted run42406688473. Linux preflight passed leaf10+integration1, facade2+2+2,
lab34, Delivery110, actual Unix actor compilation and scoped strict Clippy/policy.
Actual isolated Docker evaluated/passed8 with40 checks,0failed/not-run/unknown,
owner0, valid receipt, unchanged source and observed cleanup. Evidence:root/20261107194718
records this bounded public-consumer qualification, raw limits and0 current
annotations/Node20/Node24 warnings. The private ledger validates the actual tuple,
but its individual runtime values are not exported here; full product/release
remain false. Preserve original c321 Go8 receipts and raw commands as historical
exact-source evidence, without relabelling their result.

## Host contract

net/http, chi, Gin and Echo use standard HTTP handlers but qualify independently.
Use the original RequestURI rather than a parsed/reconstructed query when
comparing requests. Read complete bounded entity bytes before parser/effects;
context cancellation and body closure cannot authorize a delayed invocation.
Fiber/fasthttp use pooled contexts and borrowed byte slices. Never retain these
across handler return or reuse; a net/http conversion is not equivalent coverage.
Gate forwards HTTP to the external origin; it does not retain Go objects or
promise in-process middleware cancellation, ResponseWriter interfaces or hijack.

| Host | Origin transport contract | Host-owned lifetime boundary |
| --- | --- | --- |
| net/http | Standard Handler/Request/ResponseWriter | Body close and Request.Context cancellation; optional writer interfaces are not supplied by Gate |
| chi | net/http router with nested middleware/mounts | Preserve final method/target inventory across mounts; request context cancellation is separate from completing origin effects |
| Gin | net/http engine with Next/Abort handler chain | gin.Context is pooled; background work must not retain the original context; a copy does not mint authorization |
| Echo | net/http engine with framework handler/error chain | Echo contexts are pooled; do not retain pooled context across request return or use it concurrently |
| Fiber | fasthttp handler/context chain | Pooled context and borrowed entity slices require copying for retention; not the net/http body stream contract |
| fasthttp | RequestCtx and parsed request/response | Borrowed Peek/body slices and RequestCtx do not survive handler reuse; no raw wire/casing capability follows |

Rust Host metadata distinguishes pooled framework contexts (Gin/Echo included)
from fasthttp-style borrowed entity slices (Fiber/fasthttp). These are integration
premises, not measured host capabilities or extra authorization predicates.

crates/go-adapter validates deployment through actual runtime Configuration,
then transfers it to the shared composition root. No proxy, scoring, crypto,
nonce, audit or authority implementation is copied. Network isolation remains
a separately measured deployment requirement, never inferred from loopback.

## Qualification

Rust receipt validation requires exact host/version/Go-version/OS/architecture,
source/image/external-fixture/configuration/engine bindings, hosted isolated Docker
execution, complete unique eight-case accounting,
positive executed assertions, no failures and independent origin contact counts.
The initial profile is bounded-http-8-case/2. Receipts require origin_contacts,
never substitute unobserved application mutations under an effects field.
Positive cases additionally require handler_response_verified=true, backed by
actual binary echo or the HEAD/cookie response. Negative cases require false
because there must be no protected handler response.
The owned probe module checks the pinned fixture's dumped request line/query and
binary entity without reconstructing text, and checks HEAD200/empty entity plus
two separate Set-Cookie values. These are fixture assertions over the lab's
already-decoded outer response, not a new HTTP ingress parser or authorization
service. A successful assertion alone cannot prove an actual upstream process,
network isolation or the authenticity of caller-supplied observations.
The scenario evaluator accepts actual decoded response pairs/entity and optional
independent contact count, plus explicit host, pre-handoff cancellation, deadline,
half-close and kernel-denial observations. A missing counter is an error, never
zero. Every required observation must match its case before producing Outcome.
422 cases additionally require the exact shared required_capability response;
incomplete input requires truncated_body400 and a measured write half-close.
The upgrade case requires the actual http_framing400 response. Cancel/bypass
cases require no HTTP response, not a timeout relabelled as a successful denial.
The lab retains all eight observations/errors and builds a receipt only from
eight real measured successes. Evaluator errors preserve attempted check counts
with static diagnostic codes; raw request/response bytes are not Debug-logged.
Binary allow and response semantics each require one contact; dependency loss,
cancellation, incomplete body, forged provenance, bypass and upgrade require zero.
The forged-provenance case uses a route requiring unavailable authenticated
capture; ordinary harmless forged headers may instead be stripped on ALLOW.
This is a bounded initial corpus; it does not establish all framework behavior.
Receipts are accounting inputs, not cryptographic attestations. Trusted CI and
independent lab witness must supply them. Synthetic units cannot qualify a host.

## Critique reflected

P0 bypass: localhost alone is not isolation; require actual namespace deny witness.
P0 overclaim: no safe Rust program can implement Go method sets without a host
bridge; universal Gate profile is distinct from native SDK support.
P1 receipt laundering: reject sibling host, stale source, component/mock, missing,
duplicate, zero-assertion, failure and nonzero negative effect cases. A claimed
isolated_docker boolean alone does not prove topology; retain lab artifacts.
P1 response mutation: duplicate Set-Cookie, HEAD/bodyless handling need real host
tests, not receipt unit success. Stream/upgrade support is absent until separately
designed and measured. CHALLENGE is not a completed recovery experience.
No detection or a11y changes; fail-closed and T4 limits remain unchanged.

## Evidence and remaining gates

The Rust Gate Go integration adapter has actual bounded net/http qualification
at source c3211e27ba941f3f11809e420d07d61ececad623, genuine GitHub hosted Linux
run42375310198/product job128412147497. This is Rust integration with an untouched
external Go service, not work on the retiring legacy Go security core.
Evidence:root/20261107132453 retains the safe original runner projection:
8 scheduled/evaluated/passed,0 failed/not-run/unknown-assertion cases,40 executed
owner checks, owner exit0, valid receipt, owned cleanup and unchanged source.
Same-source preflight passed Go10/lab20/delivery74 units, Unix actor compilation,
selected all-target strict Clippy and workflow policy. No ignored cases were
counted as successes. Earlier438/448 image-build failures retain8NOT_RUN each;
15f retains8 attempted writer-mount failures with0evaluated and unknown counts.

The lab's typed bounded requirement verifies the real source/image/fixture/
configuration/engine tuple internally. The public projection exports source,
case accounting and private ledger digest
5ab341d113866c367fff963681289978e324891af070496bd05821f8564240a4;
it does not export individual runtime-image or other private tuple values.
Do not substitute the custody-layout probe image for the actual runtime image,
or claim independent reconstruction of the private tuple from public logs.
This qualifies only bounded-http-8-case/2 at that immutable source. The recipe's
unmeasured template fields remain null. Current metadata updates do not rerun
or qualify a different binary. Other five hosts, native Go SDK, browser/human
provenance, recovery,204/304,streaming and post-origin rollback remain outside it.
Full-product qualification and release readiness remain false; the separate
--require-full gate is unchanged. This source-bound evidence supersedes older
pending-hosted wording only for the measured bounded profile.

Root's two pure unit tests for307e9cc remain a separate older observation.
User-authorized resumption obtained explicit B and verified the three contract
pure units (3passed, zero failed/ignored/measured/filtered), plus locked scoped
all-target Clippy with denied warnings. Evidence:root/20261107075922 records the
actual runner, stable owned source hashes and one concurrent runtime test-file
change in the enumerated208-file subset. It is not whole-workspace qualification.
That result predates the probe module's three added tests; the six-unit source
requires its own assigned-slot verification and cannot inherit the older pass.
Later source c959562, including the lab-requested evaluator, passed all ten pure
units and locked all-target Clippy on explicit A. Evidence:root/20261107083740
records zero drift in216 enumerated files and exact graph/owned hashes, while
retaining the limitation that newly transitive capture/challenge source was not
in the snapshot. It does not establish a complete immutable compiled closure.
Direct Rust formatting and scoped spec check passed with zero errors/warnings.
Hosted external fixture execution, actual Go versions, image digests, isolation
witness and product e2e are NOT_RUN. No host tuple is advertised as qualified.
adapters/go/upstream-fixtures.json pins six untouched upstream candidate
entrypoints using full Git revision and actual decoded file SHA256. These are
entrypoint hashes only, not complete build/dependency/fixture artifact hashes.
Greeting-only examples cannot satisfy the required complete-body corpus.
The sole initial hosted contract is adapters/go/pilot-contract.json. It selects
the untouched net/http go-httpbin candidate, exact binary dump endpoint and a
single HEAD response with two cookies. Other hosts, 204/304 and streams remain
outside that initial measurement. Cancellation means before complete body and
origin handoff; no rollback after an effect is promised. An independent contact
counter proves absence of origin contact conservatively; positive handler echo
must separately establish actual Go execution. Contacts are not measured
application mutations. Compiler input now pins an officially sourced
Go1.26.10Alpine3.24 index and Linuxamd64 digest; fixture go.mod requires1.26.0,
so legacy1.25 is insufficient. Metadata selection is not image execution or
signature qualification. Full fixture/runtime image hashes stay null until the
hosted owner measures them; this file cannot qualify itself.

adapters/go/Dockerfile.fixture is a hosted-only untouched-upstream build recipe.
Its build context is the verified external fixture checkout, never this project.
It uses the selected exact compiler digest, CGO disabled and GOTOOLCHAIN=local,
and produces a static Linuxamd64 binary in an export stage. Exec-form Go build
authors no Go wrapper or shell script. The lab owns network-none build invocation,
artifact hashing, final witness/backend image, separate namespaces and cleanup.
No local Docker build or runnable image qualification follows from this recipe.

Official contracts: https://pkg.go.dev/net/http ; https://github.com/go-chi/chi ;
https://gin-gonic.com/en/docs/ ; https://echo.labstack.com/ ;
https://docs.gofiber.io/next/extra/internal/ ;
https://pkg.go.dev/github.com/valyala/fasthttp .
Gin context lifetime: https://gin-gonic.com/en/docs/middleware/goroutines-inside-a-middleware/ .
Echo context pool: https://github.com/labstack/echo/blob/master/echo.go .
