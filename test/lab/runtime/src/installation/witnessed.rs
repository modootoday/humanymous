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
    eprintln!("HM-LAB-ACTOR-STEP actor_installation");
    let mut installation = Installation::new(false)?;
    eprintln!("HM-LAB-ACTOR-STEP actor_writer_mount");
    let mounted = crate::mount_policy::Facts::observe(Path::new("/writer/keys"));
    eprintln!(
        "HM-LAB-ACTOR-MOUNT {}",
        json!({"schema":"humanymous.writer-key-mount/1","lookup":format!("{:?}",mounted.lookup),"uid":mounted.uid,"gid":mounted.gid,"mode":mounted.mode.map(|v|v&0o777),"directory":mounted.directory,"symlink":mounted.symlink,"os_error":mounted.os_error})
    );
    if let Some(reason) = mounted.refusal(65532) {
        eprintln!("HM-LAB-ACTOR-STEP {}", reason.code());
        return Err("actual mounted writer key directory refused".into());
    }
    eprintln!("HM-LAB-ACTOR-STEP actor_writer_keys");
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
    eprintln!("HM-LAB-ACTOR-STEP actor_writer_identity");
    service::write_frame(
        &mut io::stdout(),
        &json!({"role":"writer_identity","writer_public":writer_public}),
    )?;
    eprintln!("HM-LAB-ACTOR-STEP actor_witness_bootstrap");
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
