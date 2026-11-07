//! Hosted-only native Apache pilot executable.
#[path = "../coordinator.rs"]
mod coordinator;
#[cfg(target_os = "linux")]
#[path = "../pilot/mod.rs"]
mod pilot;

fn main() {
    if matches!(
        std::env::args().nth(1).as_deref(),
        Some("coordinate" | "coordinate-cleanup")
    ) {
        if coordinator::run().is_ok() {
            return;
        }
        eprintln!("Apache hosted coordinator failed; retain its private bundle and safe receipt");
        std::process::exit(1);
    }
    if std::env::args_os().len() != 1 {
        eprintln!("Unsupported Apache pilot arguments");
        std::process::exit(1);
    }
    #[cfg(target_os = "linux")]
    if pilot::run().is_ok() {
        return;
    }
    eprintln!("Apache hosted pilot failed; retain its source-bound report");
    std::process::exit(1);
}
