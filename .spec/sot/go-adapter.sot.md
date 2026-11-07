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

## Host contract

net/http, chi, Gin and Echo use standard HTTP handlers but qualify independently.
Use the original RequestURI rather than a parsed/reconstructed query when
comparing requests. Read complete bounded entity bytes before parser/effects;
context cancellation and body closure cannot authorize a delayed invocation.
Fiber/fasthttp use pooled contexts and borrowed byte slices. Never retain these
across handler return or reuse; a net/http conversion is not equivalent coverage.
Gate forwards HTTP to the external origin; it does not retain Go objects or
promise in-process middleware cancellation, ResponseWriter interfaces or hijack.

crates/go-adapter validates deployment through actual runtime Configuration,
then transfers it to the shared composition root. No proxy, scoring, crypto,
nonce, audit or authority implementation is copied. Network isolation remains
a separately measured deployment requirement, never inferred from loopback.

## Qualification

Rust receipt validation requires exact host/version/Go-version/OS/architecture,
source/image/external-fixture/configuration/engine bindings, hosted isolated Docker
execution, complete unique eight-case accounting,
positive executed assertions, no failures and independent origin effect counts.
Binary allow and response semantics each require one effect; dependency loss,
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

Root reports two pure unit tests passed after workspace registration (34 members).
Detailed source-bound root artifact is pending; scoped Clippy remains in progress.
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
application mutations. Missing image/toolchain/full-fixture hashes stay null
until the hosted owner measures them; this file cannot qualify itself.

Official contracts: https://pkg.go.dev/net/http ; https://github.com/go-chi/chi ;
https://gin-gonic.com/en/docs/ ; https://echo.labstack.com/ ;
https://docs.gofiber.io/next/extra/internal/ ;
https://pkg.go.dev/github.com/valyala/fasthttp .
