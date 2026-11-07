---
id: sot:root/network-capture
title: Original TLS and HTTP2 connection capture
status: active
created: 2026-11-07T07:25:00Z
updated: 2026-11-07T07:25:00Z
verify: manual
rel:
  governs: [crates/capture/**]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application, sot:root/crate-architecture]
---

# Original connection capture

Version0.1.0. Authorized Rust implementation slice; qualification incomplete.
The capture context owns bounded transport observation and connection lifetime.
Network owns existing tolerant transforms/fingerprints; runtime owns request,
session, policy, audit and origin effects. No scoring or signal changes.

## Rules

N01. Capture bytes on the actual owned TLS ingress before normalization. A
serialized adapter envelope, forwarding header, prefix parser result or supplied
boolean cannot create original-client provenance. CDN/reverse-proxy TLS describes
that ingress hop only. The deployment must identify whether it is original.

N02. Assemble the first complete ClientHello across record/chunk boundaries with
fixed budgets: record payload18432, total ingress handshake256KiB, hello64KiB,
records128. Validate exact nested vector boundaries before calling the existing
network observation parser. Its historical tolerance is unchanged. Absent,
truncated, malformed, over-budget or failed handshake observations never qualify.
Connection error is terminal. Reassembly is not TLS cryptographic validation.

N03. A safe Rust rustls connection owner reads the same bytes through its owned
transport and releases TLS observations only after rustls finishes its handshake.
No public mark_verified(bool), deserialization or caller-provided receipt. Reject
early data and HelloRetryRequest for this first profile unless a separately tested
second-ClientHello policy is implemented. Ephemeral borrowed evidence is scoped to
that owner; it is not a cross-process attestation or human identity.
A supervisor cancellation handle may only revoke/shutdown its same socket; it
cannot read/write bytes or mint observations. Revocation immediately removes
evidence availability. Owner drop closes the socket even if that handle survives.
The synchronous owner requires a supervised joined blocking worker when used by
an asynchronous service, with its absolute lifetime and explicit cancellation;
it must not become an untracked detached runtime task.
Borrowed observations are snapshots, not reusable allow tickets. The runtime
must recheck cancellation/deadline before origin effects and bind each request
to its actual connection/stream; copying a parsed hello never copies authority.

N04. For h2, validate the complete client preface, first non-ACK SETTINGS,
ordered settings including duplicate IDs, first connection WINDOW_UPDATE and
PRIORITY/HEADERS priority. Reassemble one contiguous HEADERS/CONTINUATION block;
reject interleaving, wrong stream, unsupported PUSH_PROMISE and malformed frame
length/stream forms. Bound frame16KiB, header block64KiB, decoded headers128,
decoded bytes64KiB, connection frames4096, streams128 and dynamic table4096.
These are resource/profile limits, not detection thresholds. Larger legal peers
may be refused by this explicit bounded profile.

N05. One HPACK decoding context follows connection order, including trailers.
Prevalidate exact integer and string spans for each representation before calling
the pinned decoder; its upstream truncated-integer paths cannot be trusted as a
framing validator. Feed exactly one representation per bounded decode call.
Decode bytes before normalization, preserve duplicate regular names and pseudo
order per stream. Validate unique known request pseudo fields before regular
fields, lowercase token names, forbidden connection headers, valid TE and bounded
values. A complete initial request header block is required; do not claim complete
request body or a fully validated HTTP server. Runtime must bind method, target,
authority, body and stream to its own request parser before authorizing effects.

N06. Runtime/authority currently reject required.raw_tls/original_h2 with422.
Keep that refusal until their owners integrate a same-owner borrowed observation
and actual hosted validation. Adapter0.1 capture remains null-only. Do not export
a wire format that lets an adapter manufacture capture. RIT/collector provenance,
TCP SYN, TLS ECH inner hello, QUIC, MITM resistance and T4 detection are outside
this slice. H1 TLS success cannot qualify H2.

## Verification and evidence

Pure Rust units cover every split point, record fragmentation, truncated nested
lengths, memory budgets, terminal failure, stream/continuation confusion, HPACK
dynamic state and malformed pseudo fields. Run locked scoped units, fmt, Clippy
only with root's global Cargo slot. Root owns Cargo membership/lock changes.
Actual Rust client/server TLS and h2 owned loopback pilots run on free GitHub
standard runners, with synthetic lab credentials and exact source hashes.
Actual product e2e remains Docker-only and owned by product-lab/delivery; test
missing required capture, downstream retermination, wrong stream and zero denied
origin effects. No local process/Docker/browser run or new workflow is authorized.
Units, supported hosted component tuples and product success are separate.

## Critique reflected

Blue: no second scoring engine; network transforms reused. Red P0: arbitrary
parse success and caller handshake booleans cannot mint provenance (N02/N03).
Red P1: cross-stream compression state and CONTINUATION interleaving invalidate
the connection (N04/N05). SRE: cap both bytes and counts and retain terminal
errors; no retry after ambiguous transport failure. DPO: raw hello/header bytes
are transient memory, no Debug dump/log persistence or personal certificate use.
Evaluator: capture measures one hop, not a human or full product; required planes
stay unavailable until actual binding and qualification (N01/N06).

## Sources

- https://www.rfc-editor.org/rfc/rfc8446.html (TLS record/handshake framing)
- https://www.rfc-editor.org/rfc/rfc9113.html (preface, frames, streams)
- https://www.rfc-editor.org/rfc/rfc7541.html (ordered connection HPACK state)
- https://docs.rs/rustls/0.23.46/rustls/server/struct.ServerConnection.html
- https://docs.rs/httlib-hpack/0.1.3/httlib_hpack/decoder/struct.Decoder.html

All new source and tests are NOT_RUN until recorded otherwise. No current
capture source is integrated into the runtime, and hosted/product counts are0.
