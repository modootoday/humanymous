//! Separately supervised, private inherited-pipe durable witness process.
#[cfg(any(unix, windows))]
fn main() {
    let result = humanymous_runtime::audit::checkpoint::service::serve(
        std::io::BufReader::new(std::io::stdin().lock()),
        std::io::stdout().lock(),
    );
    if result.is_err() {
        eprintln!("witness service unavailable");
        std::process::exit(1);
    }
}

#[cfg(not(any(unix, windows)))]
fn main() {}
