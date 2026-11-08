# Runtime required observer owner interfaces

This is an unapplied, preimage-bound candidate against Root's read-only
tools-integration-5f1010d4. The main checkout and isolated source are untouched.
No local compiler, runtime, browser, Docker, SDK or hosted job was run.

## Existing original Authority interfaces

The baseline owns these types and methods in authority/startup/observer.rs:

```rust
pub fn native_preparation()
    -> Result<VerifiedObserverPreparation, NativePreparationUnavailable>;
impl VerifiedObserverPreparation {
    pub fn effect_gate(&self) -> ObserverEffectGate;
    pub fn cancel(self, cause: LossCause) -> ObserverStartupFailure;
}
impl PreparedObserverSession {
    pub fn effect_gate(&self) -> ObserverEffectGate;
    pub fn cancel(self, cause: LossCause) -> ObserverStartupFailure;
}
```

NativeProof is empty, including in tests. native_preparation returns Unsupported.
There is no native publisher/W/namespace/transport factory. These are original
opaque capabilities, not Runtime flags, signed DATA or Job observations.

## Required original Authority candidate dependency

evidence:root/20261108043317 owns the proposed, UNAPPLIED consuming receipt seam:

```rust
impl VerifiedObserverPreparation {
    pub fn into_prepared(
        self,
        receipt: AuthenticatedObserverReceipt,
    ) -> Result<PreparedObserverSession, ObserverStartupFailure>;
}
```

AuthenticatedObserverReceipt has private gate and empty ReceiptOrigin fields.
It has no constructor, deserializer or test bypass. Authority's own
Owner::accept_receipt verifies actual Arc identity, OfferPending and no loss under
one lock. It rejects foreign owner, wrong phase, replay and revoked epochs.
Runtime MUST NOT duplicate this check or turn DATA into this receipt. Root must
apply/review that dependency before compiling this consumer; it is absent from
the baseline, so this candidate is not standalone-baseline compilable.

Required observer.rs preimage is
c6aefce84793d37becdcc622dea2271277441c02a1f9feec2e4d5184003fc806;
Authority proposed postimage is
8ffb051fe9324532f727a415779f4b3e0bece14f6d7bf28f2b3baa8cb04e7fc2.
State preimage is3778b0c05fd9bf53dd64cd0c616ba45388e4ed157cb77a8e9aaf0e430e52cf21;
proposed postimage66af32b9c0698996baef68999826f485ee9c531a172330a4179369887a81917a.
The Authority manifest remains authoritative for its other paths and hashes.

## Concrete Runtime consumer candidate

The SDK-visible names below are reexported only through gateway::service:

```rust
impl ServiceHost {
    pub fn begin_required_observer(configuration: Configuration)
        -> RequiredObserverStartup;
}
impl RequiredObserverStartup {
    pub fn prepare_native_once(&mut self) -> Result<(), RequiredObserverRefusal>;
    pub fn offer_ack_once(&mut self) -> Result<(), RequiredObserverRefusal>;
    pub fn bind_authenticated_receipt(
        &mut self,
        receipt: AuthenticatedObserverReceipt,
    ) -> Result<(), RequiredObserverRefusal>;
    pub fn cancel(self) -> RequiredObserverFailure;
}
impl RequiredObserverFailure {
    pub fn revoke(&mut self);
    pub fn reason(&self) -> RequiredObserverRefusal;
    pub fn native_unavailable(&self) -> Option<&NativePreparationUnavailable>;
    pub fn authority_failure(&self) -> Option<&ObserverStartupFailure>;
}
```

begin retains the exact boxed Configuration before calling the original provider.
Native unavailability retains its original typed error without unseal, credential
generation, bind or writer opening. Each attempt is spent before the provider is
called. Reuse is terminal, cannot refresh a deadline, and conserves the first
refusal. This is per-attempt bookkeeping, NOT the native process-wide claim.

offer_ack_once explicitly refuses: its actual transport producer is absent. It
does not transmit an Offer, synthesize ACK or pretend the exchange succeeded.
bind_authenticated_receipt consumes ONLY the original opaque receipt, takes the
original preparation once, invokes its consuming into_prepared, and stores the
returned original session or original failure alongside the SAME Box. Authority
owns all epoch/authenticated receipt checks. On error/replay, Runtime revokes and
retains resources. There is no receipt extraction, arbitrary callback or bool.

RequiredObserverStage is diagnostics only, with no Active variant. ReceiptBound
is not Ready or permission to connect an origin. No activate/RuntimeReadyReceipt
factory, credential exposure, cleanup-completion result or new scoring exists.
Existing standalone bind methods are unchanged; they remain experimental private
components, not this required-observer profile or public shipping composition.

## Manufacturer still required before actual Offer and activation

The native owner must supply protected original installer delegation, actual
retained P/S/O identity and artifact/BASE/config bindings, process-wide one-claim,
verified independent W and conserved monotonic/wall deadlines. B2 Job ownership,
containment, primary-token observations and signatures remain observations/DATA;
none constructs VerifiedObserverPreparation, AuthenticatedObserverReceipt or Ready.

One possible ORIGINAL Authority-owned transport interface for joint review is:

```rust
impl VerifiedObserverPreparation {
    pub async fn offer_credential(
        &mut self,
        credential: &crate::authority::credentials::Credential,
    ) -> Result<AuthenticatedObserverReceipt, ObserverOfferError>;
}
```

This signature is a proposal ONLY, not implemented/accepted here. The preparation
must own the actual bounded transport, original epoch/nonce, W and deadline;
cancellation of the borrowed future must irreversibly revoke/poison the exchange.
No caller deadline reset, generic verifier, arbitrary receipt callback, detached
secret writer or false cleanup success is permitted. Repeated Offer must refuse
before secret release. The factory must bind protected configuration/BASE scope;
the current no-argument unavailable seam exposes no such native binding evidence.

Runtime still needs actual retained partial-host/writer/credential resources and
private Ready enqueue/Active linearization under the original gate. Authority
cannot import a Gateway Ready receipt. Missing providers keep activation absent;
this partial consumer integration does NOT complete Offer/ACK/native/Ready work.

## Authored tests and limits

Six units exercise the real unavailable original provider path, exact allocation
and passphrase retention, attempt reuse, Offer-before-preparation, repeated Offer
refusal and idempotent cancellation preserving the first error. They construct no
NativeProof or receipt and are NOT wire replay/native acceptance tests. Existing
guard-before-resource-drop tests remain unchanged. All are UNRUN. The original
Authority candidate owns its same-gate/phase/replay tests; no fake successful
capability is introduced to make this consumer testable.
