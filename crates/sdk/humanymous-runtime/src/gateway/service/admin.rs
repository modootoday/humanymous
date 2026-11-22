//! The separate admin TCP listener (owner decision B01 #1; Go `cmd/gate`
//! `-admin-addr` serving `srv.AdminHandler()` beside the public edge at
//! baseline 981d25c). `gateway::admin_plane` is served on the configured
//! loopback address only; the public listener keeps refusing every
//! `/_humanymous/admin/` path as a reserved route it does not own.
//!
//! Composed here:
//! - the bearer table from `admin.operators`;
//! - a ban store built from `admin.bans`, shared by `Arc` with the edge ban
//!   gate through `Application::install_bans` under one monotonic epoch;
//! - the durable audit sink of the serving process (the same sealed log);
//! - with `admin.settings_dir`, the settings engine over that overlay store,
//!   swept on Go's one-minute GC cadence;
//! - the read-only `policy` view and the signed effective-policy version
//!   (owner decision #8): one boot slice, kill-switch flag and active overlay
//!   slot shared by the settings engine (which mirrors its active overlay into
//!   the slot), the `policy` view and the [`SignedPolicy`] the service
//!   installs with `Application::install_signed_policy`, so served records
//!   carry the `configVersion` the `policy` view reports;
//! - with `admin.models`, the behavioral-model lifecycle (owner decision #12)
//!   over the inference runtime the serving engine reads. The configured
//!   bundle is verified and staged at startup, never activated: only the
//!   dual-controlled `models/promote` serves it. As in Go's admission, a
//!   refused candidate (unreadable bundle or signature, digest, signature,
//!   shape or schema) is not fatal: nothing is staged and readiness says so.
//!   A configured public key that cannot be read or parsed refuses startup
//!   (Go fell back to heuristics only): a signed policy never degrades to an
//!   unsigned or absent one.
//!
//! The approval, operator-pseudonym and configuration-version key is random
//! per process, as Go's was without `HMN_TOKEN_KEY`: pending approvals and
//! pseudonyms do not survive a restart, and the same policy gets a new
//! `cfg-…` version after a restart. The settings boot posture has no route presets (the runtime
//! serves an exact route inventory, not Go presets), global monitor and kill
//! switch off; the stored overlay is served and validated, and only its rate
//! window reaches the shared ban store. The kill-switch flag stays off: an
//! approved Go kill-switch commit answers 409 (owner decision #6). Nothing
//! here feeds scoring.
use crate::authority::decision::DurableAudit;
use crate::gateway::admin_approval::Role;
use crate::gateway::admin_plane::models::Models;
use crate::gateway::admin_plane::{AdminPlane, Clock, policy::Policy, settings::Settings};
use crate::gateway::ban::Store;
use crate::gateway::configuration::ModelSource;
use crate::gateway::configuration::{AdminListener as AdminConfig, Configuration};
use crate::gateway::model_admission::{Ed25519Verifier, MAX_SMALL_FILE, read_bounded};
use crate::gateway::service::ShutdownRequest;
use crate::gateway::settings::configsign::{BootPolicy, SignedPolicy};
use crate::gateway::settings::overlay::{Overlay, OverlayFileStore};
use humanymous_core::model::correction::MAX_BUNDLE_BYTES;
use humanymous_core::model::inference::Runtime;
use std::collections::BTreeMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::watch;
use zeroize::Zeroizing;

/// Go GC ticker cadence for overlay expiry.
const SWEEP_INTERVAL: Duration = Duration::from_secs(60);

/// A bound admin listener and its plane.
pub(in crate::gateway) struct AdminService {
    plane: Arc<AdminPlane>,
    listener: TcpListener,
    epoch: Instant,
    bans: Arc<Mutex<Store>>,
    settings: bool,
    operators: usize,
    signed: SignedPolicy,
    models: Option<serde_json::Value>,
    /// The active-overlay slot the settings engine mirrors, when settings are
    /// configured.
    overlay: Option<Arc<RwLock<Option<Overlay>>>>,
}

