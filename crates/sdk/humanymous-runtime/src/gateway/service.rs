//! OS-independent lifecycle. The caller owns platform signals and private input.
use crate::authority::decision::DurableAudit;
use crate::gateway::{
    application::Application,
    configuration::Configuration,
    management::{Management, ManagementPolicy},
    settings::{SettingsPolicy, SettingsStore},
    startup::{ExternalWitness, SealedAudit},
};
use serde::Serialize;
use std::{
    io::{self, Read},
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener, sync::watch};
use zeroize::Zeroizing;
mod authorization;
mod lifecycle;
mod observed;
mod outcome;
mod readiness;
mod shutdown;
// Private S1 model only: no production activation or observer proof exists yet.
#[cfg(test)]
mod epoch;
pub use authorization::{AuthorizationData, AuthorizationDataError};
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

pub(in crate::gateway) struct SharedAudit(pub(in crate::gateway) Arc<Mutex<SealedAudit>>);
impl DurableAudit for SharedAudit {
    fn append(&mut self, record: crate::audit::record::Record) -> io::Result<()> {
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
    tls: Option<Arc<rustls::ServerConfig>>,
}
impl ServiceHost {
    /// Authenticate existing credentials before exposing a ready listener.
    pub async fn bind(configuration: Configuration) -> io::Result<Self> {
        Self::bind_inner(configuration, None, None, None, None, None, None).await
    }

    /// Consume a pinned external client. No witness failure permits fallback.
    pub async fn bind_with_witness(
        configuration: Configuration,
        witness: ExternalWitness,
    ) -> io::Result<Self> {
        Self::bind_inner(configuration, Some(witness), None, None, None, None, None).await
    }

    /// Bind a separately authorized native management service in the same custody
    /// lifetime. Provision its new credential privately before starting run.
    pub async fn bind_with_management(
        configuration: Configuration,
        witness: Option<ExternalWitness>,
        policy: ManagementPolicy,
    ) -> io::Result<Self> {
        Self::bind_inner(configuration, witness, Some(policy), None, None, None, None).await
    }

    /// Explicit operator recovery with an independently pinned head and witness.
    /// It never retries a prior request or implies its origin effect completed.
    pub async fn bind_recovered(
        configuration: Configuration,
        witness: ExternalWitness,
        tail: crate::audit::storage::TailRecovery,
        management: Option<ManagementPolicy>,
    ) -> io::Result<Self> {
        Self::bind_inner(
            configuration,
            Some(witness),
            management,
            Some(tail),
            None,
            None,
            None,
        )
        .await
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
            None,
            None,
        )
        .await
    }

    /// Combine explicit witnessed WAL recovery with authenticated settings
    /// recovery. Unwitnessed operational commits remain refused.
    pub async fn bind_recovered_with_settings(
        configuration: Configuration,
        witness: ExternalWitness,
        tail: crate::audit::storage::TailRecovery,
        management: Option<ManagementPolicy>,
        settings: SettingsPolicy,
    ) -> io::Result<Self> {
        Self::bind_inner(
            configuration,
            Some(witness),
            management,
            Some(tail),
            Some(settings),
            None,
            None,
        )
        .await
    }

    /// Explicit native buffered TLS/H1 ingress with the same custody lifetime.
    /// Server keys/configuration arrive from the trusted installation owner.
    pub async fn bind_tls(
        configuration: Configuration,
        witness: Option<ExternalWitness>,
        management: Option<ManagementPolicy>,
        recovery: Option<crate::audit::storage::TailRecovery>,
        settings: Option<SettingsPolicy>,
        tls: Arc<rustls::ServerConfig>,
    ) -> io::Result<Self> {
        Self::bind_inner(
            configuration,
            witness,
            management,
            recovery,
            settings,
            Some(tls),
            None,
        )
        .await
    }

    /// Enable resource-only writes using an independently provisioned operator.
    /// Destructive controls and detection policy remain unavailable here.
    pub async fn bind_with_operator(
        configuration: Configuration,
        witness: ExternalWitness,
        management: ManagementPolicy,
        operator: crate::gateway::management::OperatorPolicy,
        settings: SettingsPolicy,
        recovery: Option<crate::audit::storage::TailRecovery>,
        tls: Option<Arc<rustls::ServerConfig>>,
    ) -> io::Result<Self> {
        Self::bind_inner(
            configuration,
            Some(witness),
            Some(management),
            recovery,
            Some(settings),
            tls,
            Some(operator),
        )
        .await
    }

