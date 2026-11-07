//! Explicit owned-Docker nginx compatibility pilot, not a product e2e runner.
#![forbid(unsafe_code)]
#[cfg(target_os = "linux")]
#[path = "../pilot/mod.rs"]
mod pilot;

fn main() {
    #[cfg(target_os = "linux")]
    if let Err(error) = pilot::run() {
        eprintln!("nginx pilot failed: {error}");
        std::process::exit(1);
    }
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("unsupported nginx module target; no tests executed");
        std::process::exit(2);
    }
}
