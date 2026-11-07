---
id: sot:root/network-capture
title: Original TLS and HTTP2 connection capture
status: active
created: 2026-11-07T07:25:00Z
updated: 2026-11-07T12:29:00Z
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
PendingHandshake::new consumes the accepted socket and starts the absolute
lifetime before any TLS reads/writes. Its cancellation handle must be obtained
before worker dispatch. PendingHandshake::accept consumes that owner and moves
the same socket, revocation state and deadline into Connection; no epoch reset
or credential/capability handoff occurs. Cancellation is checked before and
after actual I/O and before releasing handshake success or plaintext. All
pending/error/drop paths shut down the owned socket even if a supervisor handle
survives. Connection::accept remains a compatibility convenience, while runtime
uses the pending owner to establish cancellation custody before blocking work.
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

N07. The bounded single-request TLS/H1 consumer must call check_h1_idle after
durable audit and immediately before an origin effect. remaining() alone checks
only deadline/revocation, not peer state. The same owner checks its rustls reader,
TLS record boundary, SO_ERROR and a nonblocking socket peek, restoring blocking
mode before returning. EOF, reset/error, buffered plaintext, partial record or
any pending ciphertext invalidates and shuts down the epoch. Pending TLS control
records are conservatively unsupported here; the probe never blocks to decrypt
them or publishes their contents. H2/stream use is refused. Transport reads are
capped at each TLS record header/payload boundary, so queued next-record input
cannot hide inside rustls after an application read. This is a point-in-time
observation, not proof against a disconnect immediately after it returns.
Runtime retains its own cancel/controller/deadline checks and worker join.

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

Default-feature local component gate is recorded in evidence:root/20261107075403:
root all-targets Clippy with warnings denied passed, followed by21pure units
passed/0failed/0ignored/0filtered. Five earlier lint diagnostics and their repair
history remain explicit. hosted-pilot feature compilation,5actual hosted cases,
runtime request/stream binding and Docker product qualification remain NOT_RUN
or unimplemented. No capture source is integrated into the runtime yet.
The user-authorized PendingHandshake continuation changes Rust source after that
historical checkpoint. Runtime accepted same-epoch cancellation custody, one
pre-queue lifetime, idempotent revoke, terminal owner Drop, Send/Sync bounds and
joined worker ownership. The new22pure units passed with0failed/ignored/filtered
under root's explicit B allocation; hosted-feature all-targets Clippy and
compile-only hosted_transport passed. Evidence:root/20261107084330 records
60recursive source/context files unchanged and executable identities. Actual5
hosted cases and runtime/Docker boundaries remain unqualified; no old or
compile-only success is transferred into those execution claims.

## Current source and qualification frontier

Capture137e633c adds record-boundary reads and the conservative H1 idle probe;
runtime80b38ee consumes it at pre-decision, post-audit and pre-origin-write
boundaries. These authored paths supersede the historical absent-consumer note
above, but have not acquired actual execution qualification.

N05 continuation: each H2 stream exposes fixed-size observed input metadata from
the same live connection: completed unpadded DATA bytes, completed trailer field
count and HeadersPending/Receiving/Ended/Reset state. Initial request headers
remain distinct from END_STREAM. Partial frames/blocks do not advance completed
observations; frame-boundary EOF does not synthesize completion. Reset removes
request fields while retaining metadata for observation; terminal connection
failure removes both. No body/trailer values are retained by this metadata.
Ended does not validate Content-Length, request semantics, flow control or
application parsing, and cannot authorize origin effects. Runtime must perform
its own exact request/body association; an H2 serving owner remains unqualified.

The current source adds three pure metadata fixtures (27 pure authored total),
not yet executed. The measured b03622-unit checkpoint remains historical. Five
hosted tests remain authored and unexecuted; Docker full product remains failed
at the recorded813 checkpoint. Root allocation is required for new execution.
The user's later local Windows/WSL comparison authorization supersedes earlier
blanket hosted-only policy; fixture hosted guards remain unchanged and must not
be spoofed. Local Docker execution must follow Root's explicit Desktop engine
allocation and the canonical Docker-only product validation workflow.

Evidence:root/20261107120400 qualifies immutable edf700bc selected components:
27pure passed with zero failed/ignored/filtered, hosted-feature all-targets Clippy
passed, hosted_transport compiled without execution. All2350archived source files
were unchanged before/after, including the full internal recursive closure and
root build context. Registry trees were not separately snapshotted. This
supersedes the current27 NOT_RUN note for pure/compile gates only; actual5hosted,
H1 socket probe behavior, runtime serving and Docker full product remain open.

Evidence:root/20261107122900 now qualifies all5actual hosted capture fixtures at
e77b9792b5c31db25e008c0b4a6f42ae7ac969da, run42372114913. Both preflight and
component jobs succeeded;5passed0failed/ignored/filtered, command exit0,
source_changed=false. Capture code matches edf700bc, but signals wire source
changed, so the full hosted closure is e77, not the historical edf snapshot.
This supersedes actual5/H1-component NOT_RUN notes above. Joined worker and
pending-drop socket checks passed within those component cases; no runtime
durable-audit/origin-effect or global orphan census is inferred. Runtime serving,
complete product H2 and Docker remain open; full_product_qualified=false.
