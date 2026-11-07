//! Source-bound native process qualification; missing full requirements stay blocked.
#[cfg(windows)]
mod actor;
mod evidence;
mod fixture;
mod harness;
mod pilot;
mod service;
use std::{io, path::PathBuf};

pub struct Options {
    report: PathBuf,
    binaries: PathBuf,
    manifest: Option<PathBuf>,
    expected_sid: Option<String>,
    source: Option<PathBuf>,
    actor_case: String,
}
fn options(
    args: &mut impl Iterator<Item = String>,
    binaries_required: bool,
) -> io::Result<Options> {
    let mut options = Options {
        report: PathBuf::new(),
        binaries: PathBuf::new(),
        manifest: None,
        expected_sid: None,
        source: None,
        actor_case: "normal".into(),
    };
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| io::Error::other("option value missing"))?;
        match flag.as_str() {
            "--report" if options.report.as_os_str().is_empty() => options.report = value.into(),
            "--binaries" if options.binaries.as_os_str().is_empty() => {
                options.binaries = value.into()
            }
            "--manifest" if options.manifest.is_none() => options.manifest = Some(value.into()),
            "--expected-sid" if options.expected_sid.is_none() => {
                options.expected_sid = Some(value)
            }
            "--source" if options.source.is_none() => options.source = Some(value.into()),
            "--actor-case" if options.actor_case == "normal" && value == "after-grant" => {
                options.actor_case = value
            }
            _ => return Err(io::Error::other("unknown/duplicate native runner option")),
        }
    }
    if !options.report.is_absolute() || (binaries_required && !options.binaries.is_absolute()) {
        return Err(io::Error::other("absolute report/binary paths required"));
    }
    Ok(options)
}
fn run() -> io::Result<i32> {
    let mut args = std::env::args().skip(1);
    let mode = args
        .next()
        .ok_or_else(|| io::Error::other("native|standard-user|service|summarize required"))?;
    let options = options(&mut args, mode != "summarize")?;
    match mode.as_str() {
        "native" => pilot::run(&options),
        "service" => service::run(&options),
        "summarize" => {
            evidence::summary(
                options
                    .source
                    .as_deref()
                    .ok_or_else(|| io::Error::other("private --source report required"))?,
                &options.report,
            )?;
            Ok(0)
        }
        #[cfg(windows)]
        "standard-user" => actor::run(&options, false),
        #[cfg(windows)]
        "standard-user-service" => actor::run(&options, true),
        _ => Err(io::Error::other("unsupported native runner mode")),
    }
}
fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("native runner refused: {error}");
            std::process::exit(1);
        }
    }
}
