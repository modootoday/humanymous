# Resource watermark initial source integration

Root accepted `crates/watermark` / `humanymous-watermark` and the reconstructed
resource-watermark SoT. This is the first source-only slice, not migration parity.
The package manifest is ready for root-owned membership/lock integration, using
only existing workspace hmac, sha2, subtle and zeroize dependencies. It inherits
workspace metadata/lints, edition2024 and Rust1.99.1, and is not publishable.
No root Cargo/lock/toolchain, runtime, HTTP, docs, codecs or policy were edited.

## API and state responsibilities

- `payload::WatermarkKey` adopts nonempty bounded secret bytes in a zeroizing
  allocation. No Debug/Clone. Caller copies and cryptographic-library transient
  state are not guaranteed erased by this wrapper.
- `Payload` preserves exact v1 HMAC preimage and14-byte carrier. New issuance
  rejects NUL identifiers, identifier lengths >1024 and ID0. Constant-time tag
  verification and explicitly marked historical suffix tolerance remain separate.
  The80-bit tag does not bind ID; it is not human/culpability/policy proof.
- `ledger::Ledger` synchronizes one bounded volatile state across clones. Explicit
  nonzero record/reservation TTLs and capacity1..65536 have no default. IDs1 through
  u32::MAX are allocated at most once; cancellation/expiry never recycles IDs.
- `reserve(now)` returns a non-Clone linear guard. Encoding failure, explicit
  `cancel`, panic unwind or ordinary Drop releases only its pending capacity.
  `commit(issuance, now)` validates payload/metadata/ID/age before record creation.
- Runtime supplies one trusted monotonic epoch and nondecreasing instants. A
  globally serialized timestamp source is required across concurrent callers;
  sampling outside synchronization can be rejected as a regression. No wall-clock
  timestamp is invented from Duration. Lookup expiration is age > TTL, retaining
  the historical exact boundary. GC also removes expired reservations and must
  be scheduled by runtime during inactivity for physical retention.
- `Lookup::Live` is privileged record data, not verified trace. Runtime must
  authenticate operators before lookup/exposure, validate recovered MAC, audit
  trace, and enforce encode → commit → response ordering and no-store isolation.
  No scores, bans, erasure, key provisioning or identity authorization here.
- State poisoning fails closed. Drop may recover a poisoned guard solely for
  deleting its own pending entry; it never attributes or revives the owner.

Metadata limits are MIME256 bytes, at most4 distinct nonempty channel names of
64 bytes, optional nonempty keyed IP pseudonym128 bytes. Record payload identifiers
are bounded1024 bytes each. Admission caps need real memory/load evidence; caller
allocations constructed before entry are outside this library's allocation control.
Returned privileged record clones require caller retention discipline too.

## Prepared validation and remaining gates

Payload tests cover literal legacy preimage/endianness, RFC4231 primitive vector,
wrong key/changed context/corrupt tag, explicit suffix tolerance, ID-not-MAC
limitation, NUL ambiguity and identifier/key limits. Ledger tests cover capacity,
pending invisibility, drop/cancel/unwind, exact TTL and GC, failed/late commit,
metadata rejection, clock regression, final-ID exhaustion, concurrent clones and
poisoned-state refusal. These are authored tests, NOT executed evidence.

After root graph integration and explicit allocation: run package check,
`cargo test --locked -p humanymous-watermark`, scoped formatting and locked Clippy
with warnings denied, under assigned target/jobs1/incremental0. Preserve first
failure, exact source/lock/config hashes, terminal footer and drift closure.
No Cargo command, Rust compiler, Docker or pilot has been run for this slice.

V1 baseline14 hashes were checked unchanged. V2 component compilation/tests await
allocation. V3 actual resource/browser/WASM pilot awaits codec implementation,
approved dependencies and allocation. V4 real authenticated HTTP issuance/trace
awaits runtime/authority/audit agreement. V5 Docker product scenario awaits lab
integration. Full goal and all original14 responsibilities remain incomplete.
New PNG/CRC/base32/WASM dependencies require a separate official primary-source
version/checksum/license/advisory proposal before root adoption. No handrolled
cryptography or foreign glue. Legacy source deletion remains root-owned.
