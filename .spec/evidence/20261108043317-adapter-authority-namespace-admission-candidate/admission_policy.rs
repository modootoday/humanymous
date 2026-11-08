//! Candidate DATA admission checks. No native ownership, protected store or Ready.
//! The transcript is a review proposal, not a replacement accepted wire protocol.
use super::{SignatureCheckedAuthorization, binding};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};
use std::{convert::Infallible, sync::{Arc, Mutex}, time::{Duration, Instant}};

/// Static refusals; never include packets, keys, paths or credentials.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Original independently selected namespace/record changed.
    Original,
    /// Invalid or aliased expected DATA.
    Binding,
    /// Strict signature verification failed.
    Signature,
    /// Original time budget, wall ordering or witness expiry failed.
    Expired,
    /// A grant, nonce or process instance was already reserved.
    Replay,
    /// Fixed tombstone capacity is full; nothing is evicted.
    Capacity,
    /// Poisoned policy state cannot be recovered as active.
    Poisoned,
    /// No replay/recovery of a previous attempt is permitted.
    Recovery,
    /// Actual namespace/channel/W/one-claim backend is not available.
    NativeUnsupported,
}

/// Untrusted projection of an original record; not a protected-file capability.
pub struct OriginalNamespaceData {
    /// Stable independently selected namespace digest.
    pub namespace: [u8; 32],
    /// Explicit protected-record generation, never an inferred renewal counter.
    pub generation: u64,
    /// Original installer verification key.
    pub installer_key: [u8; 32],
    /// Original installation epoch.
    pub installation_epoch: [u8; 16],
    /// Complete selected delegation artifact commitment.
    pub delegation: [u8; 32],
    /// Selected delegation identifier.
    pub delegation_id: [u8; 32],
    /// Complete configured scope commitment.
    pub scope: [u8; 32],
}

/// Independent selection DATA; a caller cannot turn this into native provenance.
pub struct OriginalPinData {
    /// Namespace selected independently of record/reply content.
    pub namespace: [u8; 32],
    /// Selected exact generation; no automatic rotation.
    pub generation: u64,
    /// Independently selected record digest.
    pub record_digest: [u8; 32],
}

/// Exact bounded record commitment. This is not a signature or an OS store read.
pub fn original_digest(record: &OriginalNamespaceData) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"humanymous.observer-bootstrap.admission-candidate/1/original\0");
    h.update(record.namespace);
    h.update(record.generation.to_be_bytes());
    h.update(record.installer_key);
    h.update(record.installation_epoch);
    h.update(record.delegation);
    h.update(record.delegation_id);
    h.update(record.scope);
    h.finalize().into()
}

/// Independently selected witness DATA, not a retained process or transport owner.
pub struct WitnessData {
    /// Bounded canonical SID projection, not actual TokenUser evidence.
    pub sid: Vec<u8>,
    /// Selected PID projection, not a retained handle.
    pub pid: u32,
    /// Exact native creation-time projection.
    pub creation: u64,
    /// Selected process epoch.
    pub epoch: [u8; 16],
    /// Independently selected W verification key.
    pub key: [u8; 32],
    /// Selected one-lease identifier.
    pub lease: [u8; 32],
    /// Exact witness expiry, bounded by signed authorization.
    pub not_after_ms: u64,
}

/// Selected challenge context; keys and nonces are never selected from a reply.
pub struct ChannelPolicyData {
    /// Selected channel key, not taken from the reply.
    pub peer_key: [u8; 32],
    /// Selected channel instance identifier.
    pub channel: [u8; 32],
    /// Selected nonzero peer challenge; entropy origin is not established here.
    pub peer_nonce: [u8; 16],
    /// Separate selected nonzero W challenge.
    pub witness_nonce: [u8; 16],
    /// Independently selected W context.
    pub witness: WitnessData,
}

/// Public response signatures only. No claimed peer name, key or W selector.
pub struct ResponseData {
    /// Strict peer signature over the role1 transcript.
    pub peer_signature: [u8; 64],
    /// Strict W signature over the role2 transcript.
    pub witness_signature: [u8; 64],
}

