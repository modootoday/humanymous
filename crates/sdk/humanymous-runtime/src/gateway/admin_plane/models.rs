//! Behavioral-model promotion and rollback as audited operator actions of the
//! admin listener (owner decision #12, issue #75 comment 7017558784), over the
//! lifecycle of legacy internal/mlcorrect/bundle.go (`Stage`, `Promote`,
//! `Rollback`, `Active`, `Staged`) and the admission half of
//! cmd/server/ml_setup.go at baseline 981d25c.
//!
//! [`AdminPlane::with_models`] attaches a [`Models`]: a bundle lifecycle that
//! owns its verification policy (unsigned lab policy or a configured
//! [`BundleVerifier`], legacy `NewBundleManager`) and the shared inference
//! [`Runtime`] the scoring engine reads (legacy `mlserve` seam). Without it
//! every `models` route answers the plain 404 after the role check.
//!
//! Kept from Go (pinned against
//! `tests/fixtures/gateway/ml-bundle-go-golden.json`, a Go 1.26.3 capture of
//! `BundleManager` and `mlserve.Score` at 981d25c):
//! - Verification of a candidate, in Go order: digest pin, signature when a
//!   key is configured, shape-valid decode, schema pin. A refused candidate
//!   leaves the previously staged one in place. Staging never serves.
//! - Promote installs the staged model as the served predictor and keeps the
//!   prior one for rollback; rollback reverts to the prior model or, with
//!   none retained, to abstention; promote without a staged candidate and
//!   rollback with nothing active or retained are refused. After every
//!   transition the served version and the served prediction for a fixed
//!   feature vector equal Go's bit for bit.
//!
//! New in Rust by owner decision #12 (Go had no runtime model-mutation
//! endpoint: promotion was a redeploy with a new signed `-ml-bundle`, and
//! startup promoted the admitted bundle at once):
//! - Training and startup admission never activate a model: [`Models::stage`]
//!   only verifies and holds a candidate. Activation happens only through
//!   `POST models/promote`, which becomes a pending action a distinct
//!   approver commits (`approvals/<id>`, kind [`KIND_PROMOTE`]), the
//!   gate-admin dual control legacy ml_setup.go named for any runtime
//!   promotion. `POST models/rollback` is an operator action applied at once:
//!   it only moves toward the previous model or abstention.
//! - Both bodies name the digest they act on (`Digest`): promote the staged
//!   candidate's, rollback the active one's (empty when none is active). A
//!   mismatch, at request or at commit, is a 409 and changes nothing.
//! - Every promote and rollback writes a `config.changed` record before the
//!   change; if it cannot be written nothing changes and the request answers
//!   503 (the settings write plane pattern). Transitions are serialized.
//! - Activation never changes scoring weights, thresholds, hard-rule
//!   predicates, verdict bands or signal IDs: the served model only feeds the
//!   existing weight-0 `l4.ml.behavioral` annotation, and no threshold
//!   provider, feature observer or shadow model is installed here (the lab
//!   controller and canary stay in `gateway::model_admission::lab_control`).
use super::{AdminPlane, Clock, Response, audit_unavailable, object, string};
use crate::gateway::admin_approval::{Operator, Role};
use humanymous_core::model::correction::{
    BundleIdentity, BundleManager, BundleVerifier, CorrectionError,
};
use humanymous_core::model::inference::{Mlp, Runtime};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Pending action kind of a model promotion.
pub const KIND_PROMOTE: &str = "ml.model.promote";
/// Promote and rollback body cap.
pub const BODY: usize = 1 << 12;
/// Audit event of a promotion or rollback (the frozen legacy taxonomy has no
/// model event; `config.changed` is the configuration-change record).
pub const EVENT: &str = "config.changed";

/// One verified bundle: its identity and the model decoded from the same
/// bytes that were verified.
#[derive(Clone)]
struct Verified {
    identity: BundleIdentity,
    model: Arc<Mlp>,
}

#[derive(Default)]
struct Slots {
    staged: Option<Verified>,
    active: Option<Verified>,
    previous: Option<Verified>,
}

/// The served behavioral-model lifecycle (legacy `BundleManager` plus the
/// `mlserve` seam).
pub struct Models {
    bundles: BundleManager,
    runtime: Runtime,
    slots: Mutex<Slots>,
}

