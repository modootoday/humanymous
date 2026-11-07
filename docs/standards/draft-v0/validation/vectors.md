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

## Still unresolved for Draft v0

These fixtures do not select the generic profile's serialization, canonical
commitments/signature coverage, authentication discovery, negotiation, loss/quality
fields or streaming lifecycle. They do not establish atomic cross-process
retries, restart epochs, before-origin ordering, independent implementations or
human/assistive-technology recovery. Those remain explicit
[conformance gates](conformance.md#current-status).