fn verify(key: &[u8; 32], signature: &[u8; 64], message: &[u8]) -> Result<(), Refusal> {
    VerifyingKey::from_bytes(key)
        .map_err(|_| Refusal::Signature)?
        .verify_strict(message, &Signature::from_bytes(signature))
        .map_err(|_| Refusal::Signature)
}

fn transcript(
    role: u8,
    checked: &SignatureCheckedAuthorization,
    original: &OriginalNamespaceData,
    policy: &ChannelPolicyData,
) -> Vec<u8> {
    // Fixed order, BE numbers, one bounded SID. No JSON or alternate encodings.
    let mut bytes = b"humanymous.observer-bootstrap.admission-candidate/1/challenge\0".to_vec();
    bytes.push(role); // 1: channel peer, 2: independent W; reflection must fail.
    bytes.extend_from_slice(&original_digest(original));
    bytes.extend_from_slice(checked.delegation_commitment());
    bytes.extend_from_slice(checked.grant_id());
    bytes.extend_from_slice(checked.credential_epoch());
    bytes.extend_from_slice(&binding::role(checked.service()));
    bytes.extend_from_slice(&binding::role(checked.observer()));
    bytes.extend_from_slice(&policy.channel);
    bytes.extend_from_slice(&policy.peer_key);
    bytes.extend_from_slice(&policy.peer_nonce);
    bytes.extend_from_slice(&policy.witness_nonce);
    let w = &policy.witness;
    bytes.push(w.sid.len() as u8); // checked <=68 before this encoder is called.
    bytes.extend_from_slice(&w.sid);
    bytes.extend_from_slice(&w.pid.to_be_bytes());
    bytes.extend_from_slice(&w.creation.to_be_bytes());
    bytes.extend_from_slice(&w.epoch);
    bytes.extend_from_slice(&w.key);
    bytes.extend_from_slice(&w.lease);
    bytes.extend_from_slice(&w.not_after_ms.to_be_bytes());
    bytes
}

/// Successful cryptographic DATA review, explicitly not native authentication.
pub struct ReviewedData {
    grant: [u8; 32],
    service: [u8; 32],
    observer: [u8; 32],
    channel: [u8; 32],
    peer_nonce: [u8; 16],
    witness_nonce: [u8; 16],
    wall_ms: u64,
    expiry_ms: u64,
    startup_cap_ms: u32,
}

/// Check original record/delegation, both instance bindings and fresh challenge DATA.
/// Native protection, actual TokenUser/process retention, CSPRNG origin and W
/// cancellation/JOIN remain independent mandatory inputs, not inferred here.
pub fn review_data(
    checked: &SignatureCheckedAuthorization,
    original: &OriginalNamespaceData,
    pin: &OriginalPinData,
    policy: &ChannelPolicyData,
    response: &ResponseData,
    now_ms: u64,
) -> Result<ReviewedData, Refusal> {
    let d = &checked.records.delegation.claims;
    if !binding::nonzero(&original.namespace)
        || original.generation == 0
        || original.namespace != pin.namespace
        || original.generation != pin.generation
        || original_digest(original) != pin.record_digest
        || original.installer_key != d.installer_key
        || original.installation_epoch != d.installation_epoch
        || original.delegation != *checked.delegation_commitment()
        || original.delegation_id != d.id
        || original.scope != binding::scope(checked.scope())
    {
        return Err(Refusal::Original);
    }
    let w = &policy.witness;
    let p = &checked.records.prepared.claims;
    if !binding::valid_sid(&w.sid)
        || w.pid == 0 || w.creation == 0
        || !binding::nonzero(&w.epoch) || !binding::nonzero(&w.lease)
        || !binding::nonzero(&policy.channel)
        || !binding::nonzero(&policy.peer_nonce)
        || !binding::nonzero(&policy.witness_nonce)
        || policy.peer_nonce == policy.witness_nonce
        || w.sid == checked.service().sid || w.sid == checked.observer().sid
        || w.sid == d.publisher_sid
        || w.pid == checked.service().pid || w.pid == checked.observer().pid
        || w.pid == p.publisher_pid
        || w.key == policy.peer_key || w.key == d.installer_key || w.key == d.publisher_key
        || policy.peer_key == d.installer_key || policy.peer_key == d.publisher_key
    {
        return Err(Refusal::Binding);
    }
    if now_ms < checked.records.commit.claims.issued
        || now_ms >= checked.not_after_ms()
        || now_ms >= w.not_after_ms
        || w.not_after_ms > checked.not_after_ms()
    {
        return Err(Refusal::Expired);
    }
    verify(&policy.peer_key, &response.peer_signature, &transcript(1, checked, original, policy))?;
    verify(&w.key, &response.witness_signature, &transcript(2, checked, original, policy))?;
    Ok(ReviewedData {
        grant: *checked.grant_id(), service: binding::role(checked.service()),
        observer: binding::role(checked.observer()), channel: policy.channel,
        peer_nonce: policy.peer_nonce, witness_nonce: policy.witness_nonce, wall_ms: now_ms,
        expiry_ms: w.not_after_ms.min(checked.not_after_ms()),
        startup_cap_ms: checked.startup_cap_ms(),
    })
}

