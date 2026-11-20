//! Gate container supervisors. Both provision synthetic owned sealed custody on
//! the container tmpfs. `run` starts the shipped gateway binary with private
//! inherited configuration and no witness input, so it binds writer-only.
//! `tls` starts the same shipped binary with a `tls` section naming a leaf
//! issued by a per-run lab CA minted here, so it binds its TLS listener
//! (`bind_tls`, still writer-only). `witnessed` composes the same library `ServiceHost` with a required
//! external witness in this supervisor. The shipped binary also accepts
//! inherited witness pipes (`src/gateway/main.rs` selects `bind_tls`,
//! `bind_with_witness` or `bind`), but this topology does not yet hand it
//! witness descriptors, so the witnessed Gate is library evidence only.
//! `witnessed` also mirrors the runtime's live epoch latch (`ServiceHost::epoch_health`)
//! into a private tmpfs marker for the exec readiness probe. The passphrase
//! never leaves this process and, for `run`, its child pipe.
use ed25519_dalek::SigningKey;
use humanymous_runtime::{
    audit::{
        checkpoint::{
            WitnessBootstrap,
            service::{WitnessServiceClient, read_frame, write_frame},
        },
        keys::{
            KeyMaterial,
            custody::{Custody, Owner},
        },
        log::WitnessDurability,
    },
    gateway::{configuration::Configuration, service::ServiceHost, startup::ExternalWitness},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn private_directory(path: &std::path::Path) -> io::Result<()> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

/// Freshly provisioned writer custody. Only the public key is ever reported.
struct Provisioned {
    root: PathBuf,
    keys: PathBuf,
    passphrase: Zeroizing<String>,
    writer_public: [u8; 32],
}

impl Provisioned {
    fn configuration(&self) -> Result<Value> {
        Ok(super::configuration(
            self.root.to_str().ok_or("audit path")?,
            self.keys.to_str().ok_or("custody path")?,
            &self.passphrase,
        ))
    }
}

fn provision() -> Result<Provisioned> {
    let mut seed = Zeroizing::new([0; 32]);
    let mut hmac = Zeroizing::new([0; 32]);
    let mut nonce = [0; 8];
    let mut secret = Zeroizing::new([0; 24]);
    for buffer in [
        &mut seed[..],
        &mut hmac[..],
        &mut nonce[..],
        &mut secret[..],
    ] {
        getrandom::fill(buffer)?;
    }
    let passphrase = Zeroizing::new(hex(&secret[..]));
    let root = std::env::temp_dir().join(format!("product-e2e-gate-{}", hex(&nonce)));
    private_directory(&root)?;
    let root = root.canonicalize()?;
    let keys = root.join("custody");
    private_directory(&keys)?;
    let writer_public = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    let mut custody = Custody::acquire(&keys, Owner::CurrentUser)?;
    custody.load_or_create(passphrase.as_bytes(), || {
        KeyMaterial::new(*seed, hmac.to_vec(), None, None)
    })?;
    drop(custody);
    Ok(Provisioned {
        root,
        keys,
        passphrase,
        writer_public,
    })
}

/// Provision, start the shipped binary, report readiness and wait on it.
pub fn run() -> Result<()> {
    let provisioned = provision()?;
    let config = provisioned.configuration()?;
    supervise(config, provisioned, json!({}), || Ok(()))
}

/// Shipped binary with its TLS listener. A per-run lab CA is minted here; the
/// leaf chain and key go to an owner-only tmpfs directory that the private
/// configuration names, and the key file is removed once the gateway has
/// reported readiness (it loads the pair before custody opens). Only the
/// public root leaves this container, in the readiness line.
pub fn tls() -> Result<()> {
    let provisioned = provision()?;
    let issued = super::lab_ca::issue()?;
    let material = provisioned.root.join("tls");
    private_directory(&material)?;
    let chain = material.join("chain.pem");
    let key = material.join("key.pem");
    super::lab_ca::write_private(&chain, issued.chain_pem.as_bytes())?;
    super::lab_ca::write_private(&key, issued.key_pem.as_bytes())?;
    drop(issued.key_pem);
    let config = super::tls_configuration(
        provisioned.root.to_str().ok_or("audit path")?,
        provisioned.keys.to_str().ok_or("custody path")?,
        &provisioned.passphrase,
        chain.to_str().ok_or("chain path")?,
        key.to_str().ok_or("key path")?,
    );
    let extra = json!({"lab_ca":{"schema":"humanymous.product-e2e-lab-ca/1",
        "certificate_pem":issued.root_pem,"sha256":issued.root_sha256,
        "server_name":super::lab_ca::SERVER_NAME,
        "issuance":"minted in the Gate container at start; the CA signing key was dropped after one leaf; the leaf key file is removed after readiness"}});
    supervise(config, provisioned, extra, move || {
        fs::remove_file(&key)?;
        Ok(())
    })
}

/// Start the shipped gateway with one private configuration document, print
/// the nonsecret readiness line (plus `extra`), run `after_ready`, and wait.
fn supervise(
    config: Value,
    provisioned: Provisioned,
    extra: Value,
    after_ready: impl FnOnce() -> Result<()>,
) -> Result<()> {
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
    drop(provisioned);
    let stdout = child.stdout.take().ok_or("readiness pipe")?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line);
        let _ = sender.send((result.ok(), line));
    });
    let readiness = match receiver.recv_timeout(Duration::from_secs(60)) {
        Ok((Some(n), line)) if n > 0 => serde_json::from_str::<Value>(&line)?,
        _ => {
            let _ = child.kill();
            let status = child.wait()?;
            return Err(format!("gateway did not report readiness: {status}").into());
        }
    };
    // Readiness is a nonsecret manifest; the runner retains it from the log.
    let mut line = json!({"schema":"humanymous.product-e2e-gate-ready/1",
        "binary":"/usr/local/bin/humanymous-runtime","readiness":readiness});
    if let (Some(line), Some(extra)) = (line.as_object_mut(), extra.as_object()) {
        line.extend(extra.clone());
    }
    println!("{line}");
    io::stdout().flush()?;
    if let Err(error) = after_ready() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let status = child.wait()?;
    Err(format!("gateway exited: {status}").into())
}

