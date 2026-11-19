//! Erasure commit, list, cancel and due execution of legacy
//! internal/gate/admin.go (`adminErasure` handle resolution, the `erasure`
//! case of `adminApprove`, `adminListErasures`, `adminCancelErasure`) and
//! gate.go (`RunDueShreds`, `emitErasureCertificate`) at baseline 981d25c.
//!
//! [`AdminPlane::with_erasure`] attaches a scope-bound reference [`Vault`] and
//! a [`HoldQueue`]. Without it the erasure request stays a pending action, an
//! approved erasure answers 409 and the `erasures` routes answer the plain
//! 404, as before.
//!
//! Kept from Go:
//! - The requested subject is resolved through the vault reverse index before
//!   the pending action is created; an unindexed value is already a subject.
//! - A DPO-approved erasure is scheduled, never shredded at once:
//!   `approval.granted` records `erasure scheduled (cancellable hold); id=<id>
//!   legal_basis=<basis>` and the body is `{"committed":"erasure",
//!   "executeAtUnix":<unix + hold>,"ok":true,"scheduled":true,"shredId":<id>}`.
//! - `GET erasures` (any role) lists `id`, `legalBasis`, `requester`,
//!   `approver` and `executesInSec` (whole seconds to the window end,
//!   truncated toward zero) with `count`.
//! - `POST erasures/{id}/cancel` (operator or DPO) matches Go `adminWrapped`
//!   (prefix and suffix, so `erasures/cancel` carries id `cancel`); an unknown
//!   id answers `404 no such scheduled erasure`, a known one records
//!   `erasure.cancelled` and answers `{"cancelled":<id>,"ok":true}`.
//! - [`AdminPlane::run_due_shreds`] shreds every erasure whose window elapsed
//!   and records `erasure.completed` (or `erasure.no_subject` when the vault
//!   never keyed the subject) with the certificate text, as the approver.
//!
//! Deliberate fail-closed differences from Go, each pinned by tests:
//! - Two bearer principals alone never erase. The commit also consumes an
//!   `audit::keys::vault::ErasurePermit` issued by the durable signed
//!   dual-control ledger (`ControlAuthority::issue_erasure`) and staged with
//!   [`AdminPlane::retain_erasure_permit`] for that exact subject; without one
//!   the commit answers 409 and nothing is scheduled. Shredding goes through
//!   `Vault::shred`, which verifies the permit again.
//! - The schedule and cancel records are appended before their effect; an
//!   audit failure answers 503, schedules or cancels nothing and keeps the
//!   permit staged. A completion record that cannot be appended after its
//!   shred stops the run with an error (the shred is not undone).
//! - A vault reverse-index failure answers 503 instead of using the raw value.
//!   A shred the vault refuses keeps that erasure pending and stops the run.
//! - Ids are the queue's sequential `shred-NNNNNNNNNN` (Go: 20 random hex
//!   digits) and the list is in id order (Go: map order).
//! - The certificate's signed head is supplied by the caller (the latest
//!   checkpoint of the chain the records go to); the plane's audit sink does
//!   not expose one.
//! - Persisting the vault snapshot after a shred is the caller's obligation
//!   (the reference vault is in memory, like Go's).
use super::{AdminPlane, MAX_STAGED_PERMITS, PermitRefusal, Response, audit_unavailable};
use crate::audit::keys::vault::{ErasurePermit, Vault};
use crate::gateway::admin_approval::Operator;
use crate::gateway::operations::erasure_hold::{HoldQueue, MAX_PENDING, Refusal};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// Erasure state attached to the plane.
pub(super) struct Erasure {
    vault: Arc<Vault>,
    queue: Mutex<HoldQueue<ErasurePermit>>,
    staged: Mutex<Vec<(String, ErasurePermit)>>,
}

impl Erasure {
    fn queue(&self) -> std::sync::MutexGuard<'_, HoldQueue<ErasurePermit>> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn staged(&self) -> std::sync::MutexGuard<'_, Vec<(String, ErasurePermit)>> {
        self.staged.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn take_permit(&self, subject: &str) -> Option<ErasurePermit> {
        let mut staged = self.staged();
        let index = staged.iter().position(|(staged, _)| staged == subject)?;
        Some(staged.swap_remove(index).1)
    }

    fn restage(&self, subject: String, permit: ErasurePermit) {
        self.staged().push((subject, permit));
    }
}

impl AdminPlane {
    /// Attach the erasure hold (Go `NewShredQueue(5 * time.Minute)` with the
    /// server vault). `hold` must be non-zero; Go's is
    /// `erasure_hold::DEFAULT_HOLD`.
    pub fn with_erasure(mut self, vault: Arc<Vault>, hold: Duration) -> Result<Self, Refusal> {
        self.erasure = Some(Erasure {
            vault,
            queue: Mutex::new(HoldQueue::new(hold)?),
            staged: Mutex::new(Vec::new()),
        });
        Ok(self)
    }

    /// Stage a ledger-issued erasure permit for one resolved subject. It is
    /// consumed only by the commit of an approved pending erasure of that
    /// exact subject, and only when it binds that subject in this plane's
    /// scope.
    pub fn retain_erasure_permit(
        &self,
        subject: &str,
        permit: ErasurePermit,
    ) -> Result<(), PermitRefusal> {
        let Some(erasure) = &self.erasure else {
            return Err(PermitRefusal::Unattached);
        };
        if !permit.binds(subject, &self.scope) {
            return Err(PermitRefusal::Binding);
        }
        let mut staged = erasure.staged();
        if staged.len() >= MAX_STAGED_PERMITS {
            return Err(PermitRefusal::Capacity);
        }
        staged.push((subject.to_owned(), permit));
        Ok(())
    }

