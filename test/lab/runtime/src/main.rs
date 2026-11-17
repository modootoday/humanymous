//! Contextual Docker pilots; native unsupported targets never report success.
#[cfg(unix)]
mod attack_catalog;
#[cfg(unix)]
mod attestation;
#[cfg(unix)]
mod browser_controls;
#[cfg(unix)]
mod browser_dns;
#[cfg(any(unix, test))]
#[path = "../../browser_expectations.rs"]
mod browser_expectations;
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
mod gate_tls;
#[cfg(unix)]
mod go;
#[cfg(unix)]
mod installation;
#[cfg(any(unix, test))]
#[path = "../../mount_policy.rs"]
mod mount_policy;
#[cfg(unix)]
mod origin;
#[cfg(unix)]
mod origin_audit;
#[cfg(unix)]
mod pass;
#[cfg(unix)]
mod process;
#[cfg(unix)]
mod product_e2e;
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
        Some("--go-egress") => go::egress().expect("owned Go namespace egress control"),
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
        Some("--product-origin") => product_e2e::origin().expect("owned recording origin"),
        Some("--product-gate") => product_e2e::gate().expect("owned Gate supervisor"),
        Some("--human-test-gate") => {
            product_e2e::gate_human_test().expect("human tester Gate supervisor")
        }
        Some("--product-driver") => product_e2e::driver().expect("product e2e driver"),
        Some("--product-health") => product_e2e::health(std::env::args().nth(2).as_deref()),
        Some("--product-witness") => product_e2e::witness().expect("independent audit witness"),
        Some("--product-gate-witnessed") => {
            product_e2e::gate_witnessed().expect("witness-required Gate supervisor")
        }
        Some("--product-driver-witnessed") => {
            product_e2e::driver_witnessed().expect("witnessed product e2e driver")
        }
        Some("--product-driver-origin-lost") => {
            product_e2e::driver_origin_lost().expect("origin-loss product e2e driver")
        }
        Some("--product-dns-canary") => product_e2e::dns_canary().expect("owned DNS canary"),
        Some("--product-dns-probe") => {
            product_e2e::dns_probe().expect("owned DNS canary positive control")
        }
        Some("--wargame") => wargame::run(),
        Some("--attestation") => attestation::run(),
        Some("--pass") => pass::run(),
        Some("--origin-audit-origin") => {
            origin_audit::origin::run().expect("owned backend recording origin")
        }
        Some("--origin-audit-gate") => {
            origin_audit::gate::run().expect("origin/audit Gate supervisor session")
        }
        Some("--origin-audit-client") => origin_audit::client::run(),
        Some("--origin-audit-tamper") => origin_audit::tamper::run(),
        Some("--attack-gate") => {
            attack_catalog::gate::run().expect("attack catalog Gate supervisor")
        }
        Some("--attack-client") => attack_catalog::client::run(),
        Some("--gate-tls") => attack_catalog::gate::run_tls().expect("TLS Gate supervisor"),
        Some("--gate-tls-client") => gate_tls::client::run(),
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
