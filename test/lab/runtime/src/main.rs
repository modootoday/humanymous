//! Contextual Docker pilots; native unsupported targets never report success.
#[cfg(unix)]
mod browser_dns;
#[cfg(unix)]
mod browser_product;
#[cfg(unix)]
mod browser_proxy;
#[cfg(any(unix, test))]
mod browser_result;
#[cfg(unix)]
mod client;
#[cfg(unix)]
#[path = "../../dns_wire.rs"]
mod dns_wire;
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
        Some("--browser-gate") => browser_product::gate().expect("owned browser gate"),
        Some("--browser-driver") => browser_product::browser().expect("owned browser runner"),
        Some("--browser-stop") => {
            browser_product::stop().expect("owned public head/shutdown control")
        }
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
