//! Private inherited configuration, durable audit startup and native listener,
//! plaintext loopback or TLS with one explicitly configured certificate source.
//! A non-loopback (public) TLS edge and the loopback admin listener are
//! explicit configuration opt-ins; `core` runs the Core role (owner decision
//! B01 #1).
use humanymous_runtime::gateway::{
    logging::{OperationalLogger, emit_gate_fatal, fatal_class},
    service::{ServiceHost, read_configuration},
    startup::inherited_witness,
    tls::listener,
};
use std::io::{self, IsTerminal, Write};

/// One bounded private request in, one document out. The installation key never
/// crosses the pipe; only the public key or signed statement is written.
fn attestation(mode: &str) -> Result<(), &'static str> {
    use humanymous_runtime::gateway::attestation;
    let mut request = Vec::new();
    io::Read::read_to_end(&mut io::Read::take(io::stdin().lock(), 8_193), &mut request)
        .map_err(|_| "attestation request pipe")?;
    let output = match mode {
        "attestation-provision" => attestation::provision(&request),
        _ => attestation::answer(&request),
    }
    .map_err(|refusal| refusal.code())?;
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(&output)
        .and_then(|_| stdout.write_all(b"\n"))
        .and_then(|_| stdout.flush())
        .map_err(|_| "attestation output pipe")
}

/// Wall-clock record time for operational logs; an unavailable clock stamps 0.
fn since_epoch() -> std::time::Duration {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
}

fn run(log: &mut Option<OperationalLogger>) -> Result<(), &'static str> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if io::stdin().is_terminal() || io::stdout().is_terminal() {
        return Err("private inherited configuration/readiness pipes required");
    }
    match arguments.as_slice() {
        [] => {}
        [mode] if mode == "attestation-provision" || mode == "attestation-answer" => {
            return attestation(mode.to_str().ok_or("attestation mode")?);
        }
        [mode] if mode == "core" => return core(),
        _ => return Err("private inherited configuration/readiness pipes required"),
    }
    let config = read_configuration(io::stdin().lock()).map_err(|_| "configuration pipe")?;
    // Legacy -log-* flags: the optional logging object is validated and its
    // sinks opened before custody; a refusal exits with its static code.
    if let Some(logging) = &config.logging {
        let mut logger = logging
            .open(
                env!("CARGO_PKG_VERSION"),
                &config.audit.node_id,
                Box::new(io::stderr()),
            )
            .map_err(|refusal| refusal.code())?;
        logging.emit_configured(&mut logger, since_epoch());
        *log = Some(logger);
    }
    // B02: a `tls` request selects exactly one certificate source (Go -tls-cert/
    // -tls-key, -acme-*, or the explicit dev certificate) before custody opens.
    // No source, an ambiguous one or an unavailable one fails closed here.
    let mut tls = config
        .tls
        .as_ref()
        .map(listener::prepare)
        .transpose()
        .map_err(|error| error.code())?;
    // Configured inherited witness pipes are pinned before custody is opened.
    let witness = inherited_witness(&config).map_err(|_| "witness input")?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|_| "runtime")?;
    runtime.block_on(async move {
        let host = match (&tls, witness) {
            (Some(tls), witness) => {
                ServiceHost::bind_tls(config, witness, None, None, None, tls.server_config()).await
            }
            (None, Some(witness)) => ServiceHost::bind_with_witness(config, witness).await,
            (None, None) => ServiceHost::bind(config).await,
        }
        .map_err(|_| "service startup")?;
        // ACME renewal (Go autocert) runs beside the bound listener, which
        // answers its TLS-ALPN-01 validation handshakes.
        let maintenance = tls
            .as_mut()
            .and_then(listener::PreparedTls::start_maintenance);
        {
            let mut stdout = io::stdout().lock();
            // Go logged tls_mode; the TLS readiness line carries the selected
            // source name (never a path or key). Plaintext readiness is unchanged.
            match &tls {
                Some(tls) => {
                    let mut ready =
                        serde_json::to_value(host.readiness()).map_err(|_| "readiness pipe")?;
                    ready["tls_mode"] = serde_json::Value::from(tls.mode());
                    serde_json::to_writer(&mut stdout, &ready)
                }
                None => serde_json::to_writer(&mut stdout, host.readiness()),
            }
            .map_err(|_| "readiness pipe")?;
            stdout
                .write_all(b"\n")
                .and_then(|_| stdout.flush())
                .map_err(|_| "readiness pipe")?;
        }
        // Legacy inherited-pipe entrypoint has no platform shutdown adapter.
        // The CLI/system-service adapter owns and retains the real sender.
        let (_sender, receiver) = tokio::sync::watch::channel(None);
        let stopped = host.run(receiver).await;
        if let Some(maintenance) = maintenance {
            maintenance.abort();
        }
        stopped.map(|_| ()).map_err(|_| "service stopped")
    })
}
/// Core role (legacy `cmd/server`, owner decision B01 #1): one private role
/// document in, one readiness line out, then serve until the process ends.
fn core() -> Result<(), &'static str> {
    use humanymous_runtime::gateway::core_server::role::{CoreService, core_tls, read_role};
    let role = read_role(io::stdin().lock()).map_err(|_| "core configuration")?;
    // The certificate source is selected before any key or listener exists.
    let mut tls = role
        .tls
        .as_ref()
        .map(listener::prepare)
        .transpose()
        .map_err(|error| error.code())?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|_| "runtime")?;
    runtime.block_on(async move {
        let service = CoreService::bind(
            &role,
            tls.as_ref().map(core_tls),
            std::sync::Arc::new(std::time::SystemTime::now),
        )
        .await
        .map_err(|_| "core startup")?;
        let maintenance = tls
            .as_mut()
            .and_then(listener::PreparedTls::start_maintenance);
        {
            let mut ready = service.readiness().clone();
            if let Some(tls) = &tls {
                ready["tls_mode"] = serde_json::Value::from(tls.mode());
            }
            let mut stdout = io::stdout().lock();
            serde_json::to_writer(&mut stdout, &ready)
                .map_err(|_| "readiness pipe")
                .and_then(|()| {
                    stdout
                        .write_all(b"\n")
                        .and_then(|()| stdout.flush())
                        .map_err(|_| "readiness pipe")
                })?;
        }
        // Like the inherited-pipe Gate entrypoint, there is no platform
        // shutdown adapter here; the supervisor ends the process.
        let stopped = service.run(std::future::pending()).await;
        if let Some(maintenance) = maintenance {
            maintenance.abort();
        }
        stopped.map_err(|_| "core stopped")
    })
}

fn main() {
    let mut log = None;
    if let Err(error) = run(&mut log) {
        // Legacy gateFatalf: only a fixed class of the static error is logged.
        if let Some(logger) = log.as_mut() {
            emit_gate_fatal(logger, since_epoch(), fatal_class(error));
        }
        eprintln!("humanymous-runtime: {error}");
        std::process::exit(1);
    }
}
