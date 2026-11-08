# Actual per-replica CEF and schema-2 consumer candidate

This implements UI-41/42 in Rust against the current read-only Root assembly.
All writes stayed in this Browser session's claimed evidence/spec paths after
checking the actual branch was `codex/rust`. No branch/worktree/ref operation,
shared manifest/lock/workflow change or product-source application occurred.

`cef-replica-consumer-candidate.json` binds five exact original files and two
previously absent destinations. The public patch changes two existing Rust files
and adds two Rust files; the producer contract, CLI replica and provenance JSON
remain byte-identical. Complete postimages and raw preimages are retained under
`.spec.local/root/evidence/20261108151000-browser-ui-replica-consumer/`.
`git apply --check` passed against those exact preserved preimages. These are
reviewable source postimages, not a compiled or executed implementation claim.

## Per-replica producer admission

`crates/apps/humanymous-cli/src/qualification/delivery/qualification_data.rs`
now selects the trusted producer commit per replica before invoking bounded Git
object reads. The CEF source path points to the relocated Browser producer.
The other seven destination/source pairs and their historical commit
`3137c905ac4e52b8b345fa2306af9e4ef4d42b48` are unchanged. Ownership, count-eight,
duplicate, license, bounds, digest, committed-blob and exact-byte checks remain.
The response is `humanymous.cli-qualification-data-check/2` with per-replica
`source_pins`, rather than an inaccurate single common `source_commit`.
The provenance input schema remains version 1 because its existing records already
carry individual producer commits; this does not rewrite the other seven rows.

There is no committed/reviewed relocated producer supplied for this assignment.
Therefore compiled `CEF_PRODUCER_COMMIT` is explicitly `None`, and production DATA
validation refuses CEF admission. No fake commit, zero hash, manifest-controlled
trust or permissive fallback was introduced. Root must first commit/review the
producer, then set that one compiled option and refresh L07's exact replica/blob/
commit/SHA/size provenance together. Preserve the old producer receipt in history.
Do not replace the seven historical pins with Root's new producer commit.
The current stale replica/provenance are retained deliberately, not refreshed
under an unreviewed private path. Tests inject a clearly separate trusted fixture
pin through a private helper; it is not a production source identity.

## Safe schema-2 producer and consumer

New `src/qualification/cef_pilot/receipt.rs` is a bounded context-specific consumer
of `humanymous.cef-safe-diagnostics/2`, using the existing strict JSON decoder.
It requires externally supplied expected source/manifest and distinct role hashes,
the reviewed contract, both separately observed successful role builds, matching
stage receipts and nonempty exact artifact hashes. All eight required command
stages and both protocol counters must be complete without ignored/failed or
truncated/malformed/omitted diagnostics. It requires the four unique bootstrap
override observations, real wait/reap/exit values, unchanged profile inventories
and a reported desktop pass count of two. Old/private schemas, the former aggregate, unknown roles/
stages, source drift, missing/failed fields and unsupported admission flags refuse.

The result remains a bounded receipt check: BrowserReady, full runtime/product,
hardware GPU and GUI keyboard/IME/AX qualification are explicitly false. This is
neither a signed attestation nor an independent OS process observation. Consumers
must derive expectations from their reviewed source/bundle context, not copy them
from the receipt they are checking. The producer's internal use checks consistency
against its already captured source and packaged artifact map; it does not create
an additional independent execution denominator. Root consumers must also require
the actual terminal job result and a fresh owned output directory; this pure
function does not observe job execution or establish absolute receipt freshness.

`cef_pilot/mod.rs` wires this consumer before emitting any passed receipt. It now
writes a create-new private `report.json`, retains its digest, and writes an actual
create-new `safe-report.json` from the existing safe projection. Failed attempts
can still emit sanitized diagnostics; the consumer does not count them as passed.
Neither file is overwritten as a new attempt. Raw errors, actual compiler-ledger
paths and package data remain private. Delivery must separately update its owned
workflow to upload the safe file; this patch does not change that workflow or
assert that the current private-report upload path is already corrected.

## Meaningful authored refusal fixtures

Eight new tests cover:

- CEF-only epoch changes, missing compiled admission and every unreviewed row pin.
- Actual seven historical provenance records without assuming record ordering.
- Replica digest, producer blob and exact producer-byte mismatch, including a
  rewritten local digest that must not legitimize different producer bytes.
- A complete schema-2 fixture that stays a bounded component result.
- Old/private schemas, old aggregate, wrong source/manifest, missing/failed role,
  wrong/duplicated artifact slot, ignored cases, unknown roles/stages, private
  contamination and falsely claimed readiness.
- Truncated, unreaped, timed-out, duplicate or erroneous bootstrap observations
  and changed/missing profile or test outcomes.
- Stale contract selectors, duplicate JSON keys and oversized reports.
- A real temporary-file safe-projection/no-private-canary/no-overwrite test with
  explicit two-file/directory cleanup; no child process or native host is started.

The two original DATA refusal tests are preserved and now use the explicit trusted
fixture pin. No test, formatter, compiler, Clippy, metadata command, SDK/DLL build,
native/process/browser or hosted allocation ran. Pure fixture acceptance must not
be described as CEF/IME/AX/sandbox execution or full product e2e.

## Root adoption and requested scoped allocation

1. Compare every preimage hash and absent destination; apply only in Root's owned
   assembly. Inspect/format the four Rust postimages. No live application is
   authorized by this delegate's patch.
2. Allocate cached scoped tests/strict checks for package `humanymous-cli`, bin
   `humanymous-delivery`, feature `qualification-delivery`, selecting
   `delivery::qualification_data::tests::`; and bin `humanymous-cef-pilot`, feature
   `qualification-cef-pilot`, selecting `cef_pilot::receipt::tests::` plus the exact
   `cef_pilot::tests::shareable_report_uses_safe_projection_and_refuses_overwrite`.
   Keep `--locked --no-default-features` and source-before/after evidence. Root
   decides the actual allocation; this is a request, not an execution grant.
3. Commit/review the producer, finalize the pending compiled CEF pin and exact L07
   replica/provenance, then execute the DATA equality gate. Until then its refusal
   is intentional and cannot be listed as a passing packaged-data gate.
4. Root/Delivery migrate DATA check schema-2 consumers and shareable artifact
   wiring, then rejoin final source/lock/metadata/compiler-role receipts before
   separately allocated SDK/PE and real hosted/native pilots. CEF pins, sandbox,
   CPU defaults and all unfinished UI/IME/AX/DPI/distribution goals remain intact.
