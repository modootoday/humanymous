---
id: evidence:root/20261107123751
title: Audit key custody bounded phase A source
status: active
created: 2026-11-07T12:37:51Z
updated: 2026-11-07T12:37:51Z
rel:
  governs: [crates/audit/src/verification/scan]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody bounded phase A source

## What was measured

Source-only accepted private S1 Phase-A core. No Cargo workload/allocation;
compilation, Clippy and execution NOT_RUN. Seven new pure tests are authored,
including the deadline poll test. Formatting/diff only. The scanner is currently
test-compiled and has no supported native producer or production entrypoint.

Sealed private Source opens each segment independently and a separate checkpoint
sidecar; it supplies unchanged observations under its future owning lock. Native
open returns Unsupported through an uninhabited result type until actual safe
opened-file/provider identity is accepted. No caller boolean/provider token is
accepted as authority. Fixture Source is a full-input in-memory test oracle,
not actual native identity or bounded-memory evidence.

Bounded Frames preflights JSON resources and aggregate raw bytes/nonblank values
before owned Record/Checkpoint decode. It records exact raw LF/CRLF/blank/torn
bytes, with domain-separated segment/sidecar digests and complete/torn offsets.
Resource/error/cancel failures poison retry/finalization; partial EOF refuses.
Compatibility framing maps resource errors back to historical InvalidData while
the bounded reader exposes FileTooLarge. One canonical framing implementation is
shared, without changing frozen bytes or field decoding.

Phase A parses through all frames, checks every node, checkpoint progression and
Merkle format, exact physical trusted-head membership and explicit empty/new
installation rules. It polls cancellation/deadline around I/O, preflight/decode
and final source checks, then rechecks source membership. No record HMAC/content,
writer/witness signature or Merkle authentication is claimed by this metadata.
Tests deliberately use invalid seq/hashes that metadata admits, proving that
metadata is not a cryptographic pass. No file repair/write/healthy owner exists.

Tests cover raw transcript/torn accounting, nonterminal torn concatenation
refusal, incomplete sidecar refusal, node/progression/exact pin priority,
budget-before-owned-decode and poison, invalid profile/cancellation before access,
final-source cancellation/change/membership refusal, Unsupported native source
and elapsed deadline. Profile arithmetic is address-space sanity only, not the
whole-startup allocation ledger or proof of128MiB/RSS.

## How to read it

Phase B must use the same bounded readers, shared crypto checks/frontier and
compatibility accumulator, compare complete raw transcripts and finish source/
deadline checks before any diagnostic success. Actual native backend, measured
capacity/fault venue, physical/virtual recovery provenance and all production
S2/S3 remain pending. Captured140 at319a is historical and does not qualify these
new framing/scanner changes. No existing CLI or runtime route is promoted.
