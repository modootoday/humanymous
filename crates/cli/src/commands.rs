//! Composition and strict argument handling for explicitly activated service commands.
use humanymous_cli::{
    release::{SignedRelease, parse_trust_key},
    service::{self, ServicePlan},
};
use std::{collections::BTreeMap, path::Path};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[cfg(feature = "runtime-service")]
pub(super) fn runtime_host(arguments: Vec<String>) -> Result<()> {
    let mut arguments = arguments.into_iter();
    let (mut plan, mut digest) = (None, None);
    while let Some(flag) = arguments.next() {
        match flag.as_str() {
            "--plan" if plan.is_none() => {
                plan = Some(arguments.next().ok_or("--plan value required")?)
            }
            "--plan-sha256" if digest.is_none() => {
                digest = Some(arguments.next().ok_or("--plan-sha256 value required")?)
            }
            _ => return Err("unknown or duplicate runtime-host option".into()),
        }
    }
    humanymous_cli::runtime_host::execute(
        Path::new(&plan.ok_or("--plan required")?),
        &digest.ok_or("--plan-sha256 required")?,
    )?;
    Ok(())
}

/// `humanymous ip-intel update|sources|inspect`: composition only; see
/// `humanymous_cli::ip_intel` and `sot:root/ip-reputation`.
#[cfg(feature = "ip-intel")]
pub(super) fn ip_intel(arguments: Vec<String>) -> Result<()> {
    use humanymous_cli::ip_intel::{self, HttpFetch};
    let mut arguments = arguments.into_iter();
    let now_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "clock unavailable")?
        .as_secs();
    match arguments.next().as_deref() {
        Some("sources") if arguments.next().is_none() => {
            println!("{}", serde_json::to_string_pretty(&ip_intel::registry())?);
            Ok(())
        }
        Some("inspect") => {
            let path = arguments.next().ok_or("inspect PATH required")?;
            if arguments.next().is_some() {
                return Err("inspect takes exactly one PATH".into());
            }
            println!("{}", ip_intel::inspect(Path::new(&path), now_s)?);
            Ok(())
        }
        Some("update") => {
            let plan = update_plan(arguments)?;
            let report = ip_intel::update(&plan, &HttpFetch::default(), now_s)?;
            println!("{}", serde_json::to_string(&report)?);
            if report.partial {
                return Err("snapshot written but incomplete: see missing/invalid sources".into());
            }
            Ok(())
        }
        _ => Err("usage: humanymous ip-intel update --output PATH [--repository DIR] [--source NAME]... [--input NAME=PATH]... [--url NAME=URL]... [--offline] [--allow-unpinned-checkout] | sources | inspect PATH".into()),
    }
}

#[cfg(feature = "ip-intel")]
fn update_plan(
    mut arguments: impl Iterator<Item = String>,
) -> Result<humanymous_cli::ip_intel::UpdatePlan> {
    let mut plan = humanymous_cli::ip_intel::UpdatePlan {
        repository: ".".into(),
        ..Default::default()
    };
    let (mut output, mut repository, mut offline, mut unpinned) = (None, None, false, false);
    while let Some(flag) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .filter(|v| !v.starts_with("--"))
                .ok_or(format!("{flag} value required"))
        };
        match flag.as_str() {
            "--output" if output.is_none() => output = Some(value()?),
            "--repository" if repository.is_none() => repository = Some(value()?),
            "--offline" if !offline => offline = true,
            "--allow-unpinned-checkout" if !unpinned => unpinned = true,
            "--source" => {
                let name = value()?;
                if plan.sources.contains(&name) {
                    return Err(format!("duplicate --source {name}").into());
                }
                plan.sources.push(name);
            }
            "--input" | "--url" => {
                let pair = value()?;
                let (name, value) = pair.split_once('=').ok_or("expected NAME=VALUE")?;
                let duplicate = if flag == "--input" {
                    plan.inputs.insert(name.into(), value.into()).is_some()
                } else {
                    plan.urls.insert(name.into(), value.into()).is_some()
                };
                if duplicate {
                    return Err(format!("duplicate {flag} for {name}").into());
                }
            }
            _ => return Err(format!("unsupported or duplicate ip-intel option: {flag}").into()),
        }
    }
    plan.output = output.ok_or("--output required")?.into();
    if let Some(repository) = repository {
        plan.repository = repository.into();
    }
    plan.offline = offline;
    plan.allow_unpinned = unpinned;
    humanymous_cli::ip_intel::check_plan(&plan)?;
    Ok(plan)
}

fn options(arguments: Vec<String>) -> Result<(BTreeMap<String, String>, bool, bool)> {
    let mut values = BTreeMap::new();
    let (mut activate, mut dry) = (false, false);
    let mut args = arguments.into_iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--activate" if !activate => activate = true,
            "--dry-run" if !dry => dry = true,
            "--manifest" | "--signature" | "--trust-key" | "--artifact" | "--destination" => {
                let value = args.next().ok_or("service option requires a value")?;
                if value.starts_with("--") || values.insert(flag, value).is_some() {
                    return Err("duplicate or missing service option".into());
                }
            }
            _ => return Err(format!("unsupported or duplicate service option: {flag}").into()),
        }
    }
    if activate && dry {
        return Err("--activate and --dry-run are mutually exclusive".into());
    }
    Ok((values, activate, dry))
}

