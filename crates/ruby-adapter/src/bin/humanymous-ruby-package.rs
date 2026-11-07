//! Hosted native gem build/install driver using unchanged Ruby/RubyGems tools.
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn drain(mut reader: impl Read) -> std::io::Result<(Vec<u8>, bool)> {
    let mut result = Vec::new();
    let mut truncated = false;
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let keep = n.min(1_048_576 - result.len());
        result.extend_from_slice(&buf[..keep]);
        truncated |= keep != n;
    }
    Ok((result, truncated))
}
fn run(mut command: Command) -> Result<String, String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
        .env_remove("RUBYOPT")
        .env_remove("HUMANYMOUS_HOSTED_NATIVE_PILOT");
    let mut child = Owned(command.spawn().map_err(|e| e.to_string())?);
    let out = child
        .0
        .stdout
        .take()
        .ok_or("native process stdout unavailable")?;
    let err = child
        .0
        .stderr
        .take()
        .ok_or("native process stderr unavailable")?;
    let out = std::thread::spawn(move || drain(out));
    let err = std::thread::spawn(move || drain(err));
    let started = Instant::now();
    let mut expired = false;
    let status = loop {
        if let Some(status) = child.0.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if started.elapsed() > Duration::from_secs(120) {
            expired = true;
            let _ = child.0.kill();
            break child.0.wait().map_err(|e| e.to_string())?;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (stdout, out_truncated) = out
        .join()
        .map_err(|_| "stdout reader failed")?
        .map_err(|e| e.to_string())?;
    let (stderr, err_truncated) = err
        .join()
        .map_err(|_| "stderr reader failed")?
        .map_err(|e| e.to_string())?;
    if expired || !status.success() || out_truncated || err_truncated {
        return Err(format!(
            "native package process failed: status={status} timeout={expired} output_truncated={} stderr={}",
            out_truncated || err_truncated,
            String::from_utf8_lossy(&stderr)
        ));
    }
    String::from_utf8(stdout).map_err(|_| "non-UTF8 native package receipt".into())
}
fn receipt(output: &str, field: &str, expected: &str) -> Result<serde_json::Value, String> {
    let mut values = output
        .lines()
        .filter_map(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .filter(|v| v[field] == expected);
    let value = values
        .next()
        .ok_or("expected native package receipt missing")?;
    if values.next().is_some() {
        return Err("duplicate native package receipt".into());
    }
    Ok(value)
}
fn fresh(parent: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let path = parent.join(format!("{}-{stamp}", std::process::id()));
    fs::create_dir(&path).map_err(|e| e.to_string())?;
    Ok(path)
}
fn build(ruby: &Path, packager: &Path, stage: &Path, metadata: &Path) -> Result<(), String> {
    if !packager.is_absolute() || !packager.is_file() {
        return Err("absolute package-tool native extension required".into());
    }
    let stage = stage.canonicalize().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    fs::File::open(metadata)
        .map_err(|e| e.to_string())?
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("native metadata exceeds limit".into());
    }
    let metadata: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let mut selected = serde_json::Map::new();
    for key in [
        "name", "version", "summary", "authors", "license", "homepage",
    ] {
        selected.insert(
            key.into(),
            metadata
                .get(key)
                .ok_or("native metadata member missing")?
                .clone(),
        );
    }
    let request_dir = fresh(&stage.join(".native-package-requests"))?;
    let request = request_dir.join("request.json");
    fs::write(
        &request,
        serde_json::to_vec(&serde_json::json!({"staging":stage,"metadata":selected}))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut command = Command::new(ruby);
    command
        .arg("--require")
        .arg(packager)
        .current_dir(&stage)
        .env("HUMANYMOUS_NATIVE_GEM_PACKAGE_REQUEST", request)
        .env("HUMANYMOUS_NATIVE_CAPABILITY_RECEIPT", "1");
    let output = run(command)?;
    let capabilities = receipt(&output, "adapter", "humanymous.ruby")?;
    if capabilities["hostPilotEnabled"] != false || capabilities["packageToolEnabled"] != true {
        return Err("expected dedicated package-tool without host fixture".into());
    }
    println!(
        "{}",
        receipt(&output, "schema", "humanymous.native-gem-build/1")?
    );
    Ok(())
}
fn install(ruby: &Path, gem: &Path) -> Result<(), String> {
    let gem = gem.canonicalize().map_err(|e| e.to_string())?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let directory = fresh(&root.join("target/ruby-native/install"))?;
    let mut command = Command::new(ruby);
    command
        .args([
            "-S",
            "gem",
            "install",
            "--local",
            "--no-document",
            "--ignore-dependencies",
            "--install-dir",
        ])
        .arg(&directory)
        .arg(&gem)
        .env_remove("HUMANYMOUS_NATIVE_GEM_PACKAGE_REQUEST")
        .env_remove("HUMANYMOUS_NATIVE_CAPABILITY_RECEIPT");
    run(command)?;
    let mut command = Command::new(ruby);
    command
        .args(["--require", "humanymous_ruby"])
        .env("GEM_HOME", &directory)
        .env("GEM_PATH", &directory)
        .env_remove("RUBYLIB")
        .env_remove("HUMANYMOUS_NATIVE_GEM_PACKAGE_REQUEST")
        .env("HUMANYMOUS_NATIVE_CAPABILITY_RECEIPT", "1");
    let output = run(command)?;
    let capabilities = receipt(&output, "adapter", "humanymous.ruby")?;
    if capabilities["hostPilotEnabled"] != false || capabilities["packageToolEnabled"] != false {
        return Err("fixture/package tool present in production gem".into());
    }
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.native-gem-install/1","status":"clean-install-require-succeeded",
        "capabilities":capabilities,"wholeProductQualified":false,"actualFrameworkQualified":false})
    );
    Ok(())
}
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = if std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref() != Ok("1") {
        Err("native package execution requires explicit hosted opt-in".into())
    } else {
        match args.as_slice() {
        [action,ruby,packager,stage,metadata] if action=="build"=>build(Path::new(ruby),Path::new(packager),Path::new(stage),Path::new(metadata)),
        [action,ruby,gem] if action=="install-test"=>install(Path::new(ruby),Path::new(gem)),
        _=>Err("usage: humanymous-ruby-package build RUBY PACKAGER_EXTENSION STAGE METADATA | install-test RUBY GEM".into()),
    }
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixed_upstream_output_requires_one_matching_capability_receipt() {
        let row =
            serde_json::json!({"adapter":"humanymous.ruby","hostPilotEnabled":false}).to_string();
        let output = format!("upstream package banner\n{row}\n");
        assert_eq!(
            receipt(&output, "adapter", "humanymous.ruby").unwrap()["hostPilotEnabled"],
            false
        );
        assert!(receipt("upstream banner only", "adapter", "humanymous.ruby").is_err());
        assert!(receipt(&format!("{row}\n{row}\n"), "adapter", "humanymous.ruby").is_err());
    }
    #[test]
    fn output_limit_is_not_silent_receipt_success() {
        let input = vec![b'x'; 1_048_577];
        let (captured, truncated) = drain(std::io::Cursor::new(input)).unwrap();
        assert_eq!(captured.len(), 1_048_576);
        assert!(truncated);
    }
}