struct LedgerState {
    entries: Vec<ReviewedData>,
    last_mono: Instant,
    last_wall: u64,
    expired: bool,
}

/// Bounded EPHEMERAL policy ledger, NOT OS-backed one-claim/recovery authority.
/// Constructing a fresh ledger cannot authorize any native preparation.
pub struct ReviewLedger {
    state: Arc<Mutex<LedgerState>>,
    deadline: Instant,
    capacity: usize,
    budget: Duration,
}

/// Reservation DATA only. Dropping it never removes the replay tombstone.
pub struct ReviewReservation {
    _state: Arc<Mutex<LedgerState>>,
}

impl ReviewLedger {
    /// Preserve one original deadline; no frame/retry may renew it.
    pub fn new(capacity: usize, now: Instant, budget: Duration, wall_ms: u64) -> Result<Self, Refusal> {
        if !(1..=128).contains(&capacity) || budget.is_zero() {
            return Err(Refusal::Binding);
        }
        let deadline = now.checked_add(budget).ok_or(Refusal::Expired)?;
        Ok(Self { state: Arc::new(Mutex::new(LedgerState {
            entries: Vec::with_capacity(capacity), last_mono: now, last_wall: wall_ms,
            expired: false,
        })), deadline, capacity, budget })
    }

    /// No serialized cursor, cache or successful prior ACK can recover an attempt.
    pub fn recover(_previous: &[u8]) -> Result<Self, Refusal> {
        Err(Refusal::Recovery)
    }

    /// Only reviewed signatures reach reservation, so invalid peers cannot burn it.
    /// Matching either process epoch forbids republishing under a different grant.
    pub fn reserve(&self, data: ReviewedData, now: Instant, wall_ms: u64) -> Result<ReviewReservation, Refusal> {
        let mut state = self.state.lock().map_err(|_| Refusal::Poisoned)?;
        if state.expired || now < state.last_mono || now >= self.deadline
            || wall_ms < state.last_wall || wall_ms < data.wall_ms || wall_ms >= data.expiry_ms
            || self.budget > Duration::from_millis(u64::from(data.startup_cap_ms))
        {
            state.expired = true;
            return Err(Refusal::Expired);
        }
        if state.entries.iter().any(|old| old.grant == data.grant
            || old.service == data.service || old.observer == data.observer
            || old.channel == data.channel || old.peer_nonce == data.peer_nonce
            || old.witness_nonce == data.witness_nonce || old.peer_nonce == data.witness_nonce
            || old.witness_nonce == data.peer_nonce)
        {
            return Err(Refusal::Replay);
        }
        if state.entries.len() == self.capacity { return Err(Refusal::Capacity); }
        state.last_mono = now;
        state.last_wall = wall_ms;
        state.entries.push(data);
        Ok(ReviewReservation { _state: self.state.clone() })
    }
}

/// No DATA-to-proof factory: even successful DATA reservation remains unsupported.
/// Native must supply original namespace, channel, one-claim and independent W owners.
pub fn native_admission(_review: &ReviewReservation) -> Result<Infallible, Refusal> {
    Err(Refusal::NativeUnsupported)
}
