---
id: plan:root/20261107135500
title: Resource watermark preservation and integration proposal
status: active
created: 2026-11-07T13:55:00Z
updated: 2026-11-07T13:55:00Z
rel:
  implements: [sot:root/resource-watermark]
---

# Resource watermark plan and INTEGRATION proposal

## Mission and current authority

Complete all fourteen legacy watermark responsibilities in Rust, unit and actual
resource pilots, with source-bound evidence. The persistent goal remains active.
Initial authority was read-only investigation plus this named SoT/plan/evidence
scope. Root subsequently accepted the contract and granted source-only
payload/carrier and bounded volatile ledger/reservation/cancellation with tests.
The misc claim was released and reclaimed with `crates/watermark/**` added;
no overlapping model paths were claimed. No Cargo/lock/root graph execution,
codec dependency adoption, pilot, cloud or deployment changes are allocated.
Session rust-resource-watermark claimed free misc after actual board inspection.
Foreign dirty files are untouched. Shared Git/index operations were not used.

## Milestones and root decisions

1. Prepared: read all fourteen files, original server consumer, historical strip
   oracle, routing rows and conventions; record source/hash inventory. Original
   SoT-08 is missing; obtain authoritative bytes or record root's disposition.
2. Accepted by root: `crates/watermark`, W01–W09, explicit valid-input
   compatibility versus deliberate safety corrections. Missing SoT-08 is
   explicitly disposed as a new source-reconstructed canon, never original-read.
   TTL is explicit/nonzero; limits are initial admission caps, not production SLA.
   No automatic activation or scoring. Codec dependency review remains pending.
3. Pending owner agreement: runtime selects issue/trace composition and secret
   handles; authority selects operator capability; audit agrees trace/erasure;
   lab owns scenario. Root owns shared graph and final central baseline mapping.
4. Source-only first slice authored: package manifest, payload/carrier, bounded
   volatile ledger, linear reservations and focused tests; not compiled or run.
   Root integrated existing hmac/sha2/subtle/zeroize edges and membership atomically
   with the ten frozen package files at commit
   9b064dbd8942d4fad7f1623da97b6b64c0177a7f. Remaining codecs and original full
   tests are pending. Registration/tree resolution is not compilation or parity.
5. Root A16 allocation executed the frozen initial slice at9b064dbd in its exact
   archive: native Windows fmt/check/strictClippy all0,16 pure tests passed with
   zero failed/ignored/filtered.3531 before/after files,0 difference records in
   the documented archive/registry/tool-bin/config closure. Evidence
   evidence:root/20261107141000 states native toolchain closure limitations.
   This is initial-slice V2 only; complete codec units and V3 remain pending.
6. Owner-wired actual HTTP pilot V4 and Docker product scenario V5; retained
   failures and source closure. Root integrates reviewed migration rows serially.
   Legacy removal is root-owned after consumer migration and actual qualification.

## Proposed interface for owner review

The package accepts `Payload` (tag ID, session ID, asset ID, bucket), typed secret
reference/borrow and `CodecLimits`, returning `AppliedResource` (bytes, channels,
degradation reason and hashes). Pure `frame` and `recover` return bounded typed
results. A `Ledger` owns reservations, checked IDs, commit, expiry and lookup;
`trace` returns verified/forged/unknown/expired/ambiguous without policy effects.
No HTTP, disk lookup, credential provisioning or signal scoring belongs here.
Names are a design proposal, not an already implemented API. Runtime must not
depend on assumed methods until both owners agree and root accepts graph changes.

## Work history and handover

### Preservation matrix and actual fixture readiness

This matrix turns the eight named legacy tests into independently reviewable
acceptance obligations. None has been executed in Rust yet.

| Legacy test | Preserved assertion | Additional actual-boundary obligation |
| --- | --- | --- |
| TestRoundTrip_AllFamilies | JS, CSS, HTML, SVG, JSON, opaque bytes, PNG and WASM yield tag7 and valid MAC | Parse/render/execute qualified formats; verify MIME normalization and wrong-MIME recovery |
| TestSemantic_JSONStaysValid | JSON object fields and values unchanged | Scalar/array/object JSON with existing trailing whitespace and exact no-loss comparison |
| TestSemantic_PNGChunksIntact | PNG signature, IHDR and IEND survive | Real decoder accepts pixels, CRC/order valid, both channels independently present; legacy zero-sized fixture is negative only |
| TestSemantic_WASMStillHasMagic | WASM header retained | Actual executable module validates and export result matches, code/data section hashes unchanged |
| TestTrace_IdentifiesSessionAndDetectsForgery | Known issuance yields original session; wrong key is identified as forged | Forgery is never successful attribution; unknown/expired/ambiguous and unauthorized responses cannot expose identities |
| TestFrameUnframe | Recover carrier from prefix/suffix noise | Independent CRC/HMAC vectors and bad-first/good-later markers; checked bounds/candidate limit |
| TestLSB_SurvivesMetadataStrip | Recover tag99 after decoded-pixel re-encode without metadata | Independently prove metadata absent and pixels re-encoded; alpha/color-model compatibility and qualified RGB deltas |
| TestLSB_DirectRoundTrip | Frame/tag5 recover directly from pixel channel | Independent RGB bit-position checks, capacity exact/one-bit-short and malformed length rejection |

