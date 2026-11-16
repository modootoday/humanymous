//! Independent audit witness container. It provisions its own sealed identity
//! and head state on its own tmpfs, under its own UID, with no network. Its
//! unlock passphrase is generated here and never leaves this process. The
//! trusted runner broker relays typed frames between this process and the Gate.
use ed25519_dalek::SigningKey;
use humanymous_runtime::audit::checkpoint::service::{
    self, WitnessServiceBootstrap, WitnessServiceConfig, read_frame, write_frame,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, BufReader, Cursor, Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Health marker on this container's private tmpfs, written only after the
/// audited witness service flushed its typed Ready frame.
pub const READY_MARKER: &str = "/tmp/product-e2e-witness.ready";

fn private_directory(path: &Path) -> io::Result<()> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

/// Marks readiness after the first flushed service frame (the Ready reply).
struct ReadyMarker<W: Write> {
    inner: W,
    marker: PathBuf,
    marked: bool,
}

impl<W: Write> Write for ReadyMarker<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.inner.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()?;
        if !self.marked {
            fs::write(&self.marker, b"ready\n")?;
            self.marked = true;
        }
        Ok(())
    }
}

/// Provision, announce the public identity, then serve the audited protocol.
pub fn run() -> Result<()> {
    let mut input = BufReader::new(io::stdin());
    let intro: Value = serde_json::from_slice(
        &read_frame(&mut input)?.ok_or("broker did not supply writer identity")?,
    )?;
    let writer_public: [u8; 32] = serde_json::from_value(intro["writer_public"].clone())?;
    let node_id = intro["node_id"]
        .as_str()
        .ok_or("writer node identity")?
        .to_owned();
    let mut nonce = [0; 8];
    let mut passphrase = Zeroizing::new(vec![0; 32]);
    getrandom::fill(&mut nonce)?;
    getrandom::fill(&mut passphrase[..])?;
    let suffix: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let root = std::env::temp_dir().join(format!("product-e2e-witness-{suffix}"));
    private_directory(&root)?;
    let root = root.canonicalize()?;
    let (keys, head) = (root.join("keys"), root.join("head"));
    private_directory(&keys)?;
    private_directory(&head)?;
    let (custody, material) = crate::witness::material(&keys, &passphrase, true)?;
    let witness_public = SigningKey::from_bytes(material.signing_seed())
        .verifying_key()
        .to_bytes();
    drop(material);
    drop(custody);
    write_frame(
        &mut io::stdout(),
        &json!({"role":"witness_identity","witness_public":witness_public}),
    )?;
    let config = WitnessServiceConfig {
        key_directory: keys,
        state_directory: head,
        owner_sid: None,
        passphrase,
        writer_public,
        node_id,
        bootstrap: WitnessServiceBootstrap::CreateNew,
    };
    let mut prefix = Zeroizing::new(serde_json::to_vec(&config)?);
    prefix.push(b'\n');
    drop(config);
    // The audited service reopens custody itself; this actor adds no framing.
    service::serve(
        BufReader::new(Cursor::new(prefix).chain(input)),
        ReadyMarker {
            inner: io::stdout(),
            marker: PathBuf::from(READY_MARKER),
            marked: false,
        },
    )?;
    Ok(())
}
