//! Cancellable hold window before a crypto-shred commits (SoT-28 §5/WS3).
//!
//! Port of legacy internal/gate/erasure.go and `RunDueShreds`. An approved
//! erasure is scheduled, not executed: it can be cancelled until the hold
//! window elapses, and only then is the linkage key destroyed and completion
//! evidence produced.
//!
//! The queue is generic over the approval capability `P`. In the runtime, `P`
//! is the `audit::keys::vault::ErasurePermit` issued by the durable signed
//! dual-control ledger, and the executor calls `Vault::shred`; the queue never
//! mints or widens that authority. The operator's handle is resolved to a
//! subject through the vault's reverse index before approval, as in legacy
//! `adminErasure`.
use std::collections::BTreeMap;
use std::time::Duration;

/// Legacy default hold window.
pub const DEFAULT_HOLD: Duration = Duration::from_secs(5 * 60);
/// Most pending erasures; a full queue refuses instead of evicting.
pub const MAX_PENDING: usize = 1024;

/// Resolve an operator-supplied handle to a subject, falling back to the raw
/// value only when the reverse index has no entry (legacy `adminErasure`).
pub fn resolve_subject(handle: &str, resolve: impl Fn(&str) -> Option<String>) -> String {
    resolve(handle).unwrap_or_else(|| handle.to_owned())
}

/// Scheduled erasure metadata (no permit material).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheduled {
    /// Opaque queue id.
    pub id: String,
    /// Resolved subject id.
    pub subject: String,
    /// Requesting principal.
    pub requester: String,
    /// Approving principal (DPO).
    pub approver: String,
    /// Legal basis (for example GDPR Art.17 or PIPA).
    pub legal_basis: String,
    /// Execution time on the caller clock.
    pub execute_at: Duration,
}

/// Completion evidence for one executed erasure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// `erasure.completed`, or `erasure.no_subject` when no key existed.
    pub event_type: &'static str,
    /// Legacy certificate text: `erasure-cert: sth_root=<12> tree_size=<n>
    /// legal_basis=<basis> <requester>-><approver>`.
    pub certificate: String,
    /// The executed schedule.
    pub scheduled: Scheduled,
}

/// Refusals; no subject or permit bytes are interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Subject or legal basis is empty.
    Missing,
    /// Hold window is zero.
    Hold,
    /// [`MAX_PENDING`] erasures are already scheduled.
    Capacity,
}

/// Pending erasures with their approval capabilities.
pub struct HoldQueue<P> {
    hold: Duration,
    next: u64,
    items: BTreeMap<String, (Scheduled, P)>,
}

impl<P> HoldQueue<P> {
    /// Queue with a hold window; zero is refused (an erasure must be cancellable).
    pub fn new(hold: Duration) -> Result<Self, Refusal> {
        if hold.is_zero() {
            return Err(Refusal::Hold);
        }
        Ok(Self {
            hold,
            next: 0,
            items: BTreeMap::new(),
        })
    }

    /// Schedule an approved erasure to execute after the hold window.
    pub fn schedule(
        &mut self,
        subject: &str,
        requester: &str,
        approver: &str,
        legal_basis: &str,
        permit: P,
        now: Duration,
    ) -> Result<Scheduled, Refusal> {
        if subject.is_empty() || legal_basis.is_empty() {
            return Err(Refusal::Missing);
        }
        if self.items.len() >= MAX_PENDING {
            return Err(Refusal::Capacity);
        }
        self.next += 1;
        let scheduled = Scheduled {
            id: format!("shred-{:010}", self.next),
            subject: subject.to_owned(),
            requester: requester.to_owned(),
            approver: approver.to_owned(),
            legal_basis: legal_basis.to_owned(),
            execute_at: now.saturating_add(self.hold),
        };
        self.items
            .insert(scheduled.id.clone(), (scheduled.clone(), permit));
        Ok(scheduled)
    }

    /// Cancel a pending erasure; the permit is dropped unused.
    pub fn cancel(&mut self, id: &str) -> Option<Scheduled> {
        self.items.remove(id).map(|(scheduled, _)| scheduled)
    }

    /// Remove a pending erasure and hand its permit back unused, so a caller
    /// whose audit of the schedule failed can restore the permit.
    pub fn withdraw(&mut self, id: &str) -> Option<(Scheduled, P)> {
        self.items.remove(id)
    }

    /// Whether `id` is pending.
    pub fn contains(&self, id: &str) -> bool {
        self.items.contains_key(id)
    }

    /// Number of pending erasures.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether no erasure is pending.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The configured hold window.
    pub fn hold(&self) -> Duration {
        self.hold
    }

    /// Pending erasures in id order (for the console).
    pub fn list(&self) -> Vec<Scheduled> {
        self.items.values().map(|(s, _)| s.clone()).collect()
    }

    /// Ids, in id order, of the erasures whose window elapsed at `now`.
    pub fn due(&self, now: Duration) -> Vec<String> {
        self.items
            .iter()
            .filter(|(_, (s, _))| now >= s.execute_at)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Execute one pending erasure. `None` when `id` is not pending. An error
    /// from `shred` keeps it pending so a failed shred is retried, never
    /// reported complete. The window is not rechecked; callers take ids from
    /// [`Self::due`].
    pub fn run_one<E>(
        &mut self,
        id: &str,
        checkpoint: (&str, u64),
        shred: impl FnOnce(&str, &P) -> Result<bool, E>,
    ) -> Result<Option<Completion>, E> {
        let Some((scheduled, permit)) = self.items.get(id) else {
            return Ok(None);
        };
        let existed = shred(&scheduled.subject, permit)?;
        let root: String = checkpoint.0.chars().take(12).collect();
        let certificate = format!(
            "erasure-cert: sth_root={root} tree_size={} legal_basis={} {}->{}",
            checkpoint.1, scheduled.legal_basis, scheduled.requester, scheduled.approver
        );
        let Some((scheduled, _)) = self.items.remove(id) else {
            return Ok(None);
        };
        Ok(Some(Completion {
            event_type: if existed {
                "erasure.completed"
            } else {
                "erasure.no_subject"
            },
            certificate,
            scheduled,
        }))
    }

    /// Execute every erasure whose window elapsed at `now`. `shred` returns
    /// whether a key existed; an error keeps that erasure pending and stops the
    /// run so a failed shred is retried, never reported complete. Erasures
    /// executed earlier in a run that then fails are not returned; a caller
    /// that must keep their evidence uses [`Self::due`] and [`Self::run_one`].
    pub fn run_due<E>(
        &mut self,
        now: Duration,
        checkpoint: (&str, u64),
        mut shred: impl FnMut(&str, &P) -> Result<bool, E>,
    ) -> Result<Vec<Completion>, E> {
        let due = self.due(now);
        let mut done = Vec::with_capacity(due.len());
        for id in due {
            if let Some(completion) = self.run_one(&id, checkpoint, &mut shred)? {
                done.push(completion);
            }
        }
        Ok(done)
    }
}

#[cfg(test)]
mod tests;