fn clock(epoch: Instant) -> io::Result<Clock> {
    let unix = crate::authority::now_ms().map_err(|_| io::Error::other("clock unavailable"))?;
    Ok(Clock {
        monotonic: epoch.elapsed(),
        unix: unix / 1000,
    })
}

impl AdminService {
    /// Build the plane and bind its loopback listener. Every refusal happens
    /// before the service reports readiness.
    pub(in crate::gateway) async fn bind(
        admin: &AdminConfig,
        configuration: &Configuration,
        audit: Box<dyn DurableAudit>,
        runtime: &Runtime,
    ) -> io::Result<Self> {
        admin
            .validate()
            .map_err(|_| io::Error::other("admin configuration invalid"))?;
        let mut key = Zeroizing::new(vec![0u8; 32]);
        getrandom::fill(&mut key).map_err(|_| io::Error::other("admin key entropy unavailable"))?;
        let bans = Arc::new(Mutex::new(
            admin
                .bans
                .store()
                .map_err(|_| io::Error::other("admin ban store refused"))?,
        ));
        let scope = crate::audit::control::Scope {
            installation: configuration.scope.installation.clone(),
            node: configuration.audit.node_id.clone(),
        };
        let mut plane = AdminPlane::new(
            &configuration.audit.node_id,
            scope,
            &key,
            bans.clone(),
            audit,
        );
        for operator in &admin.operators {
            let role: Role = operator
                .role()
                .ok_or_else(|| io::Error::other("admin operator role"))?;
            plane
                .auth()
                .add(operator.token.as_str(), &operator.id, role)
                .map_err(|_| io::Error::other("admin operator refused"))?;
        }
        let epoch = Instant::now();
        let boot = BootPolicy {
            global_monitor: false,
            routes: BTreeMap::new(),
            rate_window_nanos: admin
                .bans
                .window_sec
                .checked_mul(1_000_000_000)
                .and_then(|nanos| i64::try_from(nanos).ok())
                .ok_or_else(|| io::Error::other("admin rate window"))?,
            rate_soft: i64::try_from(admin.bans.soft)
                .map_err(|_| io::Error::other("admin rate cutpoint"))?,
            rate_hard: i64::try_from(admin.bans.hard)
                .map_err(|_| io::Error::other("admin rate cutpoint"))?,
            hmac_key: key.to_vec(),
        };
        let kill_switch = Arc::new(AtomicBool::new(false));
        let active: Arc<RwLock<Option<Overlay>>> = Arc::new(RwLock::new(None));
        if let Some(directory) = &admin.settings_dir {
            let store = OverlayFileStore::open(directory)
                .map_err(|_| io::Error::other("admin settings store unavailable"))?;
            // The settings engine hashes the same boot posture the signed
            // version does. A zero bound, which the legacy defaults would
            // rewrite on one side only, is refused.
            if boot.rate_window_nanos <= 0 || boot.rate_soft <= 0 || boot.rate_hard <= 0 {
                return Err(io::Error::other("admin rate bounds"));
            }
            let settings = Settings::new(boot.boot_input(false), kill_switch.clone(), store)
                .with_active_slot(active.clone());
            plane = plane
                .with_settings(settings, clock(epoch)?)
                .map_err(|_| io::Error::other("admin settings refused"))?;
        }
        plane = plane.with_policy(Policy::new(
            boot.clone(),
            kill_switch.clone(),
            active.clone(),
        ));
        let overlay_slot = active.clone();
        let signed = SignedPolicy::new(boot, kill_switch, active);
        let mut models = None;
        if let Some(source) = &admin.models {
            let (lifecycle, facts) = stage(source, runtime)?;
            plane = plane.with_models(lifecycle);
            models = Some(facts);
        }
        let listener = TcpListener::bind(admin.listen).await?;
        Ok(Self {
            plane: Arc::new(plane),
            listener,
            epoch,
            bans,
            settings: admin.settings_dir.is_some(),
            operators: admin.operators.len(),
            signed,
            models,
            overlay: admin.settings_dir.as_ref().map(|_| overlay_slot),
        })
    }

