//! Private inherited configuration, durable audit startup and native listener.
use humanymous_runtime::gateway::service::{ServiceHost, read_configuration};
use std::io::{self, IsTerminal, Write};

fn run() -> Result<(), &'static str> {
    if std::env::args_os().len() != 1 || io::stdin().is_terminal() || io::stdout().is_terminal() {
        return Err("private inherited configuration/readiness pipes required");
    }
    let config = read_configuration(io::stdin().lock()).map_err(|_| "configuration pipe")?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|_| "runtime")?;
    runtime.block_on(async move {
        let host = ServiceHost::bind(config)
            .await
            .map_err(|_| "service startup")?;
        {
            let mut stdout = io::stdout().lock();
            serde_json::to_writer(&mut stdout, host.readiness()).map_err(|_| "readiness pipe")?;
            stdout
                .write_all(b"\n")
                .and_then(|_| stdout.flush())
                .map_err(|_| "readiness pipe")?;
        }
        // Legacy inherited-pipe entrypoint has no platform shutdown adapter.
        // The CLI/system-service adapter owns and retains the real sender.
        let (_sender, receiver) = tokio::sync::watch::channel(None);
        host.run(receiver)
            .await
            .map(|_| ())
            .map_err(|_| "service stopped")
    })
}
fn main() {
    if let Err(error) = run() {
        eprintln!("humanymous-runtime: {error}");
        std::process::exit(1);
    }
}
