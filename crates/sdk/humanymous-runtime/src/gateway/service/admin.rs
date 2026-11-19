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
//!   swept on Go's one-minute GC cadence, whose active overlay is mirrored
//!   into the slot the served path reads per-gate modes from (owner decision
//!   #3, `application::modes`).
//!
//! The approval and operator-pseudonym key is random per process, as Go's was
//! without `HMN_TOKEN_KEY`: pending approvals and pseudonyms do not survive a
//! restart. The settings boot posture has no route presets (the runtime
//! serves an exact route inventory, not Go presets), global monitor and kill
//! switch off; the stored overlay is served and validated, its rate window
//! reaches the shared ban store and its gate modes reach the served path.
//! Nothing here feeds scoring.
use crate::authority::decision::DurableAudit;
use crate::gateway::admin_approval::Role;
use crate::gateway::admin_plane::{AdminPlane, Clock, settings::Settings};
use crate::gateway::application::ActiveOverlay;
use crate::gateway::ban::Store;
use crate::gateway::configuration::{AdminListener as AdminConfig, Configuration};
use crate::gateway::service::ShutdownRequest;
use crate::gateway::settings::overlay::{BootInput, OverlayFileStore};
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
    settings: Option<ActiveOverlay>,
    operators: usize,
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
        let mut active = None;
        if let Some(directory) = &admin.settings_dir {
            let store = OverlayFileStore::open(directory)
                .map_err(|_| io::Error::other("admin settings store unavailable"))?;
            let boot = BootInput {
                global_monitor: false,
                kill_switch: false,
                routes: BTreeMap::new(),
                rate_window_sec: i64::try_from(admin.bans.window_sec)
                    .map_err(|_| io::Error::other("admin rate window"))?,
                rate_soft: i64::try_from(admin.bans.soft)
                    .map_err(|_| io::Error::other("admin rate cutpoint"))?,
                rate_hard: i64::try_from(admin.bans.hard)
                    .map_err(|_| io::Error::other("admin rate cutpoint"))?,
                hmac_key: key.to_vec(),
            };
            let slot: ActiveOverlay = Arc::new(RwLock::new(None));
            let settings = Settings::new(boot, Arc::new(AtomicBool::new(false)), store)
                .with_active_slot(slot.clone());
            active = Some(slot);
            plane = plane
                .with_settings(settings, clock(epoch)?)
                .map_err(|_| io::Error::other("admin settings refused"))?;
        }
        let listener = TcpListener::bind(admin.listen).await?;
        Ok(Self {
            plane: Arc::new(plane),
            listener,
            epoch,
            bans,
            settings: active,
            operators: admin.operators.len(),
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

    /// The active-overlay slot the settings write plane publishes, when a
    /// settings directory is configured.
    pub(in crate::gateway) fn active_overlay(&self) -> Option<ActiveOverlay> {
        self.settings.clone()
    }

    /// Readiness facts; no tokens, ids or keys.
    pub(in crate::gateway) fn readiness(&self) -> io::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "listen": self.local_addr()?,
            "operators": self.operators,
            "settings": self.settings.is_some(),
            "transport": "loopback-http1",
        }))
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
