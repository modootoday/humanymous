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
mod go;
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
        Some("--go-client-release") => {
            go::client_release().expect("independent Go client namespace release")
        }
        Some("--go-evaluate") => go::evaluate().expect("measured Go case evaluation"),
        Some("--go-backend") => go::backend().expect("actual Go upstream actor"),
        Some("--go-backend-stop") => go::backend_stop().expect("owned Go upstream stop"),
        Some("--go-gate") => go::gate().expect("owned Go Gate actor"),
        Some("--go-client") => go::client().expect("owned Go client actor"),
        Some("--go-count") => go::count().expect("owned Go counter snapshot"),
        Some("--go-qualify") => go::qualify().expect("complete actual Go qualification"),
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
