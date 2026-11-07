//! Composition root for explicit native integration planning.
use humanymous_native_cli::{
    arguments::{self, Command},
    manifest, storage,
};
use std::process::ExitCode;

fn run() -> Result<u8, String> {
    let args: Vec<_> = std::env::args_os()
        .skip(1)
        .take(33)
        .map(|arg| {
            arg.into_string()
                .map_err(|_| "arguments must be valid Unicode".to_owned())
        })
        .collect::<Result<_, _>>()?;
    match arguments::parse(args)? {
        Command::Help => println!(
            "humanymous-native init --framework electron|tauri|winit|dotnet|qt|flutter\n  [--target TRIPLE] [--runtime-version VERSION]\n  [--workspace ABSOLUTE_LOCAL_DIRECTORY] [--output humanymous-native.json]\n  [--dry-run | --write]\nDefault: stdout dry-run, no filesystem access. --write creates one new manifest on Windows.\nAdapters are unimplemented; output never enables or qualifies protection.\n--help  --version"
        ),
        Command::Version => println!("humanymous-native {}", env!("CARGO_PKG_VERSION")),
        Command::Init(init) => {
            let destination = init.workspace.as_ref().map(|root| root.join(&init.output));
            let publication = if init.write {
                eprintln!(
                    "manifest destination: {}",
                    destination
                        .as_ref()
                        .ok_or("explicit workspace required")?
                        .display()
                );
                Some(storage::publish(&init).map_err(|error| error.to_string())?)
            } else {
                None
            };
            let report = serde_json::json!({"operation": if init.write {"manifest_written"} else {"dry_run"},
                "output": init.output, "destination": destination, "manifest": manifest::build(&init), "publication": publication});
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
            );
            if publication
                .as_ref()
                .is_some_and(|receipt| !receipt.cleanup_complete)
            {
                return Ok(3);
            }
        }
    }
    Ok(0)
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("refused: {error}");
            ExitCode::from(2)
        }
    }
}