    /// The bound loopback address, including an assigned ephemeral port.
    pub(in crate::gateway) fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// The store and epoch the edge ban gate must share.
    pub(in crate::gateway) fn bans(&self) -> (Arc<Mutex<Store>>, Instant) {
        (self.bans.clone(), self.epoch)
    }

    /// The signed effective-policy version source the served records bind
    /// (owner decision #8); it shares the `policy` view's inputs.
    pub(in crate::gateway) fn signed_policy(&self) -> SignedPolicy {
        self.signed.clone()
    }

    /// The active-overlay slot whose gate modes the request path reads (owner
    /// decision #3), when a settings engine mirrors into it.
    pub(in crate::gateway) fn gate_overlay(&self) -> Option<Arc<RwLock<Option<Overlay>>>> {
        self.overlay.clone()
    }

    /// Readiness facts; no tokens, ids or keys.
    pub(in crate::gateway) fn readiness(&self) -> io::Result<serde_json::Value> {
        let mut facts = serde_json::json!({
            "listen": self.local_addr()?,
            "operators": self.operators,
            "settings": self.settings,
            "transport": "loopback-http1",
        });
        if let Some(models) = &self.models {
            facts["models"] = models.clone();
        }
        Ok(facts)
    }

    /// Serve until shutdown, sweeping an expired overlay every minute. A sweep
    /// whose audit record fails stops the admin listener (fail closed).
    pub(in crate::gateway) async fn run(
        self,
        shutdown: watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<()> {
        tokio::select! {
            served = self.plane.clone().serve_from(self.listener, shutdown, self.epoch) => served,
            swept = sweep(&self.plane, self.epoch) => swept,
        }
    }
}

/// Build the model lifecycle over the serving `runtime` and stage the
/// configured candidate. Returns the readiness facts: whether a signature is
/// required and whether a candidate is staged (never which bytes or paths).
fn stage(source: &ModelSource, runtime: &Runtime) -> io::Result<(Models, serde_json::Value)> {
    let models = match &source.public_key {
        None => Models::unsigned(runtime.clone()),
        Some(path) => {
            let pem = read_bounded(path, MAX_SMALL_FILE)
                .filter(|pem| !pem.is_empty())
                .ok_or_else(|| io::Error::other("admin model public key unreadable"))?;
            let verifier = Ed25519Verifier::from_pkix_pem(&pem)
                .map_err(|_| io::Error::other("admin model public key refused"))?;
            Models::signed(Box::new(verifier), runtime.clone())
        }
    };
    let signature = match &source.signature {
        None => Ok(None),
        Some(path) => read_bounded(path, MAX_SMALL_FILE)
            .map(Some)
            .ok_or("signature_read"),
    };
    let bundle = read_bounded(&source.bundle, MAX_BUNDLE_BYTES)
        .filter(|bytes| !bytes.is_empty())
        .ok_or("bundle_read");
    let digest = (!source.digest.is_empty()).then_some(source.digest.as_str());
    let outcome = signature.and_then(|signature| {
        let bundle = bundle?;
        models
            .stage(&bundle, digest, signature.as_deref())
            .map(|_| ())
            .map_err(|_| "verification")
    });
    let facts = serde_json::json!({
        "signed": models.is_signed(),
        "staged": outcome.is_ok(),
        "refusal": outcome.err(),
    });
    Ok((models, facts))
}

/// Go `GC` calling `expireActiveOverlayIfNeeded` once a minute; returns only
/// on a failure.
async fn sweep(plane: &AdminPlane, epoch: Instant) -> io::Result<()> {
    let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    ticker.tick().await;
    loop {
        ticker.tick().await;
        plane.sweep_settings(clock(epoch)?)?;
    }
}
