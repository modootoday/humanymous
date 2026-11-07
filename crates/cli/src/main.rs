//! Local provisioning command composition root.
use humanymous_cli::Plan;
mod commands;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or("usage: humanymous doctor|install|rollback --artifact PATH --destination PATH --sha256 DIGEST --local-management-verified [--dry-run]")?;
    if command.starts_with("service-") {
        return commands::service(&command, args.collect());
    }
    if !["doctor", "install", "rollback"].contains(&command.as_str()) {
        return Err("unknown command".into());
    }
    let (mut artifact, mut destination, mut digest) = (None, None, None);
    let (mut verified, mut dry) = (false, false);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--artifact" if artifact.is_none() => artifact = Some(args.next().ok_or("artifact value required")?),
            "--destination" if destination.is_none() => destination = Some(args.next().ok_or("destination value required")?),
            "--sha256" if digest.is_none() => digest = Some(args.next().ok_or("digest value required")?),
            "--local-management-verified" if !verified => verified = true,
            "--dry-run" if !dry => dry = true,
            _ => return Err(format!("unsupported or duplicate option: {flag}; elevation and service registration are unavailable").into()),
        }
    }
    println!(
        "os={} arch={} service=none",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let plan = Plan::inspect(
        std::path::Path::new(&artifact.ok_or("--artifact required")?),
        std::path::Path::new(&destination.ok_or("--destination required")?),
        &digest.ok_or("--sha256 required")?,
        verified,
    )?;
    println!("{}", plan.describe());
    if command == "doctor" || dry {
        println!("prerequisites passed; read-only");
    } else if command == "install" {
        println!("changed={}", plan.install()?);
    } else {
        plan.rollback()?;
        println!("removed verified installation");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("refused: {error}");
        std::process::exit(1);
    }
}
