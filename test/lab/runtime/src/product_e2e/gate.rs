//! Gate container supervisor: provisions synthetic owned sealed custody on the
//! container tmpfs, then runs the real gateway binary with private inherited
//! configuration. The passphrase never leaves this process and its child pipe.
use humanymous_runtime::audit::keys::{
    KeyMaterial,
    custody::{Custody, Owner},
};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn private_directory(path: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

/// Provision, start, report readiness and then wait on the gateway process.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut seed = [0; 32];
    let mut hmac = [0; 32];
    let mut nonce = [0; 8];
    let mut secret = [0; 24];
    for buffer in [
        &mut seed[..],
        &mut hmac[..],
        &mut nonce[..],
        &mut secret[..],
    ] {
        getrandom::fill(buffer)?;
    }
    let passphrase = zeroize::Zeroizing::new(hex(&secret));
    let root = std::env::temp_dir().join(format!("product-e2e-gate-{}", hex(&nonce)));
    private_directory(&root)?;
    let root = root.canonicalize()?;
    let keys = root.join("custody");
    private_directory(&keys)?;
    let mut custody = Custody::acquire(&keys, Owner::CurrentUser)?;
    custody.load_or_create(passphrase.as_bytes(), || {
        KeyMaterial::new(seed, hmac.to_vec(), None, None)
    })?;
    drop(custody);
    let config = super::configuration(
        root.to_str().ok_or("audit path")?,
        keys.to_str().ok_or("custody path")?,
        &passphrase,
    );
    let mut child = Command::new("/usr/local/bin/humanymous-runtime")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    {
        let mut input = child.stdin.take().ok_or("configuration pipe")?;
        serde_json::to_writer(&mut input, &config)?;
        input.flush()?;
    }
    drop(config);
    let stdout = child.stdout.take().ok_or("readiness pipe")?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line);
        let _ = sender.send((result.ok(), line));
    });
    let readiness = match receiver.recv_timeout(Duration::from_secs(60)) {
        Ok((Some(n), line)) if n > 0 => serde_json::from_str::<serde_json::Value>(&line)?,
        _ => {
            let _ = child.kill();
            let status = child.wait()?;
            return Err(format!("gateway did not report readiness: {status}").into());
        }
    };
    // Readiness is a nonsecret manifest; the runner retains it from the log.
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.product-e2e-gate-ready/1","binary":"/usr/local/bin/humanymous-runtime","readiness":readiness})
    );
    let status = child.wait()?;
    Err(format!("gateway exited: {status}").into())
}