impl Models {
    /// Unsigned lab policy: schema pin and optional digest pin only (legacy
    /// `NewBundleManager(nil)`). `runtime` is the inference runtime the
    /// scoring engine reads; it is changed only by promote and rollback.
    pub fn unsigned(runtime: Runtime) -> Self {
        Self::with(BundleManager::unsigned_lab(), runtime)
    }

    /// Signed policy: every stage needs a signature `verifier` accepts over
    /// the exact bundle bytes (legacy `NewBundleManager(pubkeyPEM)`; the
    /// runtime's verifier is `gateway::model_admission::Ed25519Verifier`).
    pub fn signed(verifier: Box<dyn BundleVerifier>, runtime: Runtime) -> Self {
        Self::with(BundleManager::with_verifier(verifier), runtime)
    }

    fn with(bundles: BundleManager, runtime: Runtime) -> Self {
        Self {
            bundles,
            runtime,
            slots: Mutex::default(),
        }
    }

    /// Whether a signature is required to stage.
    pub fn is_signed(&self) -> bool {
        self.bundles.signed()
    }

    /// Verify a candidate and hold it as staged without serving it (legacy
    /// `Stage` over the exact bytes). A refusal keeps the previous candidate.
    pub fn stage(
        &self,
        bundle_bytes: &[u8],
        expected_digest: Option<&str>,
        signature: Option<&[u8]>,
    ) -> Result<BundleIdentity, CorrectionError> {
        let mut slots = self.lock();
        let identity = self
            .bundles
            .stage_signed(bundle_bytes, expected_digest, signature)?;
        // The same bytes passed the shape-valid decode inside the stage.
        let model = Mlp::from_json(bundle_bytes).map_err(|_| CorrectionError::InvalidBundle)?;
        slots.staged = Some(Verified {
            identity: identity.clone(),
            model: Arc::new(model),
        });
        Ok(identity)
    }

    /// Verified candidate awaiting promotion.
    pub fn staged(&self) -> Option<BundleIdentity> {
        self.lock()
            .staged
            .as_ref()
            .map(|slot| slot.identity.clone())
    }

    /// Served identity; `None` is legacy `"none"` (abstaining).
    pub fn active(&self) -> Option<BundleIdentity> {
        self.lock()
            .active
            .as_ref()
            .map(|slot| slot.identity.clone())
    }

    /// Identity retained for rollback.
    pub fn previous(&self) -> Option<BundleIdentity> {
        self.lock()
            .previous
            .as_ref()
            .map(|slot| slot.identity.clone())
    }

    fn lock(&self) -> MutexGuard<'_, Slots> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Legacy `Promote` under the held lock.
    fn promote(&self, slots: &mut Slots) -> Result<BundleIdentity, CorrectionError> {
        let staged = slots.staged.take().ok_or(CorrectionError::NoStaged)?;
        if let Err(error) = self.bundles.promote() {
            slots.staged = Some(staged);
            return Err(error);
        }
        self.runtime.set(Some(staged.model.clone()));
        slots.previous = slots.active.replace(staged.clone());
        Ok(staged.identity)
    }

    /// Legacy `Rollback` under the held lock; returns the identity now served.
    fn rollback(&self, slots: &mut Slots) -> Result<Option<BundleIdentity>, CorrectionError> {
        if slots.active.is_none() && slots.previous.is_none() {
            return Err(CorrectionError::NoPrevious);
        }
        self.bundles.rollback()?;
        slots.active = slots.previous.take();
        match &slots.active {
            Some(active) => self.runtime.set(Some(active.model.clone())),
            None => self.runtime.set(None),
        }
        Ok(slots.active.as_ref().map(|slot| slot.identity.clone()))
    }
}

/// Go `Active()`: `("none", "")` while abstaining.
fn active_view(identity: Option<&BundleIdentity>) -> Value {
    identity.map_or_else(
        || json!({"version": "none", "digest": ""}),
        |identity| json!({"version": identity.version, "digest": identity.digest}),
    )
}

fn view(identity: Option<&BundleIdentity>) -> Value {
    identity.map_or(
        Value::Null,
        |identity| json!({"version": identity.version, "digest": identity.digest}),
    )
}

/// The `Digest` member of a capped body; `None` is a 400.
fn digest_field(body: &[u8]) -> Option<String> {
    object(body, BODY).and_then(|fields| string(&fields, "Digest").ok())
}

impl AdminPlane {
    /// Serve the `models` routes from `models`.
    pub fn with_models(mut self, models: Models) -> Self {
        self.models = Some(models);
        self
    }

