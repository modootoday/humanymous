//! Existing Runtime ServiceHost with protected inputs and native stop supervision.
#[cfg(windows)]
pub(crate) mod activation;
mod control;
mod failure;
mod input;
#[cfg(all(windows, feature = "runtime-service-pilot"))]
mod pilot;
#[cfg(all(windows, feature = "runtime-service-pilot"))]
pub use pilot::PilotPlan;
mod plan;
mod platform;
/// Opt-in owned-process watchdog boundary fixture, never service qualification.
#[cfg(feature = "runtime-service-pilot")]
pub mod watchdog_fixture;
use control::Supervisor;
pub use failure::{HostFailure, describe_failure};
use humanymous_runtime::authority::credentials::Credential;
use humanymous_runtime::gateway::{
    service::{ServiceHost, ShutdownRequest},
    startup::ExternalWitness,
};
pub use input::{
    HostAuthorizationData, NativeSecrets, SecretSource, bind_authorization_data, read_plan,
};
use plan::invalid;
pub use plan::{HostMode, HostPlan, ObserverPlan, StopPlan};
use serde::Serialize;
use std::{
    io::{self, Read},
    path::Path,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::watch,
};

/// Trusted supervisor sink for a fresh, process-lifetime observer credential.
/// Implementations must use authenticated private transport or the actual service
/// principal's protected store, never readiness JSON, argv/env or plaintext files.
pub trait PrivateObserverProvisioner {
    /// Preserve exact identity/scope and own cleanup on startup failure/exit.
    fn provision(
        &self,
        service_name: &str,
        credential_id: &str,
        credential: &str,
    ) -> io::Result<()>;
}

/// Non-secret readiness bound to verified executable and generation metadata.
#[derive(Serialize)]
pub struct HostReadiness {
    /// CLI envelope schema.
    pub schema: &'static str,
    /// Signed service identity.
    pub service: String,
    /// Signed generation binding, not a management credential.
    pub generation: String,
    /// Exact executing artifact bytes.
    pub executable_sha256: String,
    /// Existing runtime readiness; unsupported producers remain unavailable.
    pub runtime: serde_json::Value,
    /// Listener readiness is never product qualification.
    pub product_qualified: bool,
}

/// Published only after the shared runtime actually drained and flushed.
#[derive(Serialize)]
pub struct HostExit {
    /// CLI envelope schema.
    pub schema: &'static str,
    /// Signed generation binding.
    pub generation: String,
    /// Exact executing artifact bytes.
    pub executable_sha256: String,
    /// Actual complete shutdown.
    pub stopped: bool,
    /// Owned request tasks ended.
    pub drained: bool,
    /// Synchronous audit flush returned success.
    pub audit_flushed: bool,
    /// A watchdog expiry can never produce this successful report.
    pub product_qualified: bool,
}

