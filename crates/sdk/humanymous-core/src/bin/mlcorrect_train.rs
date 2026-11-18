//! `mlcorrect-train`: offline retrain step of the self-correcting loop
//! (legacy `cmd/mlcorrect-train`). A thin process shell over
//! [`humanymous_core::model::training::mlcorrect_train::run`]; it never
//! deploys, stages or activates a model.
use humanymous_core::model::training::mlcorrect_train::{run, version_stamp};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let version = version_stamp(time::OffsetDateTime::now_utc());
    let code = run(
        &args,
        &version,
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    );
    std::process::exit(code);
}
