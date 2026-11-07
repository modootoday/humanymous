//! Hosted-only native Apache pilot executable.
#[cfg(target_os = "linux")]
#[path = "../pilot/mod.rs"]
mod pilot;

fn main() {
    #[cfg(target_os = "linux")]
    if pilot::run().is_ok() {
        return;
    }
    eprintln!("Apache hosted pilot failed; retain its source-bound report");
    std::process::exit(1);
}
