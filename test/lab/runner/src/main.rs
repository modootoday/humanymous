//! Owned Docker product-lab coordinator and in-container topology controls.
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
        _ => Err("usage: humanymous-lab-runner run --evidence BUNDLE [--collector] [--require-full] | fixture ROLE | cleanup | provider-preflight MANIFEST".into()),
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("lab refused: {error}");
            std::process::exit(1);
        }
    }
}
