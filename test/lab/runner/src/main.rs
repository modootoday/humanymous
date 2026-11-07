//! Owned Docker product-lab coordinator and measured engine/topology controls.
mod audit;
#[path = "../../browser_expectations.rs"]
mod browser_expectations;
#[path = "../../browser_outcome.rs"]
mod browser_outcome;
mod docker;
mod fixture;
#[path = "../../mount_policy.rs"]
mod mount_policy;
mod orchestration;
mod provider;
mod snapshot;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("run") => orchestration::run(&args[1..]),
        Some("fixture") => fixture::run(&args[1..]).map(|()| 0),
        Some("engine-preflight")=>docker::engine_preflight(&args[1..]),
        Some("provider-preflight") => provider::run(&args[1..]),
        Some("cleanup") => orchestration::cleanup_expired().map(|()| 0),
        Some("audit") => audit::run(&args[1..]),
        _ => Err("usage: humanymous-lab-runner run --evidence BUNDLE [--collector | --browser-product | --go-product | --mount-probe] [--require-full | --require-go-component] | engine-preflight --evidence BUNDLE | audit --evidence BUNDLE | fixture ROLE | cleanup | provider-preflight MANIFEST".into()),
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("lab refused: {error}");
            std::process::exit(1);
        }
    }
}
