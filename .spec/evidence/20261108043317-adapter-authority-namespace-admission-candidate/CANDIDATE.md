# Authority original namespace/channel/W candidate

Status: authored, not applied, not compiled or executed. Production admission is
still Unsupported. The main tree and Root's isolated tools-integration-5f1010d4
workspace were read only. No backend, Cargo, scoring or visibility relocation was
authored. Apply only through Root after every SHA256 preimage matches manifest.json.

The implementation skill implement-sot-slice informs this bounded candidate, with
execution explicitly deferred by Root's resource allocation rule. Existing public
authorization/2 records/signatures remain unchanged. The additional challenge
transcript domain admission-candidate/1 and original-record commitment are proposed
DATA policy, requiring Root/Native/Audit review before any production protocol use.
They do not establish protected origin, native peer authentication or W liveness.
No user-facing public protocol/package is published by this evidence.

## Implemented candidate logic

Original namespace record commitment is fixed-order and bounded, checked against
independent namespace/generation/digest selection AND the already strict verified
original installer key, installation epoch, exact delegation, ID and complete scope.
No reply-selected key/delegation or automatic record-generation renewal is allowed.
The public record/pin inputs are explicitly DATA, never a trusted store-read receipt.

Peer and independent W sign different role-tagged challenge transcripts with strict
Ed25519 verification under independently selected keys. Both transcripts bind the
original record, delegation, grant, credential epoch, BOTH process-role commitments,
channel ID, BOTH nonces, W SID/PID/creation/epoch/key/lease/expiry. Invalid lengths,
zero/aliased nonces/roles/keys, stale/future issuance and overlong W lifetime refuse.
This does not demonstrate native TokenUser/retained handles/CSPRNG/actual transport.
No parser/allocation is driven by a response-selected key or declared wire length.

An explicitly EPHEMERAL bounded policy ledger atomically reserves only after valid
signatures. It retains tombstones after drop/cancel, refuses same process epoch under
a fresh grant, nonce/channel/grant replay, capacity exhaustion and ALL recovery
cursors. Absolute monotonic deadline and wall rollback/expiry are latched; reservation
rechecks witness expiry and exact signed startup cap. It has no disk/OS lock and
cannot qualify restart safety. Recreating a ledger cannot create native admission:
native_admission always returns NativeUnsupported/Infallible, never a proof.

## Consuming Runtime/CLI seam

Proposed exact signature:

```rust
VerifiedObserverPreparation::into_prepared(
    self, receipt: AuthenticatedObserverReceipt,
) -> Result<PreparedObserverSession, ObserverStartupFailure>
```

AuthenticatedObserverReceipt fields stay private, including same-epoch gate and
PRIVATE EMPTY ReceiptOrigin. There is NO constructor/Clone/Default/Deserialize and
no production/test factory. Original NativeProof also remains PRIVATE EMPTY and
native_preparation stays Unsupported. Existing state Owner adds an observer-only
accept_receipt check: pointer-identical epoch, exact OfferPending phase, no first
loss, one synchronized transition to ReceiptBound. Foreign/replayed/wrong-phase/
revoked receipt refuses; consuming failure revokes the preparation. No Ready or
activation is possible. Pure state tests never construct either opaque production
type. ReceiptOrigin MUST NOT become inhabited until retained actual authenticated
channel, actual credential ACK and W/monitor lifetime ownership are jointly reviewed;
the current receipt type does not pretend to hold those unavailable OS resources.

Runtime is only the consuming owner of PreparedObserverSession and its own private
Ready/resource evidence. CLI remains DATA-only until a genuine producer exists.
Native confirmed B2 only owns low-level containment/process/thread cleanup; no
original namespace/delegation/one-claim/native peer/W admission provider exists.
No B2 facade is copied or treated as such a producer.

## Authored tests and actual missing execution

Sixteen new cases: fifteen signature/original/replay/recovery/time/capacity/atomic
policy cases plus one same-epoch/phase/revocation model test. The positive DATA case
explicitly requires NativeUnsupported and the unchanged unsupported native producer.
Test signatures use public deterministic test-only seeds and a separate transcript
encoder; they are not independent cryptographic interoperability/native pilot proof.
No cases are reported as passed. Source-only inspection found and corrected a
candidate module-path integration issue before execution; no compiler failure or
pass is fabricated. Scoped check/unit/fmt/strict Clippy and actual native/hosted
proofs remain NOT_RUN; Root allocation requested, none assumed.

Existing historical 37-unit and native receipts remain bound to their old source.
Original protected installer/SCM record and delegation provisioning, actual process/
TokenUser/rights/launch-file ownership, durable one-claim lock/recovery, live channel
and independent W cancellation/JOIN, framed aggregate startup, secret receipt,
Runtime Ready and final hosted/Docker gates are OPEN. This candidate closes none
of these production evidence gates.

## Independent source review correction

Initial candidate1fe41ab4 had two bounded DATA-policy defects. Trusted monotonic/
wall samples now advance before Replay/Capacity refusals, with rollback permanently
latched. Replay process identity now excludes role/plan and binds SID/PID/creation/
epoch, compares both positions; complete role/plan transcript binding is retained.
Three regression tests cover four rollback/refusal combinations, changed signed
base plans under fresh grants/challenges and swapped signed role identities.
These fixes are authored, NOT_RUN; native_admission still always Unsupported.
