//! Native package build/inspection driver. Invokes pinned upstream generation;
//! never generates authored foreign-language application code or evaluates JS.
use sha2::{Digest, Sha256};
#[path = "humanymous-node-package/install.rs"]
mod install;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn run(program: &str, args: &[&str], cwd: &Path) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .env_remove("NODE_OPTIONS")
        .env_remove("BUN_OPTIONS")
        .env_remove("NAPI_RS_NATIVE_LIBRARY_PATH")
        .env_remove("NAPI_RS_FORCE_WASI")
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("{program} failed: {status}"));
    }
    Ok(())
}
fn production_import(runtime: &str, arguments: &[&str], package: &Path) -> Result<(), String> {
    let output = Command::new(runtime)
        .args(arguments)
        .current_dir(package)
        .stdin(Stdio::null())
        .env("HUMANYMOUS_NODE_PRODUCTION_LOAD_REQUIRED", "1")
        .env("HUMANYMOUS_NATIVE_CAPABILITY_RECEIPT", "1")
        .env_remove("HUMANYMOUS_NODE_HOST_PILOT")
        .env_remove("NODE_OPTIONS")
        .env_remove("NODE_PATH")
        .env_remove("BUN_OPTIONS")
        .env_remove("NAPI_RS_NATIVE_LIBRARY_PATH")
        .env_remove("NAPI_RS_FORCE_WASI")
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "production native import failed: {runtime}/{arguments:?}: {}",
            output.status
        ));
    }
    if output.stderr.len() > 1_048_576 {
        return Err("production native import receipt exceeds limit".into());
    }
    let row = production_capability(&output.stdout)?;
    println!("{row}");
    Ok(())
}
fn production_capability(stdout: &[u8]) -> Result<serde_json::Value, String> {
    if stdout.len() > 65_536 {
        return Err("production native capability receipt exceeds limit".into());
    }
    let stdout = std::str::from_utf8(stdout).map_err(|_| "non-UTF8 native capability receipt")?;
    let mut rows = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|row| row["adapter"] == "humanymous.node");
    let row = rows
        .next()
        .ok_or("actual native capability receipt missing")?;
    if rows.next().is_some()
        || row["implementation"] != "rust-node-api"
        || row["hostPilotEnabled"] != false
    {
        return Err("unique production native receipt without fixture feature required".into());
    }
    Ok(row)
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates")
        .parent()
        .expect("workspace")
        .to_owned()
}
fn main() -> Result<(), String> {
    let command = std::env::args()
        .nth(1)
        .ok_or("build | inspect | install-test ARCHIVE | run-host-tests")?;
    let root = root();
    let package = root.join("adapters/node");
    let _generation_lock = if ["build", "inspect"].contains(&command.as_str()) {
        let directory = root.join("target/node-native");
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(directory.join("generation.lock"))
            .map_err(|e| e.to_string())?;
        file.try_lock()
            .map_err(|_| "native generation/inspection is already active".to_owned())?;
        Some(file)
    } else {
        None
    };
    match command.as_str() {
        "install-test" => {
            let archive = std::env::args_os()
                .nth(2)
                .ok_or("install-test requires a native npm archive")?;
            install::execute(Path::new(&archive))?;
        }
        "run-host-tests" => {
            let directory = root.join("target/debug/deps");
            let mut tests = fs::read_dir(&directory)
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with("host_process-"))
                        && if cfg!(windows) {
                            path.extension().is_some_and(|ext| ext == "exe")
                        } else {
                            path.extension().is_none()
                        }
                })
                .collect::<Vec<_>>();
            if tests.len() != 1 {
                return Err("one exact fresh native host test binary is required".into());
            }
            let binary = tests.pop().expect("one exact test");
            let status = Command::new(binary)
                .args(["--include-ignored", "--nocapture", "--test-threads=1"])
                .current_dir(root)
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!("native host tests failed: {status}"));
            }
        }
        "build" => {
            let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
            for (format, file, dts) in [
                ("commonjs", "index.cjs", "index.d.cts"),
                ("esm", "index.mjs", "index.d.mts"),
            ] {
                run(
                    npm,
                    &[
                        "exec",
                        "--",
                        "napi",
                        "build",
                        "--manifest-path",
                        "../../crates/node-adapter/Cargo.toml",
                        "--platform",
                        "--output-dir",
                        "native",
                        "--js",
                        file,
                        "--dts",
                        dts,
                        "--format",
                        format,
                        "--release",
                        "--",
                        "--locked",
                    ],
                    &package,
                )?;
            }
            inspect(&package)?;
        }
        "inspect" => {
            inspect(&package)?;
        }
        _ => return Err("unknown package command".into()),
    }
    Ok(())
}
fn inspect(package: &Path) -> Result<(), String> {
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(package.join("package.json")).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if metadata["main"] != "./native/index.cjs"
        || metadata["exports"]["."]["import"]["default"] != "./native/index.mjs"
        || metadata["files"].as_array().is_none_or(|files| {
            files.iter().any(|file| {
                file.as_str().is_none_or(|file| {
                    file.starts_with("src")
                        || file.starts_with("test")
                        || file.contains("pilot")
                        || file == "native"
                })
            })
        })
    {
        return Err("native-only explicit package public surface required".into());
    }
    let generated = package.join("native");
    let mut records = Vec::new();
    for name in ["index.cjs", "index.mjs", "index.d.cts", "index.d.mts"] {
        let path = generated.join(name);
        let bytes = fs::read(&path).map_err(|_| format!("missing generated {name}"))?;
        if bytes.is_empty() || bytes.contains(&b'\r') {
            return Err(format!("empty or non-LF generated {name}"));
        }
        if !String::from_utf8_lossy(&bytes[..bytes.len().min(512)])
            .contains("auto-generated by NAPI-RS")
        {
            return Err(format!("{name} lacks upstream generated provenance"));
        }
        records.push(serde_json::json!({"path":name,"sha256":hex::encode(Sha256::digest(bytes))}));
    }
    let binaries: Vec<_> = fs::read_dir(&generated)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|e| {
            e.path().extension().is_some_and(|x| x == "node")
                && e.file_name()
                    .to_string_lossy()
                    .starts_with("humanymous-node.")
        })
        .collect();
    if binaries.is_empty() {
        return Err("no native binary; packaging cannot substitute legacy JS".into());
    }
    for binary in binaries {
        let bytes = fs::read(binary.path()).map_err(|e| e.to_string())?;
        records.push(serde_json::json!({"path":binary.file_name().to_string_lossy(),"sha256":hex::encode(Sha256::digest(bytes))}));
    }
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    run(
        npm,
        &[
            "exec",
            "--",
            "tsc",
            "--noEmit",
            "--strict",
            "--types",
            "node",
            "--module",
            "nodenext",
            "--moduleResolution",
            "nodenext",
            "native/index.d.cts",
            "native/index.d.mts",
        ],
        package,
    )?;
    for runtime in ["node", "bun"] {
        for loader in ["native/index.cjs", "native/index.mjs"] {
            production_import(runtime, &[loader], package)?;
        }
    }
    let manifest = serde_json::json!({"generator":"@napi-rs/cli","version":"3.10.12","authoredSource":"Rust1.99.1/edition2024","qualification":"component-only","productionFeatures":[],"package_manifest_sha256":hex::encode(Sha256::digest(fs::read(package.join("package.json")).map_err(|e|e.to_string())?)),"files":records});
    fs::write(
        generated.join("provenance.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!("{manifest}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_feature_receipt_requires_exact_false_not_missing_or_truthiness() {
        let mut value = serde_json::json!({"adapter":"humanymous.node","implementation":"rust-node-api","hostPilotEnabled":false});
        assert!(production_capability(value.to_string().as_bytes()).is_ok());
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!(true),
            serde_json::json!("false"),
            serde_json::json!(0),
        ] {
            value["hostPilotEnabled"] = invalid;
            assert!(production_capability(value.to_string().as_bytes()).is_err());
        }
        value.as_object_mut().unwrap().remove("hostPilotEnabled");
        assert!(production_capability(value.to_string().as_bytes()).is_err());
    }
    #[test]
    fn absent_duplicate_invalid_and_oversized_receipts_cannot_qualify_load() {
        let value = serde_json::json!({"adapter":"humanymous.node","implementation":"rust-node-api","hostPilotEnabled":false});
        assert!(production_capability(b"upstream loader returned without a receipt").is_err());
        assert!(production_capability(format!("{value}\n{value}\n").as_bytes()).is_err());
        assert!(production_capability(&[255]).is_err());
        assert!(production_capability(&vec![b'x'; 65_537]).is_err());
    }
}
