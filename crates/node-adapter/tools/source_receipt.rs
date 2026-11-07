//! Read-only source snapshot utility; compile directly with the pinned Rust
//! toolchain when runtime dependencies are still being integrated.
use std::{fs, io, path::Path, process::Command};
fn git(args: &[&str]) -> io::Result<String> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err(io::Error::other("git read failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
fn main() -> io::Result<()> {
    println!("head\t{}", git(&["rev-parse", "HEAD"])?);
    println!("format\tpath\tworking-blob-sha1\tfiltered-blob-sha1\tlf-only");
    let paths = git(&[
        "ls-files",
        "--cached",
        "--others",
        "--exclude-standard",
        "--",
        "crates/node-adapter",
        "adapters/node/package.json",
        "adapters/node/package-lock.json",
        "adapters/node/Dockerfile.pilot",
        "adapters/node/Dockerfile.pilot.dockerignore",
        "adapters/contracts",
        ".spec/sot/adapter-service-contract.sot.md",
        ".spec/plan/20261107071000-node-adapter.plan.md",
    ])?;
    for path in paths.lines() {
        if !Path::new(path).is_file() {
            continue;
        }
        let bytes = fs::read(path)?;
        let working = git(&["hash-object", "--no-filters", path])?;
        let filtered = git(&["hash-object", &format!("--path={path}"), path])?;
        println!(
            "source\t{path}\t{working}\t{filtered}\t{}",
            !bytes.contains(&b'\r')
        );
    }
    Ok(())
}
