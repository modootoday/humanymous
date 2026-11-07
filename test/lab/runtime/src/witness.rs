//! Independent-container audit witness roles. Private framing belongs to audit.
use ed25519_dalek::SigningKey;
use humanymous_audit::{
    checkpoint::{
        Checkpoint, WitnessBootstrap,
        service::{self, WitnessServiceClient, WitnessServiceConfig},
    },
    keys::{
        KeyError, KeyMaterial,
        custody::{Custody, Owner},
    },
    log::{Config, Log, WitnessDurability},
    record::Record,
    storage::{Wal, WalOptions},
};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    io::{self, BufReader, Cursor, Read},
    path::Path,
};
use zeroize::Zeroizing;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transaction {
    create_identity: bool,
    config: WitnessServiceConfig,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WriterConfig {
    create_identity: bool,
    passphrase: Zeroizing<Vec<u8>>,
    trusted_head: Option<Checkpoint>,
}
fn material(directory: &Path, passphrase: &[u8], create: bool) -> Result<(Custody, KeyMaterial)> {
    if create && (directory.join("keys.sealed").exists() || directory.join("keys.pending").exists())
    {
        return Err("explicit creation cannot replace existing identity".into());
    }
    let mut custody = Custody::acquire(directory, Owner::CurrentUser)?;
    let (material, _) = custody.load_or_create(passphrase, || {
        if !create {
            return Err(KeyError::InvalidMaterial);
        }
        let mut seed = [0; 32];
        let mut hmac = vec![0; 32];
        getrandom::fill(&mut seed).map_err(|_| KeyError::InvalidMaterial)?;
        getrandom::fill(&mut hmac).map_err(|_| KeyError::InvalidMaterial)?;
        KeyMaterial::new(seed, hmac, None, None)
    })?;
    Ok((custody, material))
}
pub fn transaction() -> Result<()> {
    let mut input = BufReader::new(io::stdin());
    let bytes =
        Zeroizing::new(service::read_frame(&mut input)?.ok_or("missing private bootstrap")?);
    let transaction: Transaction = serde_json::from_slice(&bytes)?;
    if transaction.config.key_directory != Path::new("/witness/keys")
        || transaction.config.state_directory != Path::new("/witness/head")
        || transaction.config.owner_sid.is_some()
    {
        return Err("unexpected witness mounts/owner".into());
    }
    let (custody, material) = material(
        &transaction.config.key_directory,
        &transaction.config.passphrase,
        transaction.create_identity,
    )?;
    let public = SigningKey::from_bytes(material.signing_seed())
        .verifying_key()
        .to_bytes();
    service::write_frame(
        &mut io::stdout(),
        &json!({"role":"witness_identity","witness_public":public}),
    )?;
    drop(material);
    drop(custody);
    let mut prefix = Zeroizing::new(serde_json::to_vec(&transaction.config)?);
    prefix.push(b'\n');
    // The audited service reopens custody and verifies external pinning itself.
    service::serve(
        BufReader::new(Cursor::new(prefix).chain(input)),
        io::stdout(),
    )?;
    Ok(())
}
pub fn writer() -> Result<()> {
    let mut input = BufReader::new(io::stdin());
    let bytes = Zeroizing::new(service::read_frame(&mut input)?.ok_or("missing writer bootstrap")?);
    let config: WriterConfig = serde_json::from_slice(&bytes)?;
    let (_custody, material) = material(
        Path::new("/writer/keys"),
        &config.passphrase,
        config.create_identity,
    )?;
    let public = SigningKey::from_bytes(material.signing_seed())
        .verifying_key()
        .to_bytes();
    service::write_frame(
        &mut io::stdout(),
        &json!({"role":"writer_identity","writer_public":public}),
    )?;
    let intro: serde_json::Value = serde_json::from_slice(
        &service::read_frame(&mut input)?.ok_or("missing broker identity")?,
    )?;
    let witness: [u8; 32] = serde_json::from_value(intro["witness_public"].clone())?;
    let bootstrap = match config.trusted_head.as_ref() {
        Some(head) => WitnessBootstrap::Pinned {
            minimum: head,
            consistency_proof: &[],
        },
        None => WitnessBootstrap::CreateNew,
    };
    let client = WitnessServiceClient::from_ready(
        input,
        io::stdout(),
        witness,
        public,
        "isolated-node".into(),
        bootstrap,
    )?;
    let mut log = Log::open_with_external_witness(
        Wal::open(Path::new("/writer/wal"), WalOptions::default())?,
        Config {
            node_id: "isolated-node".into(),
            hmac_key: material.hmac_key().to_vec(),
            signing_seed: *material.signing_seed(),
            checkpoint_every: 1,
            witness: None,
        },
        config.trusted_head.as_ref(),
        config.create_identity,
        Box::new(client),
        WitnessDurability::FileAndDirectory,
    )?;
    let origin = crate::origin::Origin::start();
    let result=log.emit_and_act(Record{event_type:"isolated.effect".into(),verdict:"ALLOW".into(),..Default::default()},|_|crate::client::exchange(origin.address,b"POST /effect HTTP/1.1\r\nHost: owned.test\r\nContent-Length: 6\r\nConnection: close\r\n\r\neffect"));
    let status = if result.is_ok() { "passed" } else { "closed" };
    let checkpoint = log.checkpoints().ok().and_then(|mut heads| heads.pop());
    service::write_frame(
        &mut io::stdout(),
        &json!({"role":"writer_result","status":status,"origin_effects":origin.effects(),"audit_sequence":log.sequence(),"checkpoint":checkpoint}),
    )?;
    Ok(())
}
pub fn key_denied() -> Result<()> {
    let result = fs::read("/witness/keys/keys.sealed");
    if !matches!(result,Err(error) if error.kind()==io::ErrorKind::PermissionDenied) {
        return Err("witness key was not denied by OS permissions".into());
    }
    println!(
        "{}",
        json!({"status":"passed","assertion":"different-uid-witness-key-eacces"})
    );
    Ok(())
}