fn verify_executable(expected: &str) -> io::Result<()> {
    let path = std::env::current_exe()?;
    crate::installation::permissions::plain(&path)?;
    if !std::fs::symlink_metadata(&path)?.is_file() {
        return Err(invalid("regular host executable required"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(256 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 256 * 1024 * 1024 || crate::release::digest(&bytes) != expected {
        return Err(invalid("runtime host executable digest mismatch"));
    }
    Ok(())
}

/// Load pinned non-secret metadata, then enter the requested native ownership mode.
pub fn execute(plan_path: &Path, plan_sha256: &str) -> io::Result<()> {
    let plan = {
        let _preflight = Supervisor::preflight()?;
        let plan = read_plan(plan_path, plan_sha256)?;
        verify_executable(&plan.executable_sha256)?;
        plan
    };
    platform::execute(plan)
}

pub(super) fn runtime() -> io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
}

async fn diagnostic(
    listener: TcpListener,
    response: String,
    mut shutdown: watch::Receiver<Option<ShutdownRequest>>,
) -> io::Result<()> {
    loop {
        if shutdown.borrow().is_some() {
            return Ok(());
        }
        tokio::select! {
            changed=shutdown.changed()=>{if changed.is_err(){return Err(invalid("diagnostic controller closed"));}},
            accepted=listener.accept()=>{
                let(mut stream,peer)=accepted?;if !peer.ip().is_loopback(){return Err(invalid("nonlocal diagnostic peer"));}
                let exchange=async{let mut challenge=[0;19];stream.read_exact(&mut challenge).await?;if &challenge!=b"HUMANYMOUS-READY/1\n"{return Err(invalid("diagnostic challenge"));}stream.write_all(response.as_bytes()).await?;stream.shutdown().await};
                let _=tokio::time::timeout(Duration::from_millis(200),exchange).await;
            }
        }
    }
}

async fn run(
    mut plan: HostPlan,
    source: &impl SecretSource,
    witness: Option<ExternalWitness>,
    observer_sink: Option<&dyn PrivateObserverProvisioner>,
    supervisor: &Supervisor,
    shutdown: watch::Receiver<Option<ShutdownRequest>>,
    publish: impl FnOnce(&HostReadiness) -> io::Result<()>,
) -> io::Result<HostExit> {
    plan.validate()?;
    if plan.observer.is_some() && observer_sink.is_none() {
        return Err(invalid(
            "fresh observer credential private provisioning is not yet available",
        ));
    }
    let configuration = input::configuration(&plan, source, witness.as_ref())?;
    if supervisor.stop_handle().stopping()? {
        return Err(invalid("host stopped during protected input"));
    }
    let host = match plan.observer.take() {
        Some(observer) => {
            let credential =
                Credential::generate().map_err(|_| invalid("observer credential generation"))?;
            let credential_id = observer.credential_id;
            let policy = humanymous_runtime::gateway::management::ManagementPolicy {
                endpoint: observer.endpoint,
                peer: observer.peer,
                stop_budget: plan.stop.budget()?,
                credential,
            };
            let host = ServiceHost::bind_with_management(configuration, witness, policy).await?;
            let token = host
                .management_credential_for_private_provisioning()
                .ok_or_else(|| invalid("private observer credential unavailable"))?;
            observer_sink
                .ok_or_else(|| invalid("private observer provisioner unavailable"))?
                .provision(&plan.service_name, &credential_id, token)?;
            host
        }
        None => match witness {
            Some(w) => ServiceHost::bind_with_witness(configuration, w).await?,
            None => ServiceHost::bind(configuration).await?,
        },
    };
    let readiness = HostReadiness {
        schema: "humanymous.runtime-host.readiness.v1",
        service: plan.service_name.clone(),
        generation: plan.generation.clone(),
        executable_sha256: plan.executable_sha256.clone(),
        runtime: serde_json::to_value(host.readiness())
            .map_err(|_| invalid("runtime readiness encoding"))?,
        product_qualified: false,
    };
    let listener = TcpListener::bind(plan.diagnostic).await?;
    let should_publish = supervisor.ready()?;
    if should_publish {
        publish(&readiness)?;
        supervisor.published()?;
    }
    let diagnostic_shutdown = shutdown.clone();
    let answer = std::mem::take(&mut plan.ready_response);
    let control = supervisor.stop_handle();
    let work = async {
        let result = host.run_report(shutdown).await.map_err(io::Error::other);
        if result.is_err() {
            control.request()?;
        }
        result
    };
    let probe = async {
        let result = diagnostic(listener, answer, diagnostic_shutdown).await;
        if result.is_err() {
            control.request()?;
        }
        result
    };
    let (report, probe) = tokio::join!(work, probe);
    let report = report?;
    probe?;
    Ok(HostExit {
        schema: "humanymous.runtime-host.exit.v1",
        generation: plan.generation,
        executable_sha256: plan.executable_sha256,
        stopped: report.stopped,
        drained: report.drained,
        audit_flushed: report.audit_flushed,
        product_qualified: false,
    })
}

impl HostExit {
    fn report(&self) -> humanymous_runtime::gateway::service::ExitReport {
        humanymous_runtime::gateway::service::ExitReport {
            stopped: self.stopped,
            drained: self.drained,
            audit_flushed: self.audit_flushed,
        }
    }
}

fn start(plan: &HostPlan) -> io::Result<(Supervisor, watch::Receiver<Option<ShutdownRequest>>)> {
    Supervisor::start(Duration::from_millis(plan.startup_ms), plan.stop)
}
