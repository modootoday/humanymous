---
id: sot:root/pass-protocol
title: Source-reconstructed pure Pass alignment protocol
status: active
created: 2026-11-07T14:33:00Z
updated: 2026-11-07T14:33:00Z
verify: manual
rel:
  depends_on: [sot:root/rust-migration]
---

# Pure Pass protocol migration

This contract is reconstructed from the unchanged legacy internal/pass producer
and its two test files. Original SoT-36 was not found and was not reviewed. This
contract preserves the implemented pure protocol, not every historical comment.
Sixteen named legacy test responsibilities require explicit Rust mappings.

The existing challenge crate receives pass/{mod,generation,verification,tests}
modules and one library export. No dependency, Cargo manifest, scoring or browser
change is needed. Root owns this path and delegates the exact module slice; it
does not overlap the separately delegated Integrity optimization paths.

## Accepted byte and arithmetic contract

Derive HMAC-SHA256 over pass|, session UTF8, |, bucket big-endian u64, | and
instance big-endian u64, in that order with no extra framing. The first eight
seed bytes interpreted big-endian, OR1, initialize SplitMix64. Each draw uses
wrapping addition of0x9E3779B97F4A7C15, XOR/right-shift30 and wrapping multiply
by0xBF58476D1CE4E5B9, XOR/right-shift27 and wrapping multiply by0x94D049BB133111EB,
then XOR/right-shift31. Preserve modulo selection bias and draw order.

Clamp signed difficulty to0..3; row length is9+2*d and center is n/2. Alphabet is
exactly ABCDEFGHJKLMNPQRSTUVWXYZ23456789. For each of three rows, draw n characters
then one key index. Duplicate characters and keys already at center are allowed.
Wire names remain n, center, rows, chars and keyIndex. No secret answer is implied.

Freshness requires current>=issued and current-issued<=20, with no wrapping
adjacency. The historical bucket duration is30seconds, but this pure module takes
supplied buckets and owns no clock. Verification requires exactly three offsets
and regenerates the challenge using the supplied key/session/bucket/instance.
For the supported64-bit compatibility profile use i64 offsets, explicit wrapping
key-index addition and rem_euclid(n). Reducing offsets before addition would change
extreme-value legacy behavior. Cross32-bit Go arithmetic is not qualified.
Canonical solution offsets are (center-key_index).rem_euclid(n).

## Security and accessibility boundary

The public puzzle is intentionally derivable. A key may already be at center;
neither movement richness nor speed is required. Fixed distinct-session/instance
tests do not prove universal distinct answers: different seeds can collide on
the three offset solutions. Instance-bound request authentication, real-event
proof, issuance limits, single-use state and attestation/engine fusion remain
external required integration responsibilities. Alignment alone is not replay
protection or proof of humanity. No new detection rule or verdict follows here.

## Verification gates

Map all16 original named responsibilities, strengthen the historical r332 no-op
wrong-offset branch into an actual assertion and document that test-only delta.
Add clamps, empty/Unicode/delimiter sessions, maximum bucket/instance, SplitMix
wrap, freshness ages19/20/21/future, negative/multiple-period/i64-extreme offsets,
wrong counts and exact wire-name regressions. A Rust algorithm copied into tests
is not an independent oracle: frozen independently sourced seed/challenge vectors
remain required for stronger cross-language parity evidence.

Source preparation, allocated native units/check/strict Clippy, actual issuance
and replay/attestation process tests, and Docker product/a11y qualification are
separate gates. All authored code/tests are Rust1.99.1/edition2024. Missing external
evidence stays unavailable; this source contract cannot authorize enforcement.

## Initial source handoff

The five delegated Rust files now implement the pure protocol and27 authored
tests:16 mapped legacy responsibilities plus11 edge cases. Direct pinned
rustfmt/edition2024 and scoped whitespace checks passed. The subsequent native
Windows component run passed35 library tests including27 Pass cases and two
integration harness tests,37 Rust tests total, with no failures/ignored/filtered.
Locked/offline all-target check, strict Clippy and fmt exited0. The718 selected
package/root-manifest/dependency-source files matched before/after, with zero
hash differences and no challenge-package file inventory change. Configuration,
tool/SDK byte closure was not captured; this is not a full build-input attestation.
Raw output and summary are attached to evidence:root/20261106194410 under the
pass-protocol-first-check bundle. No independent Pass Go golden-vector or actual
issuance/replay/attestation integration result is claimed. Existing proof-of-work
and attestation modules are untouched; the library export adds no dependency edge.

Root also executed the unchanged three-file Go baseline at981d25cf using the
existing local Go1.26.3 windows/amd64 compiler, module mode disabled, proxy and
sum database disabled, local toolchain only, one build worker and a30second test
timeout. All16 original named tests passed with no source changes. Every copied
file's raw SHA256 matched its baseline retirement row. This is an unchanged
historical oracle execution, not new Go implementation or retained Go shipping.
Its prefixed source inventory, JSON test log and summary are attached to the same
private root evidence. It strengthens original-responsibility coverage but does
not produce independent challenge/seed byte vectors or prove the Rust runtime
issuance/replay/attestation composition. The r332 test-strengthening delta remains.
