//! Contextual Docker pilots; native unsupported targets never report success.
#[cfg(unix)]
mod client;
#[cfg(unix)]
mod installation;
#[cfg(unix)]
mod origin;
#[cfg(unix)]
mod process;
#[cfg(unix)]
mod provisioning;
#[cfg(unix)]
mod scenario;
#[cfg(unix)]
mod wargame;
#[cfg(unix)]
mod witness;

#[cfg(unix)]
fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--wargame") => wargame::run(),
        Some("--witness-transaction") => {
            witness::transaction().expect("private witness transaction")
        }
        Some("--witness-writer") => witness::writer().expect("private writer broker"),
        Some("--witness-key-denied") => witness::key_denied().expect("key access negative"),
        _ => scenario::run(),
    }
}
#[cfg(not(unix))]
fn main() {
    eprintln!("unsupported: Linux Docker process pilot required");
    std::process::exit(2);
}
