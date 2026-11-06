//! Private-pipe startup and composition root for the local adapter authority.
use humanymous_audit::{
    log::{Config, Log},
    storage::{Wal, WalOptions},
};
use humanymous_authority::{
    configuration::Configuration,
    decision::{ApplicationHttp, Authority},
    transport::{Listener, private_directory},
};
use std::io::{self, IsTerminal, Read, Write};

fn run() -> Result<(), &'static str> {
    if std::env::args_os().len() != 1 || io::stdin().is_terminal() || io::stdout().is_terminal() {
        return Err("private inherited stdin/stdout pipes required; no command-line secrets");
    }
    let mut input = Vec::new();
    io::stdin()
        .take(65_537)
        .read_to_end(&mut input)
        .map_err(|_| "configuration pipe failed")?;
    if input.len() > 65_536 {
        return Err("configuration budget exceeded");
    }
    let config: Configuration =
        serde_json::from_slice(&input).map_err(|_| "invalid configuration JSON")?;
    input.fill(0);
    config.validate().map_err(|_| "invalid configuration")?;
    private_directory(&config.audit.directory, &config.peer)
        .map_err(|_| "audit directory is not private and canonical")?;
    let key = hex::decode(&config.audit.hmac_key_hex).map_err(|_| "invalid audit key")?;
    let seed: [u8; 32] = hex::decode(&config.audit.signing_seed_hex)
        .map_err(|_| "invalid audit seed")?
        .try_into()
        .map_err(|_| "invalid audit seed size")?;
    if !(32..=256).contains(&key.len()) {
        return Err("invalid audit key size");
    }
    let log = Log::open(
        Wal::open(&config.audit.directory, WalOptions::default())
            .map_err(|_| "audit WAL unavailable")?,
        Config {
            node_id: config.audit.node_id.clone(),
            hmac_key: key,
            signing_seed: seed,
            checkpoint_every: 1,
            witness: None,
        },
        config.audit.trusted_head.as_ref(),
        config.audit.allow_new,
    )
    .map_err(|_| "audit authentication/recovery failed")?;
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
            Box::new(log),
        )
        .map_err(|_| "authority startup failed")?;
        let ready = serde_json::json!({"protocol":"humanymous.adapter/0.1", "endpoint":endpoint,
            "credential": authority.credential(), "qualification":"experimental-local-authority", "profile": profile});
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
