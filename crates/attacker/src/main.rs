//! Local-only red-team CLI. It never contacts a target: `--attack <name>`
//! prints that attack's defense-only request plan (steps, headers, bodies and
//! the fingerprint each step would present), `--list-attacks` lists the
//! catalog, and `--seed` runs the deterministic observation-replay regression
//! report. Live execution of the plans is not implemented: the only network
//! client, `wire::client::LoopbackClient`, refuses every non-loopback address,
//! so the Docker lab's container addresses are unreachable from it by design.
//! Running the plans against the lab will need an explicit own-lab address
//! allowlist (recorded in INTEGRATION.md); the loopback guard stays as is.
use humanymous_attacker::wire::attacks;

enum Command {
    Seed(u64),
    ListAttacks,
    Attack(String),
}

fn seed(args: impl IntoIterator<Item = String>) -> Result<u64, String> {
    let args: Vec<_> = args.into_iter().collect();
    match args.as_slice() {
        [] => Ok(0),
        [flag, value] if flag == "--seed" => value.parse().map_err(|_| "seed must be a u64".into()),
        _ => Err(
            "usage: humanymous-attacker [--seed <u64>] | [--list-attacks] | [--attack <name>]; \
             runtime/target options are unsupported"
                .into(),
        ),
    }
}

fn command(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let args: Vec<_> = args.into_iter().collect();
    match args.as_slice() {
        [flag] if flag == "--list-attacks" => Ok(Command::ListAttacks),
        [flag, name] if flag == "--attack" => Ok(Command::Attack(name.clone())),
        _ => seed(args).map(Command::Seed),
    }
}

fn main() -> std::process::ExitCode {
    let command = match command(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{error}");
            return std::process::ExitCode::from(2);
        }
    };
    match command {
        Command::ListAttacks => {
            let names: Vec<_> = attacks::CATALOG
                .iter()
                .map(|spec| {
                    serde_json::json!({
                        "attack": spec.name,
                        "tier": spec.tier,
                        "class": spec.class,
                        "expected": spec.expected,
                    })
                })
                .collect();
            if serde_json::to_writer_pretty(std::io::stdout().lock(), &names).is_err() {
                return std::process::ExitCode::FAILURE;
            }
            std::process::ExitCode::SUCCESS
        }
        Command::Attack(name) => match attacks::select(&name) {
            Some(spec) => {
                if serde_json::to_writer_pretty(std::io::stdout().lock(), &spec.plan_json())
                    .is_err()
                {
                    return std::process::ExitCode::FAILURE;
                }
                std::process::ExitCode::SUCCESS
            }
            None => {
                eprintln!("unknown attack: {name}");
                std::process::ExitCode::from(2)
            }
        },
        Command::Seed(seed) => {
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

    #[test]
    fn command_parser_handles_attack_list_and_rejects_unknown_shapes() {
        use super::Command;
        assert!(matches!(
            super::command(["--list-attacks".to_owned()]).unwrap(),
            Command::ListAttacks
        ));
        assert!(matches!(
            super::command(["--attack".to_owned(), "tls-static".to_owned()]).unwrap(),
            Command::Attack(name) if name == "tls-static"
        ));
        assert!(matches!(super::command([]).unwrap(), Command::Seed(0)));
        // An unknown attack name still PARSES; selection failure is handled in main.
        assert!(super::command(["--target".to_owned(), "x".to_owned()]).is_err());
    }
}
