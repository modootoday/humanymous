---
id: sot:root/resource-watermark
title: Resource watermark migration contract
status: active
created: 2026-11-07T13:55:00Z
updated: 2026-11-08T16:08:00Z
verify: manual
rel:
  governs: [crates/sdk/humanymous-core/src/watermark/**]
  depends_on: [sot:root/rust-migration, sot:root/crate-architecture, sot:root/runtime-application]
---

# Resource watermark migration contract

## Live package supersession (measured 2026-11-08)

The standalone package proposal `crates/watermark` / `humanymous-watermark` is
**superseded** by the live module tree
`crates/sdk/humanymous-core/src/watermark/**` inside workspace package
`humanymous-core` (`humanymous_core::watermark`). Modules present: `payload`,
`frame`, `ledger`, `codec` (text/binary/png/wasm). Root workspace already pins
`png` and `wasmparser` for carrier work. Package `humanymous-watermark` is
absent from locked metadata. Empty `crates/watermark/**` is an untracked ghost
skeleton. Historical proposal wording below is retained as chronology; agents
must use the core module paths. Carrier/pilot gates (V1–V5, WASM/PNG pilots)
remain separate and are not product e2e success. Evidence:
`.spec.local/root/evidence/20261108155700-cursor-spec-gap-audit/`.

Version 0.3.0. Root accepted the reconstructed contract and W01–W09 safety
corrections. The original distinct-package grant is superseded as above. The
first source-only implementation grant covers payload/carrier, bounded volatile
ledger/reservation/cancellation and their tests, using existing
hmac/sha2/subtle/zeroize. Root owns workspace registration and lock integration.
No Cargo execution, codec dependency adoption, runtime wiring or actual pilot is
allocated by this path correction. V1–V5 remain separate gates; final parity is
not established.

The later second source-only grant permits bounded frame, strict MIME selection
and low-level text/opaque transforms without new dependencies. Its source delta
and complete fourteen-row status are in plan:root/20261107135500. Pages window
prohibits compiler/workload execution. This newer source is not covered by the
earlier A16 source-specific pass. WASM validator adoption and PNG remain pending;
strict MIME identification alone does not implement those formats.

Evidence:root/20261107144843 qualifies corrected2eb2069a pure components with
29passing units, all-target check/strict Clippy/fmt0 and3589-file recorded closure
with0difference records. The original27f1d1d1 strict Clippy failure/tests NOT_RUN
remain preserved separately. Real grammar/resource semantics, WASM/PNG and
V3–V5 are not established by this component pass.

## 1. Brief and evidence boundary

Migrate the complete twelve implementation and two test files in
`internal/watermark` without losing valid historical carrier recovery, MIME
channels, issuance linkage or metadata-strip recovery. Resource watermarking is
L5 forensic attribution to an issued session, not proof of a human, culpability,
or an automatic DENY. Frozen scoring and motor/speed-free a11y remain unchanged.
Preserve the original baseline and every source hash. No Go deletion is granted.
No new Go or other foreign implementation, including executable string templates.
Rust 1.99.1 / edition 2024 applies to subsequent implementation and tests.
Actual resource pilots and full Docker product qualification remain separate.
T4, copying another session's complete carrier, lossy image transformations,
collusion and arbitrary watermark removal remain limitations.

The original SoT-08 is referenced by legacy code but is absent from the current
worktree, baseline Git tree, path history and searched local spec snapshots.
This contract reconstructs observed behavior; it does not claim the original
specification was read. Root explicitly accepts a new source-reconstructed canon
anchored to all fourteen exact baseline files and tests as the implementation
provenance disposition. The original remains NOT_FOUND, never read/recovered.
This resolves implementation authority, not final parity. All fourteen Go files
were read in full.
No watermark implementation or consumer was found under `crates` by path and
content searches at investigation HEAD recorded in evidence. This is a bounded
source observation, not proof about every historical branch or external service.

## 2. Responsibility and package proposal

**Superseded package location:** live ownership is
`humanymous-core` / `crates/sdk/humanymous-core/src/watermark/**` (see Live
package supersession above). The following historical proposal text is kept for
provenance only and must not be used to recreate a separate package.

Propose `crates/watermark`, package `humanymous-watermark`, as a cohesive resource
forensics library. It owns formats, bounded transformation and volatile issuance
state; it is neither challenge presentation nor collector integrity. The routing
queue's coarse `challenge`/`integrity` candidates have no existing mapping.
Root accepted this location historically; workspace membership remains root-owned.

| Legacy file | Proposed module / responsibility | Required preserved behavior |
| --- | --- | --- |
| payload.go | payload | HMAC-SHA256, 10-byte tag, big-endian tag ID and bucket |
| ecc.go | frame | HMWM marker, u16 length, IEEE CRC32, scan recovery |
| codec.go | codec/dispatch | MIME normalization and explicit family selection |
| codec_text.go | codec/text | Markup/block comments and JSON trailing whitespace |
| codec_binary.go | codec/binary | Opaque binary trailer and recovery |
| codec_png.go | codec/png/metadata | hmwm tEXt base32 frame and chunk CRC |
| codec_lsb.go | codec/png/pixels | RGB LSB, row-major, MSB-first, alpha unchanged |
| codec_png_multi.go | codec/png | Metadata plus pixels, independent recovery |
| codecwasm.go | codec/wasm | hmwm custom section with bounded u32 ULEB |
| inject.go | issuance | Typed apply result and explicit fallback outcome |
| extract.go | recovery | Preferred MIME then deterministic alternate codecs |
| ledger.go | ledger | Allocation, records, retention, authenticated trace |
| lsb_test.go | tests/png | Direct round trip and actual pixel re-encode strip |
| watermark_test.go | tests/compatibility | Eight families, semantics, trace/forgery/frame |

Use standard `src/lib.rs`, cohesive context modules and integration tests; facade
files only expose real boundaries. Do not create forwarding-only micro-crates.
Potential crypto reuse is existing hmac/sha2/subtle/zeroize; root owns exact
dependency edges. PNG decoding/encoding, CRC32, base32 and WASM validation require
an explicit dependency review. They are not currently present as named packages
in the inspected lockfile. No package version is selected by this proposal.

Runtime owns HTTP/session/authentication, canonical asset access, issue-time
bucket, operator authorization and response cache policy. Audit owns privileged
trace-event recording and dual-control erasure. Lab owns end-to-end scenarios.
The library accepts validated bytes and typed issue context; it does not read
paths, parse forwarded identities, authorize an operator or alter policy.

## 3. Compatibility and corrected decisions

W01. Preserve legacy-v1 bytes: carrier is u32be(tag_id) followed by the first ten
bytes of HMAC-SHA256(key, session_utf8 || NUL || asset_utf8 || NUL || u64be(bucket)).
The frame is ASCII HMWM || u16be(carrier_len) || carrier || u32be(IEEE CRC32(carrier)).
A normal carrier is14 bytes and frame24. CRC detects damage, not forgery and not
error correction. No ECC implementation or bit-error correction is claimed.
Use constant-time tag verification, typed zeroizing secret ownership, no raw
keys or session/asset identities in public receipts. Empty keys are rejected.
New issuance rejects embedded NUL in session/asset and tag ID zero. Preserve
legacy verification for retained records; do not silently change the HMAC input.
Tag IDs are not included in this MAC: changing an ID to another record with the
same payload is a known v1 limitation, not proof of nonrepudiation. A stronger
domain/version/key-ID/ID-bound format needs a separate accepted migration.

W02. Bound every parser before allocation and use checked arithmetic. The normal
issued carrier has exact length14; historical SplitCarrier ignores suffix bytes.
Keep that behavior only in an explicitly named legacy extraction path, with a
receipt marking ignored suffixes. New issuance accepts exact fixed carriers.
Frame encode rejects lengths above65535 rather than reproducing u16 truncation.
Scan invalid markers past CRC/truncation failures within the global scan budget.
Do not accept malformed frame CRC as authenticated content.

W03. MIME matching lowercases, trims and removes parameters. Preserve PNG, WASM,
text-like and opaque-binary families. Legacy text heuristics use leading `{`/`[` 
for JSON whitespace, `<` for markup, else block comments. New issuance must also
check declared format and valid source semantics: JSON scalars require whitespace
too; arbitrary text/plain does not gain a semantic-preservation claim merely
because the old code appended a block comment. Unsupported transform returns a
typed error unless the caller explicitly permits opaque trailer semantics.
Legacy broad substring MIME matching remains a recovery hint, never authority.
No global mutable registry or nondeterministic hash-map recovery order.

W04. Preserve padded RFC4648 base32 comment markers `!hmwm:...!`, markup wrapper
and block-comment wrapper, and newline + space(0)/tab(1), MSB-first JSON channel.
Recovery considers all bounded marker candidates rather than only the first
regex match. Distinct authenticated carriers from different candidates/channels
produce Ambiguous, never an arbitrarily attributed session. Text transform
semantics depend on MIME and parser validation, not marker round trip alone.

W05. PNG metadata is keyword hmwm + NUL + padded base32(frame), inserted before
IEND with CRC(type || data). Parse signature, chunk extents/order and CRCs safely.
LSB is u32be(frame_len) || frame in row-major R,G,B low bits, MSB first, skipping
alpha. Check `3 * width * height >= 8 * (4 + frame_len)` before processing pixels.
Preserve Go premultiplied RGBA behavior explicitly for legacy decode; palette,
grayscale, 16-bit, transparent and partial-alpha cases need cross-format fixtures.
Do not assume Rust straight-alpha image buffers match Go image.RGBA conversion.
For new dual-channel output, apply pixel transform first and insert metadata
after final PNG encoding: the legacy metadata-first re-encode discards tEXt.
Return actual channels written/recovered and degradation reasons; the legacy
png-multi label alone does not prove both channels exist. Small images may use
validated metadata only, visibly degraded in the receipt. Strict mode requires
the caller's selected channels. Animated PNG must be rejected for issuance until
frame preservation is implemented; silently flattening animation is forbidden.

W06. WASM preserves the version1 header, appends a custom section id0 named hmwm,
and preserves executable sections. Reject malformed/overflowing/truncated u32
ULEB, invalid module/header, impossible section extents and invalid name lengths.
Semantic success requires actual validation and execution of a module with an
export; magic-prefix preservation alone is not enough. Binary trailers are
limited to explicitly declared opaque formats that permit trailing bytes.
Do not blindly append trailers to malformed PNG/WASM or unknown signed formats.

W07. Apply returns transformed bytes plus actual channel set and original/output
hashes. No successful required-watermark response may contain original unmarked
bytes, including transformation, limit, ledger or key errors. The legacy caller
`cmd/server/resource_handlers.go` assigns original src on error; runtime must
replace that path with a typed unavailable/failure outcome. Cache-Control:no-store
and session isolation remain required. Never create a successful ledger issuance
for failed encoding. Atomic reserve/transform/commit must ensure a response is
not exposed before its issuance is recorded; cancellation releases reservation.
Fallback is opt-in to semantically permitted opaque trailer, not silent fail-open.

W08. Ledger is a bounded synchronized volatile owner with injected clock,
monotonic nonzero u32 IDs, exhaustion errors instead of wrap/reuse, capacity and
reservation bounds. Retain Payload, MIME, actual channels, issue time and optional
keyed IP pseudonym. Pseudonyms remain personal data. Expiry is enforced on lookup
at age > TTL (matching historical GC boundary), not only background GC. Reject
future issue times; validate TTL/configuration. GC is bounded and automatic enough
to honor retention; no expired record is traceable while awaiting GC. TTL is
explicitly configured and nonzero; no universal24-hour default is accepted.
Volatile restart loss is explicit; persistent ledger is a separate design.

W09. Trace resolves candidates and ledger record then verifies HMAC. Return
Verified, Forged, Unknown, Expired, Ambiguous, Invalid or ResourceLimit distinctly
inside the trusted operator boundary. A forgery may carry a diagnostic record
reference for an authorized operator, but must not be serialized as a successful
session attribution or scored as a verified trace. Unknown/expired are not bot
evidence. Reject key/custody errors, never invent identity or authorize bans.
Operator auth precedes request-body read; no configured credential means disabled
route. Rate/concurrency/deadline limits and audit integration are runtime duties.
Erasure/kill switch/permanent bans require the existing dual-control contracts;
library GC is normal retention, not an administrative erasure bypass.

## 4. Proposed resource limits and ownership

These are reviewable initial limits, not measurements or permission to compile.
Runtime must admit the declared worst-case budget before starting the work.

| Boundary | Proposed limit / behavior |
| --- | --- |
| Input bytes, issue or trace | 32MiB; check actual bounded read, not Content-Length only |
| Output bytes | 33MiB, checked before growth and encoding |
| PNG dimensions | 8192 per axis, at most4194304 pixels, decoded RGBA16MiB |
| PNG decode/encode workspace | hard total working-memory budget128MiB per admitted task; decoder allocation limits required |
| PNG chunks / WASM sections | at most65536; checked lengths and total scan budget |
| Frame length | carrier <=65535; LSB advertised framed bytes <=65536 and capacity check |
| Candidate frames / markers | at most64; limit outcome instead of partial attribution |
| Session and asset IDs | at most1024 UTF-8 bytes each, no NUL for new issuance |
| Ledger | at most65536 live entries including reservations; no eviction of unexpired evidence to admit a new response |
| Concurrent codec tasks | at most2 initially, runtime admission may be lower |
| Cancellation / deadline | caller-owned cancellation and bounded work counters; runtime deadline2s initial proposal, no detached survivor tasks |

Encoded-byte limits do not prevent decompression bombs. Decoder-specific hard
allocation and work limits must be demonstrated before PNG support is admitted.
If dependencies cannot enforce cancellation within a call, the resource caps and
bounded worker lifetime must be qualified or that profile remains unsupported.
No unmeasured claim that a generic timeout stops CPU work or allocations.

## 5. Verification gates

V1 source inventory: preserve baseline981d25cf71a5a54855d7e2b4359e832a9d5998e5,
registry digest and fourteen baseline/current hashes with row pointers298–311.
Separate twelve implementation files from two test files and eight named tests.
Retain immutable source fixtures; no Go execution is needed for this design step.
Explicitly record deliberate corrections W02–W09 rather than pretending byte
equality for invalid input or reproduction of unsafe behavior.

V2 allocated Rust component gates: locked package tests, fmt and Clippy; golden
HMAC/frame/base32/ULEB/LSB vectors independent of encoder round trips; wrong key,
CRC corruption, first bad/later good marker, malformed lengths, limit boundaries,
integer overflow, multiple carriers, ID exhaustion, concurrent reservations,
TTL exact edge, GC and cancellation. Preserve all original test responsibilities.
Original minimalPNG has invalid zero dimensions/no IDAT; keep it as malformed
structure evidence, not a valid-image success fixture. Original WASM magic-only
test is supplemented by validation/execution, not treated as runtime proof.

V3 allocated real-resource pilot: Rust controller reads actual repository assets
and independent valid PNG variants; issue files, parse/render JSON/HTML/SVG/CSS,
execute existing JS through an isolated real browser driven by the owning Rust
pilot (no authored foreign scripts), validate/instantiate a source-bound WASM module
and compare its export result, inspect PNG channels, strip ancillary metadata and
independently decode/re-encode pixels before recovery. Assert alpha unchanged and
RGB deltas bounded to one for the qualified input model; document unavoidable
format conversion separately. Include palette/alpha/16-bit and too-small PNG,
unknown MIME recovery and permitted opaque binary. No network/third-party probes.
The tracked tree currently contains no .wasm artifact. Provision the pilot module
from an allocated Rust wasm32 build or an approved immutable fixture with its
source/hash; neither a nonexistent asset nor the legacy eight-byte header counts
as the executable fixture. Allocation includes this build and real engine pilot.

V4 runtime-owner integration pilot: real authenticated issuance/trace HTTP paths,
two sessions with independent outputs, no-store/cache leakage check, unavailable
key/ledger/codec rejection, expired/forged/no-mark negatives, operator-disabled
and unauthorized rejection before body processing, cancellation and bounded load.
Runtime, authority and audit owners review exact interfaces; do not edit their
paths under this lane. No library-only test may satisfy V4.

V5 lab-owner Docker product scenario: exact source/image/topology, Rust runner,
actual asset download, independent strip, authenticated trace and explicit
assertion of verified linkage without changing risk scores or verdict bands.
Legacy watermark_strip.mjs fabricates DENY/risk100 from trace and only removes
ancillary chunks despite its re-encode comment. Preserve it as a historical
oracle, not a new Rust scoring rule or full pixel-reencode witness. Full product
qualification and native/OS gates remain owned by root and lab.

Every gate retains source/dependency/config/fixture hashes before/after, command,
venue, allocated budget, terminal exit/footer, failures and ignored/unsupported
cases. Source availability, requested execution and component green are distinct.

## 6. Critique reflected and unresolved gates

Blue: forensic service cannot be mixed into challenge or frozen scoring (W09).
Red: whole-carrier copying/ID substitution/ambiguous frames prevent individual
culpability claims (W01,W04,W09). SRE: strict limits, ledger exhaustion and response
atomicity avoid unsafe fallback (W07,W08). DPO: attribution is privileged personal
data with retention and audited erasure ownership (W08,W09). Evaluator: CRC is
not ECC, sample headers are not actual resource semantics, and source-only is
not product parity (V1–V5). These are single-session perspective checks, not
independent agent reviews.

Root accepted the source-reconstructed design and explicit v1 limitations for
implementation. The first grant is payload/carrier and bounded volatile ledger
only. Runtime fail-closed issuance/trace auth, cross-owner interfaces and
dependency-specific PNG allocation/alpha proofs remain prerequisites for their
later implementation slices and qualification. Initial limits are component
admission caps, not production SLAs. No open gate is disguised as completed.
Design acceptance is distinct from execution allocation and migration-row closure.

## 7. Initial payload and ledger source contract

The accepted source-only slice exposes `Payload`, owned zeroizing `WatermarkKey`,
strict `split_carrier` and marked `split_legacy_carrier`, plus synchronized
`Ledger` and linear `Reservation`. Retained v1 verification preserves NUL input
semantics within the identifier admission bounds; new issuance rejects NUL/zero
ID/oversized identifiers. The owning key allocation is zeroized; this does not
claim erasure of every upstream library's transient HMAC state or caller copies.
No frame CRC, MIME codec, resource transform, extraction or trace API is shipped
by this initial slice.

Evidence:root/20261107141000 subsequently verifies the initial source at exact
commit9b064dbd on native Windows: all-target check, strict Clippy and fmt exit0;
16 pure units passed with0failed/ignored/filtered. Recorded source/registry/tool-bin
closure3531files had0difference records. The receipt states excluded native SDK/
system tool inputs. This is initial component V2, not full fourteen-file parity,
real resource semantics, authenticated HTTP trace or V3–V5 product qualification.

Ledger uses caller-supplied trusted monotonic Duration instants from one epoch,
rejects regressions, and stores commit instants without pretending they are
wall-clock IssuedAt values. Runtime must map monotonic retention and separately
controlled display timestamps. Capacity includes reservations. Reservation
drop/cancel removes only that pending entry; unique IDs are never recycled.
Commit validates actual MIME/channel/pseudonym metadata, matching ID and
reservation TTL. It does not assert encoding happened; runtime must enforce
encode-before-commit-before-response and provenance of all supplied values.
Lookup enforces TTL at read even without GC. Pending IDs are never lookup records.
Normal GC removes expired entries and runtime must schedule it during inactivity
to enforce physical retention. No persistence/restart recovery or erasure API.
Mutex poisoning makes all public state operations fail closed; reservation Drop
may recover a poisoned guard solely to remove its own pending entry.
