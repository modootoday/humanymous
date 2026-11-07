//! Separately supervised verification-only control-anchor retention process.
#[cfg(any(unix, windows))]
fn main() {
    let result = humanymous_audit::control::retention_service::serve(
        std::io::BufReader::new(std::io::stdin().lock()),
        std::io::stdout().lock(),
    );
    if result.is_err() {
        eprintln!("control retention service unavailable");
        std::process::exit(1);
    }
}
#[cfg(not(any(unix, windows)))]
fn main() {}
