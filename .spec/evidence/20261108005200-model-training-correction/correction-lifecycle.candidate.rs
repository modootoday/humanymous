//! Offline ledger transitions only: NEVER installs models or changes thresholds.
//! Cryptographic/key custody and authenticated approvers belong to real authority
//! adapters. Test adapters are mocks, not signing or operational admission proof.
use super::Error;
use super::gates::GateReport;
use crate::behavior::features::schema_hash;
use crate::model::inference::MlpBundle;
use sha2::{Digest, Sha256};

pub(crate) type ModelDigest = [u8; 32];
const MAX_BYTES: usize = 1 << 20;

pub(crate) trait SignatureVerifier {
    /// Verify the ORIGINALexact bytes using an externally provisioned trust root.
    /// Return a bounded signer-key identity only on successful real verification.
    fn verify(&self, bytes: &[u8], signature: &[u8; 64]) -> Result<ModelDigest, Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StagePolicy { UnsignedOffline, RequireSignature }
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Identity { pub(crate) version: String, pub(crate) digest: ModelDigest, pub(crate) signer: Option<ModelDigest> }
#[derive(Debug, Clone, PartialEq)]
struct Candidate { identity: Identity, bytes: Vec<u8>, gate: Option<GateReport> }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action { PromoteOffline, RollbackOffline }

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Binding {
    pub(crate) action: Action, pub(crate) generation: u64,
    pub(crate) from: Option<ModelDigest>, pub(crate) to: Option<ModelDigest>,
    pub(crate) evidence: Option<ModelDigest>,
    pub(crate) expires: u64,
}
impl Binding {
    pub(crate) fn digest(&self) -> ModelDigest {
        let mut hash = Sha256::new();
        hash.update(b"humanymous.model.offline-lifecycle.v1\0");
        hash.update([match self.action { Action::PromoteOffline => 1, Action::RollbackOffline => 2 }]);
        hash.update(self.generation.to_be_bytes());
        for identity in [self.from, self.to, self.evidence] {
            hash.update([u8::from(identity.is_some())]);
            hash.update(identity.unwrap_or([0; 32]));
        }
        hash.update(self.expires.to_be_bytes());
        hash.finalize().into()
    }
}

pub(crate) trait ApprovalAuthority {
    /// External authenticated authority binds BOTHproofs to the exact action
    /// digest. No boolean "operator approved" flag is accepted by the ledger.
    fn verify(&self, action_digest: ModelDigest, proof: &[u8]) -> Result<String, Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Receipt {
    pub(crate) binding: Binding, pub(crate) next_generation: u64,
    pub(crate) reviewers: [String; 2],
    pub(crate) activation: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Ledger {
    selected: Option<Candidate>, previous: Option<Candidate>, staged: Option<Candidate>,
    generation: u64, last_now: u64,
}
impl Ledger {
    pub(crate) fn selected(&self) -> Option<&Identity> { self.selected.as_ref().map(|x| &x.identity) }
    pub(crate) fn staged(&self) -> Option<&Identity> { self.staged.as_ref().map(|x| &x.identity) }
    pub(crate) fn generation(&self) -> u64 { self.generation }
    /// Read/hash/decode ONEbuffer. Required digest is a deliberate stronger
    /// boundary than Go's optional digest; unsigned remains offline-only data.
    pub(crate) fn stage(&mut self, bytes: &[u8], expected: ModelDigest, policy: StagePolicy,
        signature: Option<&[u8; 64]>, verifier: Option<&dyn SignatureVerifier>) -> Result<Identity, Error>
    {
        if bytes.is_empty() || bytes.len() > MAX_BYTES { return Err(Error::Capacity); }
        let generation = self.generation.checked_add(1).ok_or(Error::Capacity)?;
        let digest: ModelDigest = Sha256::digest(bytes).into();
        if digest != expected { return Err(Error::Digest); }
        let signer = match policy {
            StagePolicy::UnsignedOffline => {
                if signature.is_some() || verifier.is_some() { return Err(Error::Config); }
                None
            }
            StagePolicy::RequireSignature => Some(verifier.ok_or(Error::Signature)?
                .verify(bytes, signature.ok_or(Error::Signature)?).map_err(|_| Error::Signature)?),
        };
        let bundle: MlpBundle = serde_json::from_slice(bytes).map_err(|_| Error::Shape)?;
        if bundle.validate().is_err() || bundle.hidden > 256 || bundle.version.is_empty()
            || bundle.version.len() > 128 { return Err(Error::Shape); }
        if bundle.schema_hash != schema_hash() { return Err(Error::Schema); }
        if bundle.w1.iter().flatten().chain(&bundle.b1).chain(&bundle.w2)
            .chain(&bundle.mean).chain(&bundle.std)
            .chain([&bundle.b2, &bundle.cal_a, &bundle.cal_b]).any(|x| !x.is_finite())
        { return Err(Error::Nonfinite); }
        let identity = Identity { version: bundle.version, digest, signer };
        let mut owned = Vec::new(); owned.try_reserve_exact(bytes.len()).map_err(|_| Error::Allocation)?;
        owned.extend_from_slice(bytes);
        self.staged = Some(Candidate { identity: identity.clone(), bytes: owned, gate: None });
        self.generation = generation;
        Ok(identity)
    }
    pub(crate) fn attach_gate(&mut self, report: &GateReport) -> Result<(), Error> {
        let staged = self.staged.as_ref().ok_or(Error::NoStaged)?;
        if report.model() != staged.identity.digest { return Err(Error::Digest); }
        if !report.allowed() { return Err(Error::Support); }
        let generation = self.generation.checked_add(1).ok_or(Error::Capacity)?;
        let report = report.clone();
        self.staged.as_mut().ok_or(Error::NoStaged)?.gate = Some(report);
        self.generation = generation;
        Ok(())
    }
    pub(crate) fn preview(&self, action: Action, expires: u64) -> Result<Binding, Error> {
        let to = match action {
            Action::PromoteOffline => {
                let staged = self.staged.as_ref().ok_or(Error::NoStaged)?;
                if staged.identity.signer.is_none() { return Err(Error::Unsigned); }
                if staged.gate.as_ref().is_none_or(|g| !g.allowed()) { return Err(Error::Missing); }
                Some(staged.identity.digest)
            }
            Action::RollbackOffline => {
                if self.selected.is_none() && self.previous.is_none() { return Err(Error::NoPrevious); }
                self.previous.as_ref().map(|x| x.identity.digest)
            }
        };
        Ok(Binding { action, generation: self.generation,
            from: self.selected.as_ref().map(|x| x.identity.digest), to,
            evidence: match action {
                Action::PromoteOffline => self.staged.as_ref().and_then(|x| x.gate.as_ref()).map(GateReport::evidence),
                Action::RollbackOffline => self.previous.as_ref().and_then(|x| x.gate.as_ref()).map(GateReport::evidence),
            }, expires })
    }
    pub(crate) fn commit(&mut self, request: &Binding, proofs: [&[u8]; 2], authority: &dyn ApprovalAuthority,
        now: u64) -> Result<Receipt, Error>
    {
        if now < self.last_now { return Err(Error::ClockRegression); }
        if request.expires <= now || request.expires - now > 300 { return Err(Error::Expired); }
        let current = self.preview(request.action, request.expires)?;
        if &current != request { return Err(Error::Generation); }
        if proofs.iter().any(|p| p.is_empty() || p.len() > 8192) { return Err(Error::Authority); }
        let next_generation = self.generation.checked_add(1).ok_or(Error::Capacity)?;
        let action_digest = request.digest();
        let a = authority.verify(action_digest, proofs[0]).map_err(|_| Error::Authority)?;
        let b = authority.verify(action_digest, proofs[1]).map_err(|_| Error::Authority)?;
        if [a.as_str(), b.as_str()].iter().any(|s| s.is_empty() || s.len() > 128) { return Err(Error::Authority); }
        if a == b { return Err(Error::SamePrincipal); }
        // No external callbacks after the first mutation; state transition is
        // serialized by &mutself. It changes only this offline ledger.
        match request.action {
            Action::PromoteOffline => { self.previous = self.selected.take(); self.selected = self.staged.take(); }
            Action::RollbackOffline => { self.selected = self.previous.take(); }
        }
        self.generation = next_generation; self.last_now = now;
        Ok(Receipt { binding: request.clone(), next_generation, reviewers: [a, b], activation: false })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanaryState { Off, Armed, Tripped, Graduated }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TripReason { HumanFp, Drift, PassAnomaly }
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Canary {
    pub(crate) state: CanaryState, pub(crate) reason: Option<TripReason>,
    max_fp: f32, probation: u64, on_drift: bool, on_pass: bool, armed_at: u64,
}
impl Canary {
    pub(crate) fn new(max_fp: f32, probation: u64, on_drift: bool, on_pass: bool) -> Result<Self, Error> {
        if !max_fp.is_finite() { return Err(Error::Nonfinite); }
        let max_fp = if max_fp <= 0.0 || max_fp >= 1.0 { 0.02 } else { max_fp };
        let probation = if probation == 0 { 200 } else { probation };
        if probation > super::MAX_OBSERVATIONS { return Err(Error::Capacity); }
        Ok(Self { state: CanaryState::Off, reason: None, max_fp, probation, on_drift, on_pass, armed_at: 0 })
    }
    pub(crate) fn arm(&mut self, humans: u64) -> Result<(), Error> {
        if humans > super::MAX_OBSERVATIONS { return Err(Error::Capacity); }
        self.armed_at = humans; self.reason = None; self.state = CanaryState::Armed; Ok(())
    }
    /// Returns a one-shot rollback PROPOSAL, never invokes a runtime callback.
    pub(crate) fn evaluate(&mut self, humans: u64, fp: f32, drift: bool, pass: bool) -> Result<Option<TripReason>, Error> {
        super::probability(fp)?;
        if self.state != CanaryState::Armed { return Ok(None); }
        let elapsed = humans.checked_sub(self.armed_at).ok_or(Error::ClockRegression)?;
        if humans > super::MAX_OBSERVATIONS { return Err(Error::Capacity); }
        let reason = if elapsed >= 20 && fp > self.max_fp { Some(TripReason::HumanFp) }
            else if self.on_drift && drift { Some(TripReason::Drift) }
            else if self.on_pass && pass { Some(TripReason::PassAnomaly) } else { None };
        if let Some(reason) = reason { self.state = CanaryState::Tripped; self.reason = Some(reason); return Ok(Some(reason)); }
        if elapsed >= self.probation { self.state = CanaryState::Graduated; }
        Ok(None)
    }
}
