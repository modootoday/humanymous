---
id: evidence:root/20261108043304
title: Preimage-bound Core watermark consumer and immutable-input candidate
status: active
created: 2026-11-08T04:33:04Z
updated: 2026-11-08T04:40:38Z
rel:
  depends_on: [sot:root/resource-watermark, sot:root/structural-development, plan:root/20261107135500, evidence:root/20261107144843]
files:
  - {path: candidate/crates/sdk/humanymous-core/src/watermark/mod.rs, sha256: 59ed938e16959a774864d1f5016c946727b6fd0f9673711b1f637426d55cdefb, bytes: 3353, store: repo}
  - {path: candidate/crates/sdk/humanymous-core/tests/watermark.rs, sha256: a5ce56029c83e02d8dce63f921cb46cbfaddeca29837c46d4f72214c7a099950, bytes: 117, store: repo}
  - {path: candidate/crates/sdk/humanymous-core/tests/watermark/fixtures/public-boundary.json, sha256: 10c1e43c846532a7d0e6ddebd4579d6266778ab16ad06de920acd2dc4b67221d, bytes: 656, store: repo}
  - {path: candidate/crates/sdk/humanymous-core/tests/watermark/fixtures/public-boundary.sha256, sha256: 90f2c2d58172c1fb734eb472f5a1e5c87828c390521b594d50533d38d14a5276, bytes: 65, store: repo}
  - {path: candidate-manifest.json, sha256: 1b077e91b715777d1aeb480bf5fc982e6cae35687b6ebc09d4b421b5b91397e0, bytes: 5664, store: repo}
  - {path: candidate/crates/sdk/humanymous-core/tests/watermark/public_boundary.rs, sha256: 945c079d8cf978e30e220849518b0e21838f15a7e7308ac268a30eee4bf21b5d, bytes: 10157, store: repo}
---

# Source-ready candidate, not applied or executed

Root assigned this task against the read-only isolated8 input
`.spec.local/root/evidence/20261108054000-core-consolidation/tools-integration-5f1010d4`.
Actual branch codex/rust was checked before writes. The owner narrowed/reclaimed
misc for the own resource-watermark spec and this unique public/private evidence
prefix only. No source application, branch/worktree/ref operation, manifest,
scoring or Model SGD change was made. No compiler/runtime/Docker/browser/hosted
workload was started and there is no owned target lease or process handle.

## Exact application contract

candidate-manifest.json binds all16 watermark input files, Core manifest/library
context identity and five output hashes. Root must validate every present-input
hash and all absent-target preconditions atomically before applying. One existing
target changes (watermark/mod.rs documentation only), four new files are added.
Stale/partial application must fail closed; do not rewrite inputs to make hashes
match or apply into a different context. Output files are attached under their
exact candidate/crates/sdk/humanymous-core/... destination-relative paths.

| Output | Responsibility |
| --- | --- |
| src/watermark/mod.rs | Public consumer control and8independent compile-fail examples; no runtime behavior change |
| tests/watermark.rs | Context-local external consumer entry, like other Core context tests |
| tests/watermark/public_boundary.rs |10public API regression tests |
| tests/watermark/fixtures/public-boundary.json | Fixed independent IEEE frame bytes, carrier layout and synthetic identity/JSON inputs |
| tests/watermark/fixtures/public-boundary.sha256 | Exact fixture digest, checked by the consumer tests |

The inputs live inside the package and are included by paths relative to the
test file. No repository-root, private evidence, original Go tree, other package,
runtime cwd, environment variable or external download supplies fixture bytes.
The digest is fixture provenance/integrity bookkeeping, not trusted runtime
authority or tamper-proof publication. Existing Core serde/serde_json/sha2/hex
dependencies suffice; no manifest/dependency edit or new package is proposed.

## Meaningful consumer cases

The independent generic frame vector is HMWM/u16be9/ASCII123456789/IEEE CRC32
cbf43926. A valid nine-byte CRC carrier must still fail the strict fourteen-byte
watermark split. Tests enumerate every truncation and every one-bit corruption,
deduplicate equal frames and reject distinct CRC-valid carriers as ambiguous.
Strict/legacy split policies, exact ID/tag layout and ignored suffixes are
explicit. Scalar/object JSON is parsed before/after using the actual existing
serde_json parser; opaque resource prefixes are unchanged.

Identity cases change session/asset/bucket/key independently, retain the known
v1 ID-not-MAC limitation, and reframe a forged MAC with a valid CRC to demonstrate
that framing success is not authentication. Public ledger use requires reserve,
encode and commit before live lookup; cancellation, nonreuse, exact TTL and a
new ledger reusing a numeric ID cannot promote an old session's carrier. MIME
identification for PNG/WASM is not advertised as implemented codec capability.

The positive rustdoc control establishes usable public frame, Record,
Reservation and key names. Separate compile-fail examples protect external
access to Candidates, Budget, unchecked Limits fields, Ledger.inner,
Reservation::new, forged Record literals, secret key bytes and private base32.
They test public consumer access, not arbitrary Core sibling visibility.
Existing context restrictions still need root's internal boundary review.
No high-level grammar-validation, authenticated trace, HTTP or Ready capability
is invented. Public Payload and syntax choices remain caller input.

## Verification status and requested allocation

Direct rustfmt source formatting succeeded. The16read-only watermark preimages
were rehashed unchanged after candidate construction.10external tests,
1positive rustdoc and8compile-fail examples are AUTHORED/NOT_RUN. The earlier
29-unit C29 receipt is historical source-specific evidence, not proof of this
merged context or new tests. Fmt/spec checks do not substitute for execution.

After root applies the exact hash-bound candidate and assigns a bounded native
slot/target, run scoped Core formatting, the watermark:: library cases, test
target watermark, filtered watermark rustdoc tests and strict Clippy for this
consumer target. Observe actual separate unit/integration/rustdoc denominators
and private-boundary diagnostics, preserving first failures. Root owns package
staging/manifest completeness checks and any actual package-local extraction
pilot. No new execution authorization follows from these proposed commands.

The original14responsibility migration remains incomplete: WASM validator/codec,
PNGmetadata/LSB/alpha/resource/cancellation, complete validated apply/recover/trace
and actual file/resource/browser/HTTP/Docker gates are retained. This is a ready
public-boundary/immutable-input candidate, not a narrower definition of success.
