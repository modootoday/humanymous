//! Hosted build artifact provenance writer. Does not install, sign or publish.
#![forbid(unsafe_code)]
use humanymous_nginx_adapter::package::{self, Manifest};
use std::{io::Write, path::Path, process::Command};
fn run() -> Result<(), Box<dyn std::error::Error>> {
    if !cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        return Err("unsupported nginx package build tuple".into());
    }
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 4 {
        return Err("expected module, engine, Cargo.lock and new manifest paths".into());
    }
    let module = package::artifact(Path::new(&args[0]), 64 * 1024 * 1024)?;
    let engine = package::artifact(Path::new(&args[1]), 64 * 1024 * 1024)?;
    let lock = package::artifact(Path::new(&args[2]), 4 * 1024 * 1024)?;
    let version = Command::new(&args[1]).arg("-V").output()?;
    if !version.status.success() || version.stderr.len() > 8192 {
        return Err("nginx engine build identity unavailable".into());
    }
    let manifest = Manifest {
        schema: "humanymous.nginx.package/0.1".into(),
        package: "humanymous-nginx-adapter".into(),
        target: package::TARGET.into(),
        rust: "1.99.1".into(),
        nginx: "1.30.6".into(),
        nginx_source_sha256: package::NGINX_SOURCE_SHA256.into(),
        sdk_revision: package::SDK_REV.into(),
        engine_build: String::from_utf8(version.stderr)?,
        source_revision: option_env!("HUMANYMOUS_SOURCE_REV").unwrap_or("").into(),
        source_manifest: option_env!("HUMANYMOUS_SOURCE_MANIFEST")
            .unwrap_or("")
            .into(),
        cargo_lock_sha256: lock.sha256,
        module,
        engine,
        abi_load_qualified: false,
        signed_release: false,
    };
    manifest.validate()?;
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[3])?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("nginx package failed: {error}");
        std::process::exit(1);
    }
}