    async fn bind_inner(
        mut configuration: Configuration,
        witness: Option<ExternalWitness>,
        management_policy: Option<ManagementPolicy>,
        recovery: Option<crate::audit::storage::TailRecovery>,
        settings_policy: Option<SettingsPolicy>,
        tls: Option<Arc<rustls::ServerConfig>>,
        operator: Option<crate::gateway::management::OperatorPolicy>,
    ) -> io::Result<Self> {
        configuration
            .validate()
            .map_err(|_| io::Error::other("configuration invalid"))?;
        if let Some(tls) = &tls {
            crate::gateway::tls::validate_configuration(&configuration, tls)?;
        }
        if operator.is_some()
            && (management_policy.is_none() || settings_policy.is_none() || witness.is_none())
        {
            return Err(io::Error::other(
                "operator requires management and witnessed durable settings",
            ));
        }
        if let (Some(operator), Some(policy)) = (&operator, &management_policy) {
            operator.validate_observer(&policy.credential)?;
        }
        let witness_durability = witness.as_ref().map(ExternalWitness::durability_name);
        let mut sealed = match (witness, recovery) {
            (Some(witness), Some(tail)) => crate::gateway::startup::recover_audit_with_witness(
                &mut configuration,
                witness,
                tail,
            ),
            (Some(witness), None) => {
                crate::gateway::startup::open_audit_with_witness(&mut configuration, witness)
            }
            (None, None) => crate::gateway::startup::open_audit(&mut configuration),
            (None, Some(_)) => return Err(io::Error::other("recovery requires external witness")),
        }
        .map_err(io::Error::other)?;
        let settings = if let Some(policy) = settings_policy {
            let owner = crate::gateway::startup::native_owner(&configuration.owner)
                .map_err(io::Error::other)?;
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
        let mut profile = readiness::profile(
            &configuration,
            witness_durability,
            recovery,
            management_policy.is_some(),
            settings.as_ref().map(|store| store.snapshot().revision()),
        );
        if tls.is_some() {
            readiness::tls_h1(&mut profile, &configuration);
        }
        profile["administration_writable"] = serde_json::json!(operator.is_some());
        let profile_shared = Arc::new(std::sync::RwLock::new(profile.clone()));
        let (operator, commands) = if let Some(operator) = operator {
            let (sender, receiver) = tokio::sync::mpsc::channel(1);
            (Some((operator, sender)), Some(receiver))
        } else {
            (None, None)
        };
        let management = management_policy
            .map(|policy| {
                Management::bind(policy, &configuration, profile_shared.clone(), operator)
            })
            .transpose()?;
        let mut application = Application::new(
            configuration,
            Default::default(),
            Box::new(SharedAudit(audit.clone())),
        )
        .map_err(|_| io::Error::other("application startup"))?;
        if let Some(settings) = settings {
            application.install_settings(settings, audit.clone(), profile_shared, commands);
        }
        if let Some(tls) = &tls {
            application.validate_tls_h1(tls)?;
        }
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
            tls,
        })
    }
    /// No raw credentials or audit material are included.
    pub fn readiness(&self) -> &Readiness {
        &self.readiness
    }
    /// Authenticated persisted operational revision, absent when not configured.
    pub fn operational_revision(&self) -> Option<u64> {
        self.application.operational_revision()
    }

    /// Separate management capability for the trusted broker's private channel.
    /// It is deliberately absent from the serializable readiness document.
    pub fn management_credential_for_private_provisioning(&self) -> Option<&str> {
        self.management
            .as_ref()
            .map(Management::credential_for_private_provisioning)
    }
    /// Independent resource operator capability for the trusted private broker.
    pub fn operator_credential_for_private_provisioning(&self) -> Option<&str> {
        self.management
            .as_ref()
            .and_then(Management::operator_credential_for_private_provisioning)
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
            lifecycle::run(
                &mut self.application,
                self.listener,
                management,
                shutdown,
                self.tls,
            )
            .await
        } else if let Some(tls) = self.tls {
            self.application
                .serve_tls_until(self.listener, tls, shutdown)
                .await
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
