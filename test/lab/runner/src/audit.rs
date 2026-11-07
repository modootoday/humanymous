//! Image/lock scans are control-plane evidence, separate from product traffic.
use crate::{
    docker::{Docker, Result},
    snapshot,
};
use humanymous_platform::testing::process::capture_with_stderr;
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command, time::Duration};
const SCANNER: &str =
    "aquasec/trivy@sha256:8f6bb947dbc528902e11f2f5be52f7b519f6a39d5d148f41166f2040499768fb";
pub fn run(args: &[String]) -> Result<i32> {
    if args.len() != 2 || args[0] != "--evidence" {
        return Err("audit --evidence EXISTING_PRIVATE_BUNDLE required".into());
    }
    let root = snapshot::root()?;
    let evidence = PathBuf::from(&args[1]).canonicalize()?;
    if !evidence.starts_with(root.join(".spec.local/root/evidence"))
        || !evidence.join("index.evidence.md").is_file()
    {
        return Err("private evidence bundle required".into());
    }
    if evidence.join("audit-report.json").exists() {
        return Err("refused: existing audit result must not be overwritten".into());
    }
    let mut nonce = [0; 8];
    getrandom::fill(&mut nonce)?;
    let project = format!(
        "hm-rust-lab-{}",
        nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    let temporary = snapshot::Snapshot(std::env::temp_dir().canonicalize()?.join(&project));
    fs::create_dir(&temporary.0)?;
    let cache = temporary.0.join("cache");
    fs::create_dir(&cache)?;
    let mut docker = Docker::new(&root);
    let mut scans = Vec::new();
    let mut clean = true;
    for role in [
        "controls",
        "runtime",
        "collector",
        "witness",
        "browser-product",
    ] {
        let path = evidence.join(format!("{role}-image.json"));
        if !path.is_file() {
            continue;
        }
        let image: Value = serde_json::from_slice(&fs::read(path)?)?;
        let id = image["Id"].as_str().ok_or("recorded image ID missing")?;
        if id.len() != 71
            || !id.starts_with("sha256:")
            || !id[7..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("recorded immutable image ID invalid".into());
        }
        let archive = temporary.0.join(format!("{role}.tar"));
        docker.call(&[
            "image",
            "save",
            id,
            "-o",
            archive.to_str().ok_or("image archive encoding")?,
        ])?;
        let archive_sha = {
            use sha2::{Digest, Sha256};
            use std::io::Read;
            let mut source = fs::File::open(&archive)?;
            let mut hash = Sha256::new();
            let mut buf = [0; 65536];
            loop {
                let count = source.read(&mut buf)?;
                if count == 0 {
                    break;
                }
                hash.update(&buf[..count]);
            }
            format!("{:x}", hash.finalize())
        };
        let args = vec![
            "run".into(),
            "--rm".into(),
            "--read-only".into(),
            "--cap-drop".into(),
            "ALL".into(),
            "--security-opt".into(),
            "no-new-privileges".into(),
            "--tmpfs".into(),
            "/tmp:rw,noexec,nosuid,size=512m".into(),
            "--mount".into(),
            format!("type=bind,source={},target=/root/.cache", cache.display()),
            "--mount".into(),
            format!(
                "type=bind,source={},target=/scan/image.tar,readonly",
                archive.display()
            ),
            SCANNER.into(),
            "image".into(),
            "--input".into(),
            "/scan/image.tar".into(),
            "--scanners".into(),
            "vuln,secret".into(),
            "--format".into(),
            "json".into(),
            "--quiet".into(),
        ];
        let result = docker.invoke(&args, &[], false)?;
        if let Some(last) = docker.transcript.last_mut() {
            last["stdout"] = json!("scanner stdout is sanitized before evidence storage");
        }
        if !result.success {
            clean = false;
            scans.push(json!({"role":role,"image":id,"status":"not_run"}));
            continue;
        }
        let mut report: Value = serde_json::from_str(&result.stdout)?;
        let mut vulnerabilities = 0;
        let mut secrets = 0;
        if let Some(targets) = report["Results"].as_array_mut() {
            for target in targets {
                vulnerabilities += target["Vulnerabilities"].as_array().map_or(0, Vec::len);
                if let Some(values) = target["Secrets"].as_array_mut() {
                    secrets += values.len();
                    for secret in values {
                        let mut sanitized = json!({});
                        for field in ["RuleID", "Title", "Severity", "StartLine", "EndLine"] {
                            if let Some(value) = secret.get(field) {
                                sanitized[field] = value.clone();
                            }
                        }
                        *secret = sanitized;
                    }
                }
            }
        }
        // The Docker transcript could contain secret matches from the scanner;
        // retain sanitized report rather than raw scanner stdout in that record.
        if let Some(last) = docker.transcript.last_mut() {
            last["stdout"] = json!("scanner stdout sanitized before evidence storage");
        }
        snapshot::write(&evidence, &format!("{role}-trivy.json"), &report)?;
        scans.push(json!({"role":role,"image":id,"archive_sha256":archive_sha,"image_config":report["Metadata"]["ImageID"],"scanned_at":report["CreatedAt"],"vulnerabilities":vulnerabilities,"secret_findings":secrets,"status":if vulnerabilities==0&&secrets==0{"passed"}else{"findings"}}));
        clean &= vulnerabilities == 0 && secrets == 0;
    }
    let db = cache.join("trivy/db/metadata.json");
    let database: Value = if db.is_file() {
        serde_json::from_slice(&fs::read(db)?)?
    } else {
        clean = false;
        json!({"status":"not_run"})
    };
    let lock = evidence.join("built-cargo.lock");
    let source: Value = serde_json::from_slice(&fs::read(evidence.join("source-manifest.json"))?)?;
    let expected = source["files"]
        .as_array()
        .ok_or("source files absent")?
        .iter()
        .find(|f| f["path"] == "Cargo.lock")
        .ok_or("built lock commitment absent")?["sha256"]
        .clone();
    let lock_matches = lock.is_file() && expected == snapshot::digest(&fs::read(&lock)?);
    let mut cargo = Command::new("cargo");
    cargo
        .args(["audit", "--json", "--file"])
        .arg(&lock)
        .current_dir(&root);
    let cargo = capture_with_stderr(&mut cargo, b"", Duration::from_secs(180), 4 * 1024 * 1024)?;
    let cargo_report: Value = serde_json::from_slice(&cargo.stdout)?;
    snapshot::write(&evidence, "cargo-audit.json", &cargo_report)?;
    clean &= cargo.status.success() && lock_matches && !scans.is_empty();
    if docker.engine_identity.is_some() {
        docker.recheck_engine()?;
    }
    snapshot::write(
        &evidence,
        "audit-engine-affinity.private.json",
        &json!({"observation":docker.engine_identity,"actual_sdk_connection_observed":false}),
    )?;
    let summary = json!({"schema":"humanymous.lab-audit/1","scanner":SCANNER,"database":database,"scans":scans,"cargo":{"status":cargo.status.code(),"matches_built_lock":lock_matches,"warnings":cargo_report["warnings"],"vulnerabilities":cargo_report["vulnerabilities"]["count"]},"clean":clean,"full_product_qualified":false});
    snapshot::write(&evidence, "audit-report.json", &summary)?;
    snapshot::write(
        &evidence,
        "audit-transcript.json",
        &json!(docker.transcript),
    )?;
    println!("{summary}");
    Ok(if clean { 0 } else { 1 })
}