    pub(super) fn models_view(&self) -> Response {
        let Some(models) = &self.models else {
            return Response::not_found();
        };
        let slots = models.lock();
        let identity = |slot: &Option<Verified>| slot.as_ref().map(|slot| slot.identity.clone());
        Response::json(&json!({
            "active": active_view(identity(&slots.active).as_ref()),
            "previous": view(identity(&slots.previous).as_ref()),
            "signed": models.is_signed(),
            "staged": view(identity(&slots.staged).as_ref()),
        }))
    }

    pub(super) fn models_promote(
        &self,
        body: &[u8],
        operator: &Operator,
        clock: Clock,
    ) -> Response {
        let Some(models) = &self.models else {
            return Response::not_found();
        };
        let Some(digest) = digest_field(body) else {
            return Response::text(400, "bad request");
        };
        if digest.is_empty() {
            return Response::text(400, "digest required");
        }
        let staged = {
            let slots = models.lock();
            match &slots.staged {
                None => return Response::text(409, "no verified candidate staged"),
                Some(staged) if staged.identity.digest != digest => {
                    return Response::text(409, "CAS: digest does not match the staged candidate");
                }
                Some(staged) => staged.identity.clone(),
            }
        };
        let params = BTreeMap::from([
            ("digest".to_owned(), staged.digest.clone()),
            ("version".to_owned(), staged.version.clone()),
        ]);
        self.request_approval(
            KIND_PROMOTE,
            &params,
            operator,
            Role::Approver,
            format!(
                "model promote pending approval: version={} digest={}",
                staged.version, staged.digest
            ),
            clock,
        )
    }

    /// Commit an approved promotion of the candidate whose digest the
    /// pending action binds.
    pub(super) fn commit_models_promote(
        &self,
        params: &BTreeMap<String, String>,
        requester: &str,
        approver: &Operator,
    ) -> Response {
        let Some(models) = &self.models else {
            return Response::not_found();
        };
        let digest = params.get("digest").map_or("", String::as_str);
        let mut slots = models.lock();
        let Some(staged) = slots.staged.as_ref().map(|slot| slot.identity.clone()) else {
            return Response::text(409, "no verified candidate staged");
        };
        if staged.digest != digest {
            return Response::text(409, "CAS: digest does not match the staged candidate");
        }
        let reason = format!(
            "ml.model.promoted version={} digest={} (dual-control: {requester}\u{2192}{})",
            staged.version, staged.digest, approver.id
        );
        if self
            .emit(self.record(EVENT, self.actor(requester), KIND_PROMOTE, reason))
            .is_err()
        {
            return audit_unavailable();
        }
        match models.promote(&mut slots) {
            Ok(active) => Response::json(&json!({
                "ok": true,
                "committed": KIND_PROMOTE,
                "active": active_view(Some(&active)),
            })),
            Err(_) => Response::text(409, "no verified candidate staged"),
        }
    }

    pub(super) fn models_rollback(&self, body: &[u8], operator: &Operator) -> Response {
        let Some(models) = &self.models else {
            return Response::not_found();
        };
        let Some(digest) = digest_field(body) else {
            return Response::text(400, "bad request");
        };
        let mut slots = models.lock();
        let active = slots.active.as_ref().map(|slot| slot.identity.clone());
        let current = active
            .as_ref()
            .map_or("", |identity| identity.digest.as_str());
        if digest != current {
            return Response::text(409, "CAS: digest does not match the active model");
        }
        if slots.active.is_none() && slots.previous.is_none() {
            return Response::text(409, "no previous bundle to roll back to");
        }
        let target = slots.previous.as_ref().map(|slot| slot.identity.clone());
        let reason = format!(
            "ml.model.rolled_back from version={} digest={} to {}",
            active
                .as_ref()
                .map_or("none", |identity| identity.version.as_str()),
            current,
            target.as_ref().map_or_else(
                || "none (abstain)".to_owned(),
                |identity| format!("version={} digest={}", identity.version, identity.digest)
            ),
        );
        if self
            .emit(self.record(EVENT, self.actor(&operator.id), "ml.model.rollback", reason))
            .is_err()
        {
            return audit_unavailable();
        }
        match models.rollback(&mut slots) {
            Ok(now) => Response::json(&json!({
                "ok": true,
                "rolledBack": true,
                "active": active_view(now.as_ref()),
            })),
            Err(_) => Response::text(409, "no previous bundle to roll back to"),
        }
    }
}

#[cfg(test)]
mod tests;