    /// Execute every erasure whose hold window elapsed at `now` (the plane's
    /// monotonic clock) and record its completion evidence (Go
    /// `RunDueShreds`). `checkpoint` is the latest signed head (`root`,
    /// `tree_size`) of the chain the records go to, or `("", 0)` when there
    /// is none. Returns how many erasures executed; 0 without an attached
    /// erasure hold.
    pub fn run_due_shreds(&self, now: Duration, checkpoint: (&str, u64)) -> io::Result<usize> {
        let Some(erasure) = &self.erasure else {
            return Ok(0);
        };
        let mut queue = erasure.queue();
        let mut executed = 0;
        for id in queue.due(now) {
            let completion = queue
                .run_one(&id, checkpoint, |subject, permit| {
                    erasure.vault.shred(subject, permit)
                })
                .map_err(|_| io::Error::other("vault refused the shred"))?;
            let Some(completion) = completion else {
                continue;
            };
            executed += 1;
            self.emit(self.record(
                completion.event_type,
                self.actor(&completion.scheduled.approver),
                "",
                completion.certificate,
            ))?;
        }
        Ok(executed)
    }

    /// The subject an erasure request names (Go `adminErasure`): the reverse
    /// index entry for the handle, else the value itself.
    pub(super) fn erasure_subject(&self, requested: String) -> Result<String, Response> {
        let Some(erasure) = &self.erasure else {
            return Ok(requested);
        };
        match erasure.vault.resolve(&requested) {
            Ok(resolved) => Ok(resolved.unwrap_or(requested)),
            Err(_) => Err(Response::text(503, "vault unavailable")),
        }
    }

    /// The `erasure` case of Go `adminApprove`, after the approval record.
    pub(super) fn commit_erasure(
        &self,
        params: &BTreeMap<String, String>,
        requester: &str,
        approver: &Operator,
        unix: u64,
        now: Duration,
    ) -> Response {
        let Some(erasure) = &self.erasure else {
            return Response::text(
                409,
                "erasure commit needs the vault erasure permit and hold queue; not served here",
            );
        };
        let field = |name: &str| params.get(name).map_or("", String::as_str);
        let (subject, basis) = (field("subject"), field("legalBasis"));
        if subject.is_empty() || basis.is_empty() {
            return Response::text(500, "invalid pending erasure");
        }
        let mut queue = erasure.queue();
        if queue.len() >= MAX_PENDING {
            return Response::text(503, "erasure hold queue is full");
        }
        let Some(permit) = erasure.take_permit(subject) else {
            return Response::text(
                409,
                "erasure commit needs a ledger-issued erasure permit for this subject",
            );
        };
        let scheduled = match queue.schedule(subject, requester, &approver.id, basis, permit, now) {
            Ok(scheduled) => scheduled,
            Err(_) => return Response::text(503, "erasure hold queue refused the erasure"),
        };
        let recorded = self.emit(self.record(
            "approval.granted",
            self.actor(&approver.id),
            "",
            format!(
                "erasure scheduled (cancellable hold); id={} legal_basis={basis}",
                scheduled.id
            ),
        ));
        if recorded.is_err() {
            if let Some((scheduled, permit)) = queue.withdraw(&scheduled.id) {
                erasure.restage(scheduled.subject, permit);
            }
            return audit_unavailable();
        }
        Response::json(&json!({
            "ok": true,
            "committed": "erasure",
            "scheduled": true,
            "shredId": scheduled.id,
            "executeAtUnix": unix.saturating_add(queue.hold().as_secs()),
        }))
    }

    /// Go `adminListErasures`.
    pub(super) fn list_erasures(&self, now: Duration) -> Response {
        let Some(erasure) = &self.erasure else {
            return Response::not_found();
        };
        let scheduled: Vec<Value> = erasure
            .queue()
            .list()
            .into_iter()
            .map(|entry| {
                json!({
                    "id": entry.id,
                    "legalBasis": entry.legal_basis,
                    "requester": entry.requester,
                    "approver": entry.approver,
                    "executesInSec": seconds_until(entry.execute_at, now),
                })
            })
            .collect();
        Response::json(&json!({"scheduled": scheduled, "count": scheduled.len()}))
    }

    /// Go `adminCancelErasure`.
    pub(super) fn cancel_erasure(&self, id: &str, operator: &Operator) -> Response {
        let Some(erasure) = &self.erasure else {
            return Response::not_found();
        };
        let mut queue = erasure.queue();
        if !queue.contains(id) {
            return Response::text(404, "no such scheduled erasure");
        }
        if self
            .emit(self.record(
                "erasure.cancelled",
                self.actor(&operator.id),
                "",
                format!("erasure cancelled in hold window; id={id}"),
            ))
            .is_err()
        {
            return audit_unavailable();
        }
        match queue.cancel(id) {
            Some(cancelled) => Response::json(&json!({"ok": true, "cancelled": cancelled.id})),
            None => Response::text(404, "no such scheduled erasure"),
        }
    }
}

/// Go `int(executeAt.Sub(now).Seconds())`: signed, truncated toward zero.
fn seconds_until(at: Duration, now: Duration) -> i64 {
    let delta = i128::try_from(at.as_nanos()).unwrap_or(i128::MAX)
        - i128::try_from(now.as_nanos()).unwrap_or(i128::MAX);
    i64::try_from(delta / 1_000_000_000).unwrap_or(if delta < 0 { i64::MIN } else { i64::MAX })
}
