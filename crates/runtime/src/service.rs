//! OS-independent lifecycle. The caller owns platform signals and private input.
use crate::{
    application::Application,
    configuration::Configuration,
    management::{Management, ManagementPolicy},
    settings::{SettingsPolicy, SettingsStore},
    startup::{ExternalWitness, SealedAudit},
};
use humanymous_authority::decision::DurableAudit;
use serde::Serialize;
use std::{
    io::{self, Read},
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener, sync::watch};
use zeroize::Zeroizing;
mod lifecycle;
mod outcome;
mod readiness;
mod shutdown;
pub use outcome::{ExitFailure, ExitReport, ExitStage};
pub use readiness::standalone_profile;
pub use shutdown::{ShutdownRequest, StopBudget};

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

pub(crate) struct SharedAudit(pub(crate) Arc<Mutex<SealedAudit>>);
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

/// Holds sealed custody through listener work and the final synchronous flush.
/// Blocking custody/WAL I/O requires an external process supervisor deadline.
pub struct ServiceHost {
    application: Application,
    listener: TcpListener,
    audit: Arc<Mutex<SealedAudit>>,
    readiness: Readiness,
    management: Option<Management>,
    settings: Option<SettingsStore>,
}
impl ServiceHost {
    /// Authenticate existing credentials before exposing a ready listener.
    pub async fn bind(configuration: Configuration) -> io::Result<Self> {
        Self::bind_inner(configuration, None, None, None, None).await
    }

    /// Consume a pinned external client. No witness failure permits fallback.
    pub async fn bind_with_witness(
        configuration: Configuration,
        witness: ExternalWitness,
    ) -> io::Result<Self> {
        Self::bind_inner(configuration, Some(witness), None, None, None).await
    }

    /// Bind a separately authorized native management service in the same custody
    /// lifetime. Provision its new credential privately before starting run.
    pub async fn bind_with_management(
        configuration: Configuration,
        witness: Option<ExternalWitness>,
        policy: ManagementPolicy,
    ) -> io::Result<Self> {
        Self::bind_inner(configuration, witness, Some(policy), None, None).await
    }

    /// Explicit operator recovery with an independently pinned head and witness.
    /// It never retries a prior request or implies its origin effect completed.
    pub async fn bind_recovered(
        configuration: Configuration,
        witness: ExternalWitness,
        tail: humanymous_audit::storage::TailRecovery,
        management: Option<ManagementPolicy>,
    ) -> io::Result<Self> {
        Self::bind_inner(configuration, Some(witness), management, Some(tail), None).await
    }

    /// Restore or explicitly provision witnessed private operational state before
    /// listener readiness. Management writes remain separately authorized.
    pub async fn bind_with_settings(
        configuration: Configuration,
        witness: ExternalWitness,
        management: Option<ManagementPolicy>,
        settings: SettingsPolicy,
    ) -> io::Result<Self> {
        Self::bind_inner(
            configuration,
            Some(witness),
            management,
            None,
            Some(settings),
        )
        .await
    }

    /// Combine explicit witnessed WAL recovery with authenticated settings
    /// recovery. Unwitnessed operational commits remain refused.
    pub async fn bind_recovered_with_settings(
        configuration: Configuration,
        witness: ExternalWitness,
        tail: humanymous_audit::storage::TailRecovery,
        management: Option<ManagementPolicy>,
        settings: SettingsPolicy,
    ) -> io::Result<Self> {
        Self::bind_inner(
            configuration,
            Some(witness),
            management,
            Some(tail),
            Some(settings),
        )
        .await
    }

    async fn bind_inner(
        mut configuration: Configuration,
        witness: Option<ExternalWitness>,
        management_policy: Option<ManagementPolicy>,
        recovery: Option<humanymous_audit::storage::TailRecovery>,
        settings_policy: Option<SettingsPolicy>,
    ) -> io::Result<Self> {
        configuration
            .validate()
            .map_err(|_| io::Error::other("configuration invalid"))?;
        let witness_durability = witness.as_ref().map(ExternalWitness::durability_name);
        let mut sealed = match (witness, recovery) {
            (Some(witness), Some(tail)) => {
                crate::startup::recover_audit_with_witness(&mut configuration, witness, tail)
            }
            (Some(witness), None) => {
                crate::startup::open_audit_with_witness(&mut configuration, witness)
            }
            (None, None) => crate::startup::open_audit(&mut configuration),
            (None, Some(_)) => return Err(io::Error::other("recovery requires external witness")),
        }
        .map_err(io::Error::other)?;
        let settings = if let Some(policy) = settings_policy {
            let owner =
                crate::startup::native_owner(&configuration.owner).map_err(io::Error::other)?;
            let store = SettingsStore::open(policy, owner, &configuration, &mut sealed)?;
            store
                .restore_configuration(&mut configuration)
                .map_err(|_| io::Error::other("operational restore failed"))?;
            Some(store)
        } else {
            None
        };
        let recovery=sealed.recovery_coverage().map(|coverage|serde_json::json!({
            "durable_sequence":coverage.durable_sequence,"witnessed_through":coverage.witnessed_through,
            "ambiguous_tail_len":coverage.ambiguous_tail_len,"fully_witnessed":coverage.fully_witnessed,
            "authentication":coverage.authentication,
        }));
        let audit = Arc::new(Mutex::new(sealed));
        let listener = TcpListener::bind(configuration.listen).await?;
        let profile = readiness::profile(
            &configuration,
            witness_durability,
            recovery,
            management_policy.is_some(),
            settings.as_ref().map(|store| store.snapshot().revision()),
        );
        let management = management_policy
            .map(|policy| Management::bind(policy, &configuration, profile.clone()))
            .transpose()?;
        let application = Application::new(
            configuration,
            Default::default(),
            Box::new(SharedAudit(audit.clone())),
        )
        .map_err(|_| io::Error::other("application startup"))?;
        let readiness = Readiness {
            listen: listener.local_addr()?,
            profile,
        };
        Ok(Self {
            application,
            listener,
            audit,
            readiness,
            management,
            settings,
        })
    }
    /// No raw credentials or audit material are included.
    pub fn readiness(&self) -> &Readiness {
        &self.readiness
    }
    /// Authenticated persisted operational revision, absent when not configured.
    pub fn operational_revision(&self) -> Option<u64> {
        self.settings
            .as_ref()
            .map(|store| store.snapshot().revision())
    }

    /// Separate management capability for the trusted broker's private channel.
    /// It is deliberately absent from the serializable readiness document.
    pub fn management_credential_for_private_provisioning(&self) -> Option<&str> {
        self.management
            .as_ref()
            .map(Management::credential_for_private_provisioning)
    }

    /// A closed controller or incomplete drain is an error, never Stopped.
    pub async fn run(
        self,
        shutdown: watch::Receiver<Option<ShutdownRequest>>,
    ) -> io::Result<ExitReport> {
        self.run_report(shutdown).await.map_err(io::Error::other)
    }

    /// Preserve both shutdown phases. A flush is attempted after failed drain,
    /// but neither failure can be reported as a successful Stopped transition.
    pub async fn run_report(
        mut self,
        shutdown: watch::Receiver<Option<ShutdownRequest>>,
    ) -> Result<ExitReport, ExitFailure> {
        let drained = if let Some(management) = self.management {
            lifecycle::run(&mut self.application, self.listener, management, shutdown).await
        } else {
            self.application.serve_until(self.listener, shutdown).await
        };
        let flushed = self
            .audit
            .lock()
            .map_err(|_| io::Error::other("audit poisoned"))
            .and_then(|mut audit| audit.flush());
        outcome::finish(drained, flushed)
    }
}
