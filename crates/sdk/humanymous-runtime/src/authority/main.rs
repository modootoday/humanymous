//! Private-pipe startup and composition root for the local adapter authority.
use humanymous_runtime::authority::{
    configuration::Configuration,
    decision::{ApplicationHttp, Authority},
    transport::Listener,
};
use std::io::{self, IsTerminal, Read, Write};

fn run() -> Result<(), &'static str> {
    if std::env::args_os().len() != 1 || io::stdin().is_terminal() || io::stdout().is_terminal() {
        return Err("private inherited stdin/stdout pipes required; no command-line secrets");
    }
    let mut input = zeroize::Zeroizing::new(Vec::new());
    io::stdin()
        .take(65_537)
        .read_to_end(&mut input)
        .map_err(|_| "configuration pipe failed")?;
    if input.len() > 65_536 {
        return Err("configuration budget exceeded");
    }
    let mut config: Configuration =
        serde_json::from_slice(&input).map_err(|_| "invalid configuration JSON")?;
    input.fill(0);
    config.validate().map_err(|_| "invalid configuration")?;
    let audit_source = match config.audit.keys.as_ref().map(|keys| &keys.unlock) {
        None => "inline-component-nonproduction",
        Some(
            humanymous_runtime::authority::configuration::UnlockSource::InheritedPrivatePipe {
                ..
            },
        ) => "sealed-inherited-private-pipe-external-provenance-required",
        Some(humanymous_runtime::authority::configuration::UnlockSource::OsServiceStore {
            ..
        }) => "sealed-os-service-store",
    };
    let log =
        humanymous_runtime::authority::startup::open_audit(&mut config.audit, &config.peer, None)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(128)
        .enable_all()
        .build()
        .map_err(|_| "runtime unavailable")?;
    runtime.block_on(async move {
        let listener = Listener::bind(&config).map_err(|_| "restricted IPC bind failed")?;
        let (capacity, rate, timeout) =
            (config.capacity, config.rate_per_second, config.timeout_ms);
        let endpoint = config.endpoint.clone();
        let profile = config.capability_manifest();
        let authority = Authority::new(
            config,
            Default::default(),
            Box::new(ApplicationHttp),
            log,
        )
        .map_err(|_| "authority startup failed")?;
        let ready = serde_json::json!({"protocol":"humanymous.adapter/0.1", "endpoint":endpoint,
            "credential": authority.credential(), "qualification":"experimental-local-authority", "profile": profile,
            "audit_source":audit_source, "external_parent_attested":false});
        let mut stdout = io::stdout().lock();
        serde_json::to_writer(&mut stdout, &ready).map_err(|_| "private readiness pipe failed")?;
        stdout
            .write_all(b"\n")
            .and_then(|_| stdout.flush())
            .map_err(|_| "private readiness pipe failed")?;
        drop(stdout);
        listener
            .serve(authority, capacity, rate, timeout)
            .await
            .map_err(|_| "IPC service stopped")
    })
}
fn main() {
    if let Err(error) = run() {
        eprintln!("humanymous-authority: {error}");
        std::process::exit(1);
    }
}
