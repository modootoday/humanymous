---
id: evidence:root/20261107142252
title: Bounded carrier codec source candidate
status: active
created: 2026-11-07T14:22:52Z
updated: 2026-11-07T14:24:53Z
rel:
  depends_on: [sot:root/resource-watermark, plan:root/20261107135500]
files:
  - {path: source.json, sha256: f33dee4627beede20d857377d3d9f8cae582b88d13a1e28d160d5d17508d009b, bytes: 3658, store: repo}
---

# Source-only carrier candidate

Root authorized source/tests only for the existing crate, with no dependency or
graph changes and no compiler/workload during the Pages window. Normal misc lane
was free and claimed with crates/watermark plus the owning spec paths. The
accepted plan delta was written before source, including bounds, ambiguity,
NUL/TTL/ID rules and all fourteen legacy responsibility statuses.

New source implements generic legacy CRC frames, deterministic strict MIME
selection, low-level text comments/JSON whitespace and explicit opaque trailers.
The original payload/ledger files remain unchanged. Generic frame decoding is
not a successful authenticated fourteen-byte carrier trace. Unknown MIME never
automatically selects a trailer. PNG/WASM identification does not imply an
implemented encoder. WASM standards validation remains conditional on root
dependency adoption; PNG alpha/resource/cancellation gates remain open.

Text transformations validate UTF-8/NUL/admission bounds, not the source grammar.
They are not a high-level issuance API. W03 requires source syntax/semantics
validation in the future composition root, and W07 requires successful ledger
commit before response. No new API claims arbitrary invalid JSON/markup/code is
safe to serve. Exact legacy comment and whitespace carrier bytes are preserved
by design; actual compatibility tests and resource semantics remain unexecuted.

The CRC definition is anchored to the primary
[W3C PNG CRC appendix](https://www.w3.org/TR/png-3/#D-CRCAppendix); CRC is corruption
detection, not ECC/authentication. Base32 follows
[RFC4648 section6 and vectors](https://www.rfc-editor.org/rfc/rfc4648.html#section-6).
It is byte encoding, not handrolled cryptography. No external package was added.

Source formatting with direct rustfmt succeeded. There are29 authored tests:
the original16 plus5frame,2opaque,2MIME,3text and1base32 case. NO compiler, Cargo,
test or pilot ran on this candidate. Earlier A16 evidence applies exclusively to
the original9b064dbd source and must not be inherited by new exports/codec source.
The attached source snapshot binds the current package files. Full14-file
migration, original all-family tests, real resources, HTTP and Docker remain open.