/// Witness-required Gate. Stdin/stdout carry only typed audit witness frames
/// relayed by the trusted runner broker; diagnostics and readiness go to stderr.
pub fn witnessed() -> Result<()> {
    let provisioned = provision()?;
    let mut config = provisioned.configuration()?;
    config["audit"]["require_external_witness"] = json!(true);
    let node = config["audit"]["node_id"]
        .as_str()
        .ok_or("audit node")?
        .to_owned();
    let mut output = io::stdout();
    write_frame(
        &mut output,
        &json!({"role":"writer_identity","writer_public":provisioned.writer_public,"node_id":node}),
    )?;
    let mut input = BufReader::new(io::stdin());
    let intro: Value = serde_json::from_slice(
        &read_frame(&mut input)?.ok_or("broker did not supply witness identity")?,
    )?;
    let witness: [u8; 32] = serde_json::from_value(intro["witness_public"].clone())?;
    let client = WitnessServiceClient::from_ready(
        input,
        output,
        witness,
        provisioned.writer_public,
        node,
        WitnessBootstrap::CreateNew,
    )?;
    let external = ExternalWitness::new(
        Box::new(client),
        witness,
        WitnessDurability::FileAndDirectory,
    )?;
    let configuration: Configuration = serde_json::from_value(config)?;
    drop(provisioned);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let host = ServiceHost::bind_with_witness(configuration, external).await?;
        let health = host.epoch_health();
        if !health.serving() {
            return Err("serving epoch failed closed before readiness".into());
        }
        // Readiness marker for the exec health probe. It exists only while the
        // runtime's own fail-closed latch reports serving; a marker that cannot
        // be withdrawn ends the process, so the probe never outlives the latch.
        fs::write(super::SERVING_MARKER, b"serving\n")?;
        tokio::spawn(async move {
            while health.serving() {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            eprintln!(
                "{}",
                json!({"schema":"humanymous.product-e2e-gate-epoch/1","serving":false})
            );
            if fs::remove_file(super::SERVING_MARKER).is_err() {
                std::process::exit(70);
            }
        });
        eprintln!(
            "{}",
            json!({"schema":"humanymous.product-e2e-gate-ready/1",
                "composition":"humanymous-runtime library ServiceHost::bind_with_witness in the lab broker supervisor",
                "readiness":serde_json::to_value(host.readiness())?})
        );
        let (_sender, receiver) = tokio::sync::watch::channel(None);
        host.run(receiver).await?;
        Err::<(), Box<dyn std::error::Error>>("witnessed gateway stopped".into())
    })
}
