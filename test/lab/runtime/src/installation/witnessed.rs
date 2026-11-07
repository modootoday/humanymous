//! One private witness/key installation factory for contextual product actors.
use super::Installation;
use humanymous_audit::{
    checkpoint::{
        WitnessBootstrap,
        service::{self, WitnessServiceClient},
    },
    log::WitnessDurability,
};
use humanymous_runtime::startup::ExternalWitness;
use serde_json::json;
use std::{
    io::{self, BufReader},
    path::Path,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub struct WitnessedInstallation {
    pub installation: Installation,
    pub external: ExternalWitness,
    pub writer_public: [u8; 32],
    pub witness_public: [u8; 32],
}
pub fn fresh(
    mut input: BufReader<io::Stdin>,
    configure: impl FnOnce(&mut Installation) -> Result<()>,
) -> Result<WitnessedInstallation> {
    let mut installation = Installation::new(false)?;
    let (custody, material) = crate::witness::material(
        Path::new("/writer/keys"),
        b"synthetic-owned-installation",
        true,
    )?;
    installation.seed = *material.signing_seed();
    installation.hmac.copy_from_slice(material.hmac_key());
    let writer_public = ed25519_dalek::SigningKey::from_bytes(&installation.seed)
        .verifying_key()
        .to_bytes();
    drop(material);
    drop(custody);
    installation.config["audit"]["directory"] = json!("/writer/wal");
    installation.config["audit"]["keys"]["directory"] = json!("/writer/keys");
    installation.config["audit"]["require_external_witness"] = json!(true);
    installation.config["audit"]["node_id"] = json!("isolated-node");
    configure(&mut installation)?;
    service::write_frame(
        &mut io::stdout(),
        &json!({"role":"writer_identity","writer_public":writer_public}),
    )?;
    let bytes = service::read_frame(&mut input)?.ok_or("missing witness readiness")?;
    let intro = humanymous_test_support::json::decode_strict_json(&bytes, 65536)?;
    let witness_public: [u8; 32] = serde_json::from_value(intro["witness_public"].clone())?;
    let client = WitnessServiceClient::from_ready(
        input,
        io::stdout(),
        witness_public,
        writer_public,
        "isolated-node".into(),
        WitnessBootstrap::CreateNew,
    )?;
    let external = ExternalWitness::new(
        Box::new(client),
        witness_public,
        WitnessDurability::FileAndDirectory,
    )?;
    Ok(WitnessedInstallation {
        installation,
        external,
        writer_public,
        witness_public,
    })
}
