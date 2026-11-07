//! Rust-only network ledger verifier. Full-product acceptance remains separate.
use humanymous_attacker::network::NetworkReport;
use std::{io::Read, path::PathBuf, process::ExitCode};

fn arguments(args: impl Iterator<Item = String>) -> Result<(PathBuf, bool), &'static str> {
    let values: Vec<_> = args.collect();
    match values.as_slice() {
        [path] if !path.starts_with('-') => Ok((path.into(), false)),
        [flag, path] if flag == "--require-full" && !path.starts_with('-') => {
            Ok((path.into(), true))
        }
        _ => Err("usage: humanymous-wargame-verify [--require-full] REPORT.json"),
    }
}
fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let (path, full) = arguments(std::env::args().skip(1)).map_err(std::io::Error::other)?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(4_194_305)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4_194_304 {
        return Err(std::io::Error::other("report size budget").into());
    }
    let report: NetworkReport = serde_json::from_slice(&bytes)?;
    report.validate()?;
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.wargame-verification.v1","internally_consistent":true,"scoped_passed":report.scoped_passed,"full_product_qualified":report.full_product_qualified,"totals":report.totals})
    );
    Ok(report.scoped_passed && (!full || report.full_product_qualified))
}
fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(error) => {
            eprintln!("wargame verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn no_target_or_ambiguous_mode_is_accepted() {
        for args in [
            vec!["--target", "https://example.invalid"],
            vec![],
            vec!["--require-full"],
            vec!["a", "b"],
        ] {
            assert!(super::arguments(args.into_iter().map(str::to_owned)).is_err());
        }
        assert!(
            super::arguments(["--require-full".into(), "report.json".into()].into_iter())
                .unwrap()
                .1
        );
    }
}
