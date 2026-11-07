//! Source-bound native process qualification; missing full requirements stay blocked.
#[cfg(windows)]
mod actor;
#[cfg(windows)]
mod controlled;
mod evidence;
mod fixture;
mod harness;
mod pilot;
#[cfg(test)]
#[path = "actor/prebuilt.rs"]
mod prebuilt;
mod service;
mod workspace;
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
    #[cfg(windows)]
    if mode == "controlled-capture" {
        return controlled::run(args);
    }
    if mode == "summarize-service" {
        return service::summary::run(args);
    }
    let options = options(&mut args, mode != "summarize")?;
    let result = match mode.as_str() {
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
        "standard-user" => actor::run(&options, actor::Workload::Native),
        #[cfg(windows)]
        "standard-user-service" => actor::run(&options, actor::Workload::Service),
        #[cfg(windows)]
        "standard-user-workspace" => actor::run(&options, actor::Workload::Workspace),
        "workspace" => workspace::run(&options),
        _ => Err(io::Error::other("unsupported native runner mode")),
    };
    if let Err(error) = &result
        && evidence::record_initial_failure(&options.report, &mode, error).is_err()
    {
        eprintln!("initial failure report could not be preserved");
    }
    result
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