pub(super) fn service(command: &str, arguments: Vec<String>) -> Result<()> {
    if ![
        "service-plan",
        "service-install",
        "service-upgrade",
        "service-doctor",
        "service-rollback",
        "service-remove",
    ]
    .contains(&command)
    {
        return Err("unknown service command".into());
    }
    let (mut options, activate, dry) = options(arguments)?;
    let destination = options
        .remove("--destination")
        .ok_or("--destination required")?;
    let key = parse_trust_key(
        &options
            .remove("--trust-key")
            .ok_or("--trust-key required")?,
    )?;
    let root = Path::new(&destination);
    if ["service-doctor", "service-rollback", "service-remove"].contains(&command) {
        if !options.is_empty() {
            return Err("unexpected artifact/release options for existing deployment".into());
        }
        if command == "service-doctor" {
            if activate || dry {
                return Err("service-doctor does not accept --activate or --dry-run".into());
            }
            let report = service::doctor(root, key)?;
            println!("{}", serde_json::to_string(&report)?);
            if !report.running || !report.diagnostic_ready {
                return Err("service not diagnostically ready".into());
            }
        } else if dry {
            println!("{}", service::existing_plan(root, key, command)?);
        } else if command == "service-rollback" {
            service::rollback(root, key, activate)?;
            println!("{{\"rolled_back\":true,\"product_qualified\":false}}");
        } else {
            service::remove(root, key, activate)?;
            println!("{{\"removed_owned_service\":true,\"product_qualified\":false}}");
        }
        return Ok(());
    }
    let manifest = options.remove("--manifest").ok_or("--manifest required")?;
    let signature = options
        .remove("--signature")
        .ok_or("--signature required")?;
    let artifact = options.remove("--artifact").ok_or("--artifact required")?;
    let release = SignedRelease::read(Path::new(&manifest), signature, &key)?;
    let plan = ServicePlan::inspect(root, Path::new(&artifact), release, key)?;
    if command == "service-upgrade" {
        plan.check_upgrade()?;
    }
    if command == "service-install" {
        plan.check_install()?;
    }
    if command == "service-plan" || dry {
        if activate {
            return Err("service-plan cannot activate".into());
        }
        println!("{}", plan.describe());
    } else if command == "service-install" {
        println!(
            "{{\"changed\":{},\"product_qualified\":false}}",
            plan.install(activate)?
        );
    } else {
        plan.upgrade(activate)?;
        println!("{{\"upgraded\":true,\"product_qualified\":false}}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_service_arguments_never_ignore_flags() {
        for args in [
            vec!["--activate", "--activate"],
            vec!["--activate", "--dry-run"],
            vec!["--artifact"],
            vec!["--unknown"],
            vec!["--trust-key", "a", "--trust-key", "b"],
        ] {
            assert!(options(args.into_iter().map(String::from).collect()).is_err());
        }
        assert!(options(vec!["--dry-run".into()]).unwrap().2);
    }
    #[cfg(feature = "ip-intel")]
    #[test]
    fn strict_ip_intel_update_arguments() {
        let parse = |args: &[&str]| update_plan(args.iter().map(|a| (*a).to_owned()));
        let plan = parse(&[
            "--output",
            "o.json",
            "--source",
            "aws-ip-ranges",
            "--input",
            "gcp-cloud=g.json",
            "--url",
            "azure-service-tags=https://example.test/a.json",
            "--offline",
            "--allow-unpinned-checkout",
        ])
        .unwrap();
        assert_eq!(plan.output, std::path::PathBuf::from("o.json"));
        assert_eq!(plan.repository, std::path::PathBuf::from("."));
        assert!(plan.offline && plan.allow_unpinned && plan.sources == ["aws-ip-ranges"]);
        assert!(!parse(&["--output", "o.json"]).unwrap().allow_unpinned);
        for bad in [
            &[][..],
            &["--output"],
            &["--output", "--offline"],
            &["--output", "a", "--output", "b"],
            &["--output", "a", "--input", "noequals"],
            &["--output", "a", "--url", "x=1", "--url", "x=2"],
            &["--output", "a", "--unknown"],
            &["--output", "a", "--offline", "--offline"],
            // A source named twice, an unknown source, and --url for a source
            // with a fixed or pinned endpoint are refused, never ignored.
            &[
                "--output",
                "a",
                "--source",
                "gcp-cloud",
                "--source",
                "gcp-cloud",
            ],
            &["--output", "a", "--source", "not-a-source"],
            &["--output", "a", "--input", "not-a-source=x"],
            &[
                "--output",
                "a",
                "--url",
                "not-a-source=https://example.test/",
            ],
            &[
                "--output",
                "a",
                "--url",
                "aws-ip-ranges=https://example.test/",
            ],
            &[
                "--output",
                "a",
                "--url",
                "x4bnet-datacenter=https://example.test/",
            ],
            &[
                "--output",
                "a",
                "--allow-unpinned-checkout",
                "--allow-unpinned-checkout",
            ],
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }
}
