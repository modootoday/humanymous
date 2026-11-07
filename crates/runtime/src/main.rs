//! Private inherited configuration, durable audit startup and native listener.
use humanymous_runtime::{application::Application, configuration::Configuration};
use std::io::{self, IsTerminal, Read, Write};
use zeroize::{Zeroize, Zeroizing};

fn run() -> Result<(), &'static str> {
    if std::env::args_os().len() != 1 || io::stdin().is_terminal() || io::stdout().is_terminal() {
        return Err("private inherited configuration/readiness pipes required");
    }
    let mut input = Zeroizing::new(Vec::new());
    io::stdin()
        .take(65_537)
        .read_to_end(&mut input)
        .map_err(|_| "configuration pipe")?;
    if input.len() > 65_536 {
        return Err("configuration budget");
    }
    let mut config: Configuration =
        serde_json::from_slice(&input).map_err(|_| "configuration JSON")?;
    input.zeroize();
    config.validate().map_err(|_| "configuration invalid")?;
    let log = humanymous_runtime::startup::open_audit(&mut config)?;
    let address = config.listen;
    let app = Application::new(config, Default::default(), Box::new(log))
        .map_err(|_| "application startup")?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|_| "runtime")?;
    runtime.block_on(async move {
        let listener=tokio::net::TcpListener::bind(address).await.map_err(|_|"listener")?;
        let ready=serde_json::json!({"listen":listener.local_addr().map_err(|_|"listener address")?.to_string(),"profile":app.readiness()});
        {let mut stdout=io::stdout().lock();
            serde_json::to_writer(&mut stdout,&ready).map_err(|_|"readiness pipe")?;
            stdout.write_all(b"\n").and_then(|_|stdout.flush()).map_err(|_|"readiness pipe")?;}
        app.serve(listener).await.map_err(|_|"listener stopped")
    })
}
fn main() {
    if let Err(error) = run() {
        eprintln!("humanymous-runtime: {error}");
        std::process::exit(1);
    }
}
