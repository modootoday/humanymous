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

The original and internationalization corpora select only the Node and nginx
profiles. The [producer-profile table](../protocol/implementation-experiment.md#producer-profiles-and-service-readiness)
also lists newer Python ASGI/WSGI, Ruby Rack, Apache and ASP.NET Core parser
contracts, but those are not covered by those two fixture sets. Their source
unit tests are a separate evidence layer; neither adding a profile enum nor
passing this unchanged corpus establishes cross-host compatibility. Expanding
the corpus requires separately reviewed exact bytes and expected results, not
renaming an existing passing profile or regenerating its acceptance record.

## Normalized host profile corpus execution

A separate [normalized-profile corpus](fixtures/adapter-normalized-profiles-v1.json)
contains 25 authored cases for Python ASGI/WSGI, Ruby Rack, Apache and ASP.NET
Core. Each profile has one expected acceptance and four expected rejections:
the wrong observation mode, the wrong producer identity, fabricated raw header
order and an unsupported claimed peer. Inputs have fixed exact bytes and digests;
the expected total is five acceptances and 20 rejections.

The runner selects those trusted profiles explicitly through the shared parser.
The 2026-11-07 Windows component run compiled the extension, passed all 27 library
tests and passed all-target check and Clippy with warnings denied. The read-only
`vectors --fixtures` execution measured five acceptances and 20 rejections with
no errors for fixture SHA-256
`63991623d692c135597249e73378c6f9a0228c5d149c617ce1ae4079900625c3`.
The unchanged earlier corpora also matched all 24 and 10 expectations. No
expectations were regenerated. These results add no independent implementation,
native-host qualification or generic Draft v0 wire semantics; the runner reports
`draft_v0_conformance: false`.

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

## Separate internationalization and binary-byte corpus

The [internationalization corpus](fixtures/adapter-parser-i18n-v1.json) adds ten
synthetic cases without changing the original 24-case corpus. The current shared
parser accepts six and rejects four; these are parser results only.

```console
cargo run --locked -p humanymous-pages -- vectors --fixtures docs/standards/draft-v0/validation/fixtures/adapter-parser-i18n-v1.json
```

| Input | Authored expected parser behavior |
| --- | --- |
| Binary body `00 ff 80` | Accept its exact length and digest without treating the body as UTF-8 |
| Composed `é` (`c3 a9`) and decomposed `e` plus combining acute (`65 cc 81`) bodies | Accept each with its own exact-byte digest; do not normalize one to the other |
| Decomposed body with the composed body's digest | Reject despite similar rendered text |
| Non-Latin target and Korean/Arabic JSON header value | Accept at the envelope-parser boundary; this does not qualify downstream HTTP header/target fidelity |
| Percent-encoded UTF-8 target | Accept while retaining the original target representation |
| Invalid decoded UTF-8, encoded NUL and encoded dot segment in a target | Reject |

Each input commitment is over the fixture's exact decoded JSON bytes. Body
commitments are separately over decoded body bytes. The two layers must not be
confused. This corpus selects no generic Draft v0 Unicode-normalization rule,
internationalized-host mapping, signature canonicalization or locale behavior.
Those decisions still need their own wire contract and independent vectors.

The current authority route inventory separately requires ASCII paths without
percent escapes. Its lookup uses the unnormalized path before the query, so
the accepted non-Latin and percent-encoded **path** fixtures do not establish
service route readiness or origin forwarding. A Unicode query on an otherwise
eligible path is a different case. See the
[service route boundary](../protocol/implementation-experiment.md#producer-profiles-and-service-readiness).

## Still unresolved for Draft v0

These fixtures do not select the generic profile's serialization, canonical
commitments/signature coverage, authentication discovery, negotiation, loss/quality
fields or streaming lifecycle. They do not establish atomic cross-process
retries, restart epochs, before-origin ordering, independent implementations or
human/assistive-technology recovery. Those remain explicit
[conformance gates](conformance.md#current-status).
