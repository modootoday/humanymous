//! Read-only canonical agent checks and bounded, sandboxed Codex evaluations.
use humanymous_agent_maintenance::{evaluation, inventory, language, package, paired, product, routing, sessions};

use std::{env, path::Path, process::ExitCode};

fn run(args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    match args.first().map(String::as_str) {
        Some("language-check") if args.len() == 3 => {
            let source = Path::new(&args[1]);
            let preservation = Path::new(&args[2]);
            for path in [source, preservation] {
                let meta = std::fs::symlink_metadata(path)?;
                if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > 2 * 1024 * 1024 {
                    return Err("unsafe/oversized language-check input".into());
                }
            }
            let entries = serde_json::from_slice::<Vec<language::Preservation>>(&std::fs::read(preservation)?)?;
            let report = language::inspect(&args[1], &std::fs::read_to_string(source)?, &entries);
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(report.errors.is_empty() && report.findings.iter()
                .all(|finding| finding.classification == "preserved-exact-line"))
        }
        Some("paired-check") if args.len() == 2 => {
            let path = Path::new(&args[1]);
            let metadata = std::fs::symlink_metadata(path)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 16 * 1024 * 1024 {
                return Err("unsafe/oversized paired result".into());
            }
            let value = serde_json::from_slice(&std::fs::read(path)?)?;
            let result = paired::validate(&value);
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(result.errors.is_empty())
        }
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
        Some("skill-check") if args.len() == 2 => {
            let result = product::inspect(Path::new(&args[1]))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(result.errors.is_empty())
        }
        Some("lane-check") if args.len() == 2 => {
            let result = sessions::catalog::inspect(Path::new(&args[1]))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(result.errors.is_empty())
        }
        Some("thread-check") if args.len() == 2 => {
            let result=sessions::registry::inspect(Path::new(&args[1]))?;
            println!("{}",serde_json::to_string_pretty(&result)?);Ok(result.errors.is_empty())
        }
        Some("session-list") if args.len() == 2 => {
            println!("{}",serde_json::to_string_pretty(&sessions::storage::list(Path::new(&args[1]))?)?);Ok(true)
        }
        Some("session-claim") if args.len() == 3 => {
            let claim=sessions::storage::claim_request_file(Path::new(&args[1]),Path::new(&args[2]))?;
            println!("{}",serde_json::json!({"schema":"humanymous.session-claim.v1","claim":claim,"runtime_qualified":false}));Ok(true)
        }
        Some("session-release") if args.len() == 6 => {
            sessions::storage::release(Path::new(&args[1]),&args[2],&args[3],&args[4],&args[5])?;
            println!("{}",serde_json::json!({"schema":"humanymous.session-release.v1","released":true,"runtime_qualified":false}));Ok(true)
        }
        Some("stage-eval") if args.len() == 4 => {
            let receipt=package::stage(Path::new(&args[1]),Path::new(&args[2]),&args[3])?;
            println!("{}",serde_json::to_string_pretty(&receipt)?);Ok(true)
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
            "usage: agent-maintenance check ROOT | routing ROOT | skill-check ROOT | lane-check ROOT | thread-check ROOT | session-list ROOT | session-claim ROOT REQUEST_JSON_FILE | session-release ROOT LANE PROVIDER SESSION NOTE | stage-eval ROOT NEW_OUTPUT product|developer | normalize ROOT | observe-skills PATH | eval ROOT CLI NEW_OUTPUT SUITE"
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
