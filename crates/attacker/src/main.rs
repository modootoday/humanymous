//! Local-only simulation CLI. Network execution is deliberately unavailable.
fn seed(args: impl IntoIterator<Item = String>) -> Result<u64, String> {
    let args: Vec<_> = args.into_iter().collect();
    match args.as_slice() {
        [] => Ok(0),
        [flag, value] if flag == "--seed" => value.parse().map_err(|_| "seed must be a u64".into()),
        _ => Err(
            "usage: humanymous-attacker [--seed <u64>]; runtime/target options are unsupported"
                .into(),
        ),
    }
}
fn main() -> std::process::ExitCode {
    let seed = match seed(std::env::args().skip(1)) {
        Ok(seed) => seed,
        Err(error) => {
            eprintln!("{error}");
            return std::process::ExitCode::from(2);
        }
    };
    let report = humanymous_attacker::run(seed);
    if let Err(error) = serde_json::to_writer_pretty(std::io::stdout().lock(), &report) {
        eprintln!("report write failed: {error}");
        return std::process::ExitCode::FAILURE;
    }
    if report.regressions_pass() {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn rejects_network_targets_runtime_and_ambiguous_inputs() {
        for args in [
            vec!["--target", "https://example.invalid"],
            vec!["--runtime"],
            vec!["--seed", "-1"],
            vec!["--seed", "1", "extra"],
        ] {
            assert!(super::seed(args.into_iter().map(str::to_owned)).is_err());
        }
        assert_eq!(super::seed(["--seed".into(), "42".into()]).unwrap(), 42);
    }
}
