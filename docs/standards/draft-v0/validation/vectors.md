---
layout: doc
title: Synthetic experimental parser vectors
description: Exact-byte shared-parser fixtures and their limited meaning for the project draft.
permalink: /standards/draft-v0/validation/vectors/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---
# Synthetic experimental parser vectors

**Diátaxis quadrant:** Reference. **Audience:** protocol implementers and evaluators.

**Status:** experimental adapter-parser fixtures, not generic Draft v0 wire or
independent interoperability vectors. Every input is synthetic. There are no
operational bearer credentials, session keys, custody paths or visitor traces.

The [fixture corpus](fixtures/adapter-parser-v1.json) fixes a clock input and
trusted producer profile for each exact byte sequence. Each case contains a
stable descriptive id, lowercase input hex, its SHA-256 and an independently
authored expected acceptance boolean. Decoding hex preserves duplicate/escaped
keys, byte-order marks and invalid UTF-8 that a JSON-map round trip would erase.

## Execute the unchanged corpus

```console
cargo run --locked -p humanymous-pages -- vectors --fixtures docs/standards/draft-v0/validation/fixtures/adapter-parser-v1.json
```

The Rust runner reuses the actual shared integrity parser. It reports accepted
and rejected inputs, mismatches and the complete fixture digest. Acceptance is
not an ALLOW verdict, authenticated peer, durable audit or origin invocation.
`draft_v0_conformance` remains false even when all expectations match.

| Cases | Boundary |
| --- | --- |
| Node raw / Fetch normalized | Two positive parser profiles with declared observation loss |
| Explicit nginx raw / normalized | Two positive shared-parser cases; not proof the authority executable selects nginx |
| Deadline / safe integer / protocol / producer mismatch | Version, trusted-profile and freshness refusal |
| Missing nullable / duplicate / escaped duplicate / unknown nested field | Exact presence and strict structural validation |
| Body completeness / length / digest / Base64 padding | Exact selected entity-byte integrity |
| Fabricated capture / TLS / stream | No invented unavailable capability |
| Ambiguous target / header CRLF / BOM / invalid UTF-8 | Request and byte-framing rejection |

Validation never regenerates expectations. The separate fixture-authoring command
refuses overwrite and is not called by the build or CI validator. A proposed
fixture change needs review of input bytes, expected behavior and source change.

## Still unresolved for Draft v0

These fixtures do not select the generic profile's serialization, canonical
commitments/signature coverage, authentication discovery, negotiation, loss/quality
fields or streaming lifecycle. They do not establish atomic cross-process
retries, restart epochs, before-origin ordering, independent implementations or
human/assistive-technology recovery. Those remain explicit
[conformance gates](conformance.md#current-status).
