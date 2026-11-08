//! Owned Docker product-lab coordinator and measured engine/topology controls.
#![recursion_limit = "512"]
mod audit;
#[path = "../../browser_expectations.rs"]
mod browser_expectations;
#[path = "../../browser_outcome.rs"]
mod browser_outcome;
#[path = "../../browser_wire.rs"]
mod browser_wire;
mod clean_room_matrix;
mod docker;
mod fixture;
#[path = "../../mount_policy.rs"]
mod mount_policy;
mod orchestration;
mod provider;
mod snapshot;
mod transfer_applicator;
mod transfer_preimage;
mod unlock_gate;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("run") => orchestration::run(&args[1..]),
        Some("fixture") => fixture::run(&args[1..]).map(|()| 0),
        Some("engine-preflight")=>docker::engine_preflight(&args[1..]),
        Some("provider-preflight") => provider::run(&args[1..]),
        Some("cleanup") => orchestration::cleanup_expired().map(|()| 0),
        Some("audit") => audit::run(&args[1..]),
        Some("matrix-plan") => clean_room_matrix::plan(&args[1..]),
        Some("matrix-completeness") => clean_room_matrix::completeness(&args[1..]),
        Some("transfer-probe") => transfer_applicator::run(&args[1..]),
        Some("transfer-apply") => {
            let mut rest = args[1..].to_vec();
            if !rest.iter().any(|arg| arg == "--apply") {
                rest.push("--apply".into());
            }
            transfer_applicator::run(&rest)
        }
        Some("transfer-verify") => transfer_preimage::verify(&args[1..]),
        Some("unlock-probe") => unlock_gate::probe(&args[1..]),
        _ => Err("usage: humanymous-lab-runner run --evidence BUNDLE [--collector | --browser-product | --go-product | --mount-probe | --authority-node-ipc | --w2-h2-product-binding | --w2-h2-hosted-venue | --n06-owner-join-readiness | --g2-collector-product-binding | --g2-collector-product-binding-readiness | --g5-witness-transport | --g5-witness-transport-honesty | --clean-room-product | --browser-ready-interactive-at-honesty | --n06-owner-auth-handoff-honesty | --g4-cef-at] [--require-full | --require-go-component] | engine-preflight --evidence BUNDLE | audit --evidence BUNDLE | fixture ROLE | cleanup | provider-preflight MANIFEST | matrix-plan --matrix PATH [--emit PATH] | matrix-completeness --matrix PATH [--emit PATH] [--expect-head SHA] [--workspace PATH] | transfer-probe --manifest PATH [--workspace PATH] [--emit PATH] | transfer-apply --manifest PATH --grant PATH [--workspace PATH] [--dry-run] [--emit PATH] | transfer-verify --manifest PATH [--root PATH] [--emit PATH] [--expect-applied] | unlock-probe [--manifest PATH] [--root PATH] [--emit PATH]".into()),
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("lab refused: {error}");
            std::process::exit(1);
        }
    }
}
