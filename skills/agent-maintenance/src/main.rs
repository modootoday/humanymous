//! Read-only canonical agent checks and bounded, sandboxed Codex evaluations.
mod evaluation;
mod inventory;
mod routing;

use std::{env, path::Path, process::ExitCode};

fn run(args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    match args.first().map(String::as_str) {
        Some("check") if args.len() == 2 => {
            let result = inventory::check(Path::new(&args[1]))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(result.errors.is_empty())
        }
        Some("routing") if args.len() == 2 => {
            let result = routing::evaluate(Path::new(&args[1]))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(result.failed == 0)
        }
        Some("observe-skills") if args.len() == 2 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&inventory::external(Path::new(&args[1]))?)?
            );
            Ok(true)
        }
        Some("normalize") if args.len() == 2 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&inventory::normalize(Path::new(&args[1]))?)?
            );
            Ok(true)
        }
        Some("eval") if args.len() == 5 => evaluation::run(
            Path::new(&args[1]),
            Path::new(&args[2]),
            Path::new(&args[3]),
            &args[4],
        ),
        _ => Err(
            "usage: agent-maintenance check ROOT | routing ROOT | normalize ROOT | observe-skills PATH | eval ROOT CLI NEW_OUTPUT SUITE"
                .into(),
        ),
    }
}

fn main() -> ExitCode {
    match run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
