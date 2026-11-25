//! G7 lab: the shipped runtime binary generates its installation key inside the
//! Docker topology, the challenger pins the enrollment public key, then sends
//! fresh nonces and verifies every answer. Unknown targets must be refused.
//! Not hardware, loader, listener or cross-host transport attestation.
use crate::installation::Installation;
use humanymous_runtime::gateway::attestation::{
    self, Attestation, CONFIGURATION_FILE, ENROLLMENT_SCHEMA, Expected, REQUEST_SCHEMA, Refusal,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Answer {
    success: bool,
    stdout: Vec<u8>,
    refusal: String,
}

/// Fixed sibling of the supervisor executable, as for the wargame child.
fn runtime() -> Result<PathBuf> {
    let supervisor = std::env::current_exe()?.canonicalize()?;
    let path = supervisor
        .parent()
        .ok_or("supervisor directory")?
        .join("humanymous-runtime");
    if !std::fs::symlink_metadata(&path)?.file_type().is_file() {
        return Err("runtime must be a regular installed image".into());
    }
    Ok(path)
}

fn digest(path: &Path) -> Result<String> {
    Ok(hex::encode(Sha256::digest(std::fs::read(path)?)))
}

fn invoke(binary: &Path, mode: &str, request: &Value) -> Result<Answer> {
    let mut child = Command::new(binary)
        .arg(mode)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut input = child.stdin.take().ok_or("request pipe")?;
    input.write_all(&serde_json::to_vec(request)?)?;
    drop(input);
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("attestation answer deadline".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .ok_or("answer pipe")?
        .take(65_536)
        .read_to_end(&mut stdout)?;
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .ok_or("refusal pipe")?
        .take(4_096)
        .read_to_string(&mut stderr)?;
    let refusal = stderr
        .trim()
        .strip_prefix("humanymous-runtime: ")
        .unwrap_or("")
        .to_owned();
    Ok(Answer {
        success: status.success(),
        stdout,
        refusal,
    })
}

fn nonce() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// One provisioned target: actual installation plus its exact configuration file.
struct Target {
    installation: Installation,
    configuration: Vec<u8>,
}
impl Target {
    fn new(scope: &str) -> Result<Self> {
        let mut installation = Installation::new(false)?;
        installation.config["scope"]["installation"] = json!(scope);
        let configuration = serde_json::to_vec(&installation.config)?;
        write_private(
            &installation.directory.join(CONFIGURATION_FILE),
            &configuration,
        )?;
        Ok(Self {
            installation,
            configuration,
        })
    }
    fn directory(&self) -> &Path {
        &self.installation.directory
    }
    fn configuration_sha256(&self) -> String {
        hex::encode(Sha256::digest(&self.configuration))
    }
}

fn enroll(binary: &Path, target: &Target) -> Result<(Value, [u8; 32])> {
    let answer = invoke(
        binary,
        "attestation-provision",
        &json!({"schema":REQUEST_SCHEMA,"installation_directory":target.directory()}),
    )?;
    if !answer.success {
        return Err(format!("enrollment refused: {}", answer.refusal).into());
    }
    let enrollment: Value = serde_json::from_slice(&answer.stdout)?;
    let mut pinned = [0u8; 32];
    let public = enrollment["public_key"].as_str().ok_or("public key")?;
    if enrollment["schema"] != ENROLLMENT_SCHEMA || public.len() != 64 {
        return Err("enrollment schema".into());
    }
    for (index, byte) in pinned.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&public[index * 2..index * 2 + 2], 16)?;
    }
    Ok((enrollment, pinned))
}

fn challenge(
    binary: &Path,
    target: &Target,
    installation: &str,
    nonce: &str,
) -> Result<(Answer, Option<Attestation>)> {
    let answer = invoke(
        binary,
        "attestation-answer",
        &json!({"schema":REQUEST_SCHEMA,"installation_directory":target.directory(),
            "installation":installation,"nonce":nonce}),
    )?;
    let parsed = if answer.success {
        Some(serde_json::from_slice(&answer.stdout)?)
    } else {
        None
    };
    Ok((answer, parsed))
}

fn outcome(result: std::result::Result<(), Refusal>) -> &'static str {
    match result {
        Ok(()) => "accepted",
        Err(refusal) => refusal.code(),
    }
}

fn row(id: &str, passed: bool, observed: Value) -> Value {
    json!({"id":id,"status":if passed{"passed"}else{"failed"},"observed":observed})
}

