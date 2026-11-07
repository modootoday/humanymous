---
id: evidence:root/20261107131217
title: Audit key custody two pass crypto source
status: active
created: 2026-11-07T13:12:17Z
updated: 2026-11-07T13:12:17Z
rel:
  governs: [crates/audit/src/verification/scan]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody two pass crypto source

## What was measured

First allocated immutable8dbb attempt compiled with three private-interface
warnings, then strict Clippy failed101 before any tests. Source fix narrows
inspect and Diagnostic to the private scan module, matching the accepted sealed
S1 boundary. No type widening/lint suppression or crypto behavior change. Original
snapshot/logs remain in private evidence:root/20261107132346; corrected source
requires a fresh immutable snapshot and the same bounded allocated checks.

Source-only accepted private S1 Phase B. No Cargo workload; compilation, Clippy
and tests NOT_RUN. Eight new pipeline tests authored, formatting/diff only. The
same sealed Source/finite Limits/Control context runs complete Phase A then B.
Native producer remains Unsupported and the whole scanner remains test-compiled;
no CLI/runtime entrypoint, mutable Log, repair or authority token is promoted.

Phase B streams one segment at a time and a separate checkpoint cursor, using
the existing fixed64-peak frontier and shared canonical/HMAC/base/witness checks.
Sparse checkpoint sizes join exact physical record positions; remaining heads
beyond record EOF are still checked and cannot imply missing-prefix coverage.
All complete records/checkpoints continue after earlier cryptographic failures.
Diagnostic prefix Scope remains records-before-base-before-witness, separate
from actual attempt/success Work. Required nonempty witness coverage is refused
after otherwise successful verification; explicit empty/new install returns no
record Report, matching existing recovery semantics.

The exact previous raw record hash and previous raw writer-signature text are
moved into separate owned strings, never truncated64/128 or silently skipped.
Their logical lengths are constrained by admitted frame/string limits; actual
String capacities/allocator overhead remain unmeasured. Phase-A physical_latest
is dropped before opening Phase-B readers, while counts, exact-pin commitment,
raw transcripts/torn facts and the caller's same immutable policy remain bound.
No full checkpoint or previous-link payload is returned in the diagnostic.

Both passes share one raw transcript encoder. Counts/bytes/digests, sidecar
presence, torn offsets, frontier count, source membership/unchanged and final
cancellation/deadline must agree before any Report/Work is returned. A changed
source refuses even if decoded crypto fields still match. Tests use whole-history
fixtures only as oracles; the implementation retains no history Vec.

Authored coverage: valid two-pass native Scope parity; sparse prefixes across
power boundaries with an unsigned complete tail; later sequence/HMAC/base errors
overriding earlier witness failure; witness prefix/all-base Scope; malformed raw
hash/signature links preserving later actual successes; raw/pin-sidecar mutation
between passes; heads beyond EOF/missing required witness/explicit new-empty;
and final Phase-B cancellation or resource refusal without a Report.

## How to read it

No execution/pass count, native identity guarantee, measured capacity/RSS, complete
S1, hosted/product qualification or production S2/S3 follows. Actual safe native
Source construction, capacity/fault venue, physical versus virtual external
recovery-candidate handling and downstream startup remain requirements. The
isolated Phase-A21 proof at4b49 is historical and does not qualify these new edits.
