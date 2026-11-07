//! Owned Docker product-lab coordinator and in-container topology controls.
mod audit;
mod docker;
mod fixture;
mod orchestration;
mod provider;
mod snapshot;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("run") => orchestration::run(&args[1..]),
        Some("fixture") => fixture::run(&args[1..]).map(|()| 0),
        Some("provider-preflight") => provider::run(&args[1..]),
        Some("cleanup") => orchestration::cleanup_expired().map(|()| 0),
        Some("audit") => audit::run(&args[1..]),
        _ => Err("usage: humanymous-lab-runner run --evidence BUNDLE [--collector | --browser-product | --go-product] [--require-full] | audit --evidence BUNDLE | fixture ROLE | cleanup | provider-preflight MANIFEST".into()),
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("lab refused: {error}");
            std::process::exit(1);
        }
    }
}
