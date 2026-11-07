---
layout: doc
title: Candidate profile terminology
description: Shared meanings for provenance, binding, capability and evidence maturity in the developing proposal.
permalink: /standards/draft-v0/protocol/terminology/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---
# Candidate profile terminology

**Diátaxis quadrant:** Reference. **Audience:** implementers and protocol reviewers.

**Status:** terminology for the independent developing project profile. These
definitions explain its candidate requirements; they do not establish external
adoption or an implemented generic wire contract.

| Term | Meaning in this profile |
| --- | --- |
| Observation | A fact captured or asserted at an identified boundary, with declared source, scope and completeness |
| Provenance | Who produced an observation, how it was obtained, and what verification supports that attribution |
| Direct server observation | A fact actually measured by the identified server-side collector; a browser's claim is not promoted to this class |
| Authenticated forwarded observation | A fact attributed to an authenticated configured producer over a protected channel; authentication does not prove every asserted fact true |
| Application/browser assertion | A field supplied by application or browser code, retaining that lower-authority origin |
| Derived observation | A result computed from identified inputs; its assumptions and missing inputs remain visible |
| Capability | An available observation or lifecycle hook in an exact deployment; package declarations and runtime evidence are separate |
| Binding | The verified association of evidence/decision with its exact producer, tenant, connection/request/stream, route, body and policy context |
| Complete body | All bytes of the explicitly selected encoding layer, not a parsed object or an unspecified partial stream |
| Commitment | A value intended to bind specified bytes or facts; exact canonicalization and verification are part of the contract, not implied by the word |
| Verdict | ALLOW, CHALLENGE or DENY under a declared policy, distinct from input/transport/dependency failure |
| Origin effect | Application work that may change state or release protected information, not merely parser acceptance |
| Replay refusal | Rejection of reused bound authorization/nonce; it does not by itself establish crash-safe exactly-once application execution |
| Independent interoperability | Agreement between independently implemented interpretations, with applicable positive and negative evidence |

Authenticated client identity, user authorization, natural-person presence and
benign intent are separate questions. A valid credential or a coherent browser
does not answer all four. A request ID or trace ID supplied by a client is not
an authoritative internal correlation identity.

The [requirements](profile.md#requirement-language) and
[implementation map](../validation/implementation-map.md) use these terms.
The [experimental wire](implementation-experiment.md) has its own concrete
fields and limits; its local producer/connection claims do not silently acquire
the generic profile's stronger evidence meaning.