fn execute() -> Result<Value> {
    let binary = runtime()?;
    let binary_sha256 = digest(&binary)?;
    let own = Target::new("pilot")?;
    let (enrollment, pinned) = enroll(&binary, &own)?;
    let configuration_sha256 = own.configuration_sha256();
    let check = |attestation: &Attestation, nonce: &str| {
        attestation::verify(
            &pinned,
            &Expected {
                installation: "pilot",
                binary_sha256: &binary_sha256,
                configuration_sha256: &configuration_sha256,
                nonce,
            },
            attestation,
        )
    };
    let mut rows = Vec::new();
    rows.push(row(
        "enrollment-binds-configuration",
        enrollment["installation"] == "pilot"
            && enrollment["configuration_sha256"] == configuration_sha256.as_str(),
        json!({"installation":enrollment["installation"]}),
    ));

    // installation-target-binding: fresh nonce, pinned key, exact digests.
    let first = nonce()?;
    let (answer, attested) = challenge(&binary, &own, "pilot", &first)?;
    let bound = attested.as_ref().map(|a| check(a, &first));
    rows.push(row(
        "installation-target-binding",
        answer.success && bound == Some(Ok(())),
        json!({"answered":answer.success,"verification":bound.map(outcome)}),
    ));

    // A previous answer is never valid for a new challenge.
    let second = nonce()?;
    let replay = attested.as_ref().map(|a| check(a, &second));
    rows.push(row(
        "stale-nonce-refused",
        replay == Some(Err(Refusal::NonceMismatch)),
        json!({"verification":replay.map(outcome)}),
    ));

    // Tampered signature on the genuine answer.
    let tampered = attested.as_ref().map(|a| {
        let mut forged = a.clone();
        let flipped = if forged.signature.starts_with('0') {
            "1"
        } else {
            "0"
        };
        forged.signature.replace_range(0..1, flipped);
        check(&forged, &first)
    });
    rows.push(row(
        "tampered-signature-refused",
        tampered == Some(Err(Refusal::Signature)),
        json!({"verification":tampered.map(outcome)}),
    ));

    // Unknown target: same claimed installation, its own in-topology key.
    let impostor = Target::new("pilot")?;
    let (_, impostor_pinned) = enroll(&binary, &impostor)?;
    let third = nonce()?;
    let (answer, foreign) = challenge(&binary, &impostor, "pilot", &third)?;
    let unknown = foreign.as_ref().map(|a| check(a, &third));
    rows.push(row(
        "unknown-key-refused",
        answer.success && impostor_pinned != pinned && unknown == Some(Err(Refusal::UnpinnedKey)),
        json!({"answered":answer.success,"verification":unknown.map(outcome)}),
    ));

    // Mismatched installation: the target refuses a challenge addressed to
    // another installation, and the challenger refuses a foreign statement.
    let other = Target::new("foreign")?;
    let (_, other_pinned) = enroll(&binary, &other)?;
    let fourth = nonce()?;
    let (refused, _) = challenge(&binary, &other, "pilot", &fourth)?;
    let (answer, statement) = challenge(&binary, &other, "foreign", &fourth)?;
    let mismatched = statement.as_ref().map(|a| {
        attestation::verify(
            &other_pinned,
            &Expected {
                installation: "pilot",
                binary_sha256: &binary_sha256,
                configuration_sha256: &other.configuration_sha256(),
                nonce: &fourth,
            },
            a,
        )
    });
    rows.push(row(
        "mismatched-installation-refused",
        !refused.success
            && refused.stdout.is_empty()
            && refused.refusal == Refusal::InstallationMismatch.code()
            && answer.success
            && mismatched == Some(Err(Refusal::InstallationMismatch)),
        json!({"target_refusal":refused.refusal,"verification":mismatched.map(outcome)}),
    ));

    // A changed configuration on the pinned installation is a different target.
    let path = own.directory().join(CONFIGURATION_FILE);
    let mut changed: Value = serde_json::from_slice(&own.configuration)?;
    changed["scope"]["revision"] = json!("r2");
    write_private(&path, &serde_json::to_vec(&changed)?)?;
    let fifth = nonce()?;
    let (answer, drifted) = challenge(&binary, &own, "pilot", &fifth)?;
    let drift = drifted.as_ref().map(|a| check(a, &fifth));
    write_private(&path, &own.configuration)?;
    rows.push(row(
        "changed-configuration-refused",
        answer.success && drift == Some(Err(Refusal::ConfigurationMismatch)),
        json!({"answered":answer.success,"verification":drift.map(outcome)}),
    ));

    // Positive control after every negative: the pinned target still verifies.
    let sixth = nonce()?;
    let (answer, control) = challenge(&binary, &own, "pilot", &sixth)?;
    let restored = control.as_ref().map(|a| check(a, &sixth));
    rows.push(row(
        "positive-control-after-negatives",
        answer.success && restored == Some(Ok(())),
        json!({"answered":answer.success,"verification":restored.map(outcome)}),
    ));

    let passed = rows.iter().all(|row| row["status"] == "passed");
    Ok(
        json!({"schema":"humanymous.target-attestation-lab/1","status":if passed{"passed"}else{"failed"},
        "binary_sha256":binary_sha256,"configuration_sha256":configuration_sha256,
        "pinned_public_key":enrollment["public_key"],"cases":rows,
        "key_generated_by":"shipped runtime binary inside this container at lab start",
        "private_key_exported":false,"distinct_nonces":6,
        "not_qualified":["hardware/OS loader attestation","listener-to-key binding","cross-host network transport","owner or deployment key"],
        "full_product_qualified":false}),
    )
}

pub fn run() {
    let report = execute().unwrap_or_else(|error| {
        json!({"schema":"humanymous.target-attestation-lab/1","status":"failed","cases":[],
            "error":error.to_string(),"full_product_qualified":false})
    });
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &report).expect("attestation report output");
    stdout.write_all(b"\n").expect("attestation report output");
    if report["status"] != "passed" {
        std::process::exit(1);
    }
}
