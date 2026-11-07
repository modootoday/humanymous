//! Composition and strict argument handling for explicitly activated service commands.
use humanymous_cli::{
    release::{SignedRelease, parse_trust_key},
    service::{self, ServicePlan},
};
use std::{collections::BTreeMap, path::Path};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
}
