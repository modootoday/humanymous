//! OS-independent lifecycle. The caller owns platform signals and private input.
use crate::{
    application::Application,
    configuration::Configuration,
    startup::{ExternalWitness, SealedAudit},
};
use humanymous_authority::decision::DurableAudit;
use serde::Serialize;
use std::{
    io::{self, Read},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{net::TcpListener, sync::watch};
use zeroize::Zeroizing;

/// Validated bounded drain request; no OS handle or credential crosses this API.
#[derive(Clone, Copy)]
pub struct ShutdownRequest {
    drain_ms: u64,
}
impl ShutdownRequest {
    /// Bound the asynchronous drain to at most thirty seconds.
    pub fn new(drain_ms: u64) -> io::Result<Self> {
        if !(1..=30_000).contains(&drain_ms) {
            return Err(io::Error::other("shutdown drain budget"));
        }
        Ok(Self { drain_ms })
    }
    pub(crate) fn drain_timeout(self) -> Duration {
        Duration::from_millis(self.drain_ms)
    }
}

/// Read one private document. Validating the opened input's provenance is the
/// caller's responsibility; this helper does not certify an arbitrary file.
pub fn read_configuration(input: impl Read) -> io::Result<Configuration> {
    let mut bytes = Zeroizing::new(Vec::new());
    input.take(65_537).read_to_end(&mut bytes)?;
    if bytes.len() > 65_536 {
        return Err(io::Error::other("configuration budget"));
    }
    let configuration: Configuration =
        serde_json::from_slice(&bytes).map_err(|_| io::Error::other("configuration JSON"))?;
    configuration
        .validate()
        .map_err(|_| io::Error::other("configuration invalid"))?;
    Ok(configuration)
}

struct SharedAudit(Arc<Mutex<SealedAudit>>);
impl DurableAudit for SharedAudit {
    fn append(&mut self, record: humanymous_audit::record::Record) -> io::Result<()> {
        self.0
            .lock()
            .map_err(|_| io::Error::other("audit poisoned"))?
            .append(record)
    }
}

/// Readiness is available only after custody recovery and listener bind.
#[derive(Serialize)]
pub struct Readiness {
    /// Actual bound loopback endpoint, including an assigned ephemeral port.
    pub listen: SocketAddr,
    /// Existing capability manifest; absent producers remain unavailable.
    pub profile: serde_json::Value,
}

/// Complete shutdown evidence, returned only after request work and audit flush.
pub struct ExitReport {
    /// True means all owned asynchronous request work ended before flush.
    pub stopped: bool,
}

/// Holds sealed custody through listener work and the final synchronous flush.
/// Blocking custody/WAL I/O requires an external process supervisor deadline.
pub struct ServiceHost {
    application: Application,
    listener: TcpListener,
    audit: Arc<Mutex<SealedAudit>>,
    readiness: Readiness,
}
impl ServiceHost {
    /// Authenticate existing credentials before exposing a ready listener.
    pub async fn bind(configuration: Configuration) -> io::Result<Self> {
        Self::bind_inner(configuration, None).await
    }

    /// Consume a pinned external client. No witness failure permits fallback.
    pub async fn bind_with_witness(
        configuration: Configuration,
        witness: ExternalWitness,
    ) -> io::Result<Self> {
        Self::bind_inner(configuration, Some(witness)).await
    }

    async fn bind_inner(
        mut configuration: Configuration,
        witness: Option<ExternalWitness>,
    ) -> io::Result<Self> {
        configuration
            .validate()
            .map_err(|_| io::Error::other("configuration invalid"))?;
        let witness_durability = witness.as_ref().map(ExternalWitness::durability_name);
        let sealed = match witness {
            Some(witness) => crate::startup::open_audit_with_witness(&mut configuration, witness),
            None => crate::startup::open_audit(&mut configuration),
        }
        .map_err(io::Error::other)?;
        let audit = Arc::new(Mutex::new(sealed));
        let listener = TcpListener::bind(configuration.listen).await?;
        let application = Application::new(
            configuration,
            Default::default(),
            Box::new(SharedAudit(audit.clone())),
        )
        .map_err(|_| io::Error::other("application startup"))?;
        let mut profile = application.readiness();
        profile["audit_external_witness"] = serde_json::json!(witness_durability.is_some());
        profile["audit_witness_required_durability"] = serde_json::json!(witness_durability);
        // A connected/pinned client alone cannot certify deployment independence.
        profile["audit_witness_independence_qualified"] = serde_json::json!(false);
        let readiness = Readiness {
            listen: listener.local_addr()?,
            profile,
        };
        Ok(Self {
            application,
            listener,
            audit,
            readiness,
        })
    }
    /// No raw credentials or audit material are included.
    pub fn readiness(&self) -> &Readiness {
        &self.readiness
    }

    /// A closed controller or incomplete drain is an error, never Stopped.
    pub async fn run(
        mut self,
        shutdown: watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<ExitReport> {
        self.application
            .serve_until(self.listener, shutdown)
            .await?;
        self.audit
            .lock()
            .map_err(|_| io::Error::other("audit poisoned"))?
            .flush()?;
        Ok(ExitReport { stopped: true })
    }
}