Read-only fixture review at HEAD37a85fe1d7d6bdd5602169856786760e40437483:
`web/img/sample.png` is224 bytes; its IHDR advertises96x96 RGB8, large enough
for the224-bit length-plus-normal-frame channel. Header inspection does not
prove PNG decode success. `web/honeypot/analytics.js` is existing first-party
`void 0;`, suitable for an actual browser no-op execution comparison without
executing the full legacy detector. `test/externalinput/fixture.css` is an
existing stylesheet candidate. `web/index.html` loads the legacy Go WASM runner
and collector; using it as-is cannot isolate markup semantics or prove Rust
collector behavior. Do not fabricate absent dependencies or relabel page load
as watermark semantics. Fixtures and expected assertions require pilot-owner
review before use.

`git ls-files '*.wasm'` found zero tracked artifacts. Therefore the WASM pilot
must obtain a source-bound module from a separately allocated Rust wasm32 build
or approved immutable fixture; an eight-byte empty legacy fixture proves no
export execution. Realistic JSON/SVG variants likewise need explicit fixture
selection, not invented claims that they already exist. The fixture receipt
records hashes for available candidates without running or changing them.

- 2026-11-07: applied coordinate-sessions and design-sot; read persona inputs,
  performed local perspective synthesis and adversarial completeness check.
- Continuation: applied implement-sot-slice after root's explicit acceptance.
  Authored the package manifest, payload/carrier and bounded ledger with16 tests;
  direct rustfmt source formatting succeeded. Cargo/compiler/tests/pilots remain
  NOT_RUN. Manifest and initial source hashes are handed off in initial-source.json;
  root graph integration and execution allocation are separate pending actions.
- Read-only official codec dependency candidate proposal attached as
  codec-dependencies.md in the owned design evidence bundle: exact live registry
  versions/checksums/licenses/MSRV, feature/direct edges and dated RustSec tree
  observation. Nothing adopted. PNG best-effort decoder limits do not establish
  hard whole-task allocation/cancellation bounds; exact source/graph and actual
  resource proof remain open. Candidate docs/version gaps are explicit.
- Later continuation directly reached the four pinned official docs pages over
  HTTP200 with requested versions advertised; pinned-api-review.json records
  decoded HTML digests and role markers. This closes docs access uncertainty,
  not the dependency source audit, compilation or resource-bound proof. Cargo
  membership and execution allocation still absent at this observation; no
  frozen package source changes or test execution occurred.
- Root atomic integration commit9b064dbd8942d4fad7f1623da97b6b64c0177a7f was
  independently inspected:13 changed files comprise ten frozen package files,
  root Cargo.toml/Cargo.lock and root migration SoT decision. Worktree package
  diff against that exact commit is empty; Git reports both index and worktree
  LF for all ten package files. Their hashes match initial-source.json. The
  owner spec/evidence commit is now authorized separately; no component execution
  allocation has yet been received. Root's offline/locked tree resolution report
  remains a graph observation, not any of the sixteen tests running.
- Frozen ten-file package checkpoint rechecked: zero hash drift. All fourteen
  baseline Go file hashes still match. Sixteen authored tests remain unexecuted.
  Sent frozen handoff and dependency proposal to root. Release misc while root
  reviews/integrates/allocates; reclaim actual free lane before any source change.
- `spec context` ran for SoT, plan and evidence before writing; no governing
  watermark document existed. Draft creation does not grant Rust path ownership.
- Read-only `rg` searches found legacy consumers at cmd/server/server.go,
  cmd/server/resource_handlers.go and helpers.go; no Rust watermark consumer.
- Discovered legacy hazards: metadata-first PNG re-encode discards metadata,
  permissive binary fallback can break semantics, handler serves unmarked src on
  error, CRC mislabeled as ECC, GC-only expiry, ID wrap and invalid minimal PNG.
- No Cargo/Docker/browser/process pilot was run. No test or parity pass claimed.
- Sent the concrete design, limits, compatibility corrections and V1–V5 gates to
  root integration chat. Human cross-chat coordination authority was verified in
  root user messages 01a1b4b1-6334-76a2-bf39-0c247f9ed54e and
  01a1b4e4-6987-7423-9ed3-c0bd6deeaf2b. This is a proposal, not acceptance.
- Source review: fourteen of fourteen current hashes match the baseline and the
  baseline registry digest matches. Scoped `spec check` with `--no-local` passed
  with zero errors/warnings after correcting the evidence relation name. Full
  check failed with72384 errors/14 warnings in the shared/private tree; those
  out-of-scope findings were not modified. No global pass is claimed.

Next reader: AGENTS.md, .agents/lessons/HARD-WON.md, the resource-watermark SoT,
this plan and resource-watermark-design evidence. Root integration chat is
01a1b25e-8e4c-7a70-b7b7-f51a66c58514; source chat is
01a1b63c-3b93-7371-b055-9fc2cec587b8. Out of scope: docs/**, shared Cargo/lock,
toolchain, production scoring, foreign lane edits, tag/publish/OIDC and Go deletion.

Expected verification after allocation: scoped locked package tests, package fmt
and locked Clippy, independent vectors, real resource/browser/WASM pilot, then
owner HTTP pilot and Docker product scenario. `spec check` is a document check,
not a substitute for any of these. Initial design is accepted and A16 pure
component verification passed. Further codec/source slices, dependencies and
actual pilots remain unallocated. The immutable INTEGRATION.md at the initial
checkpoint describes its then-unexecuted state; the later A16 evidence supersedes
that execution status without rewriting the tested source checkpoint.
