//! `ml-train`: offline bootstrap trainer for the behavioral MLP (legacy
//! `cmd/ml-train`). A thin process shell over
//! [`humanymous_core::model::training::ml_train::run`]; it writes a bundle file
//! only and never stages, signs, promotes or serves a model.
use humanymous_core::model::training::ml_train::run;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(
        &args,
        time::OffsetDateTime::now_utc(),
        &mut std::io::stderr().lock(),
    );
    std::process::exit(code);
}
