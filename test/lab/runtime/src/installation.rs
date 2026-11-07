//! One actual custody/config/origin owner, reused by all contextual scenarios.
pub(crate) mod witnessed;
use crate::{origin::Origin, provisioning};
use humanymous_runtime::audit::{
    keys::{
        KeyMaterial,
        custody::{Custody, Owner},
    },
    log::{Config, Log},
    storage::{Wal, WalOptions},
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

pub struct Installation {
    pub origin: Origin,
    pub config: Value,
    pub directory: PathBuf,
    pub key_directory: PathBuf,
    pub hmac: [u8; 32],
    pub seed: [u8; 32],
}
impl Installation {
    pub fn new(cli: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let mut seed = [0; 32];
        let mut hmac = [0; 32];
        let mut nonce = [0; 8];
        getrandom::fill(&mut seed)?;
        getrandom::fill(&mut hmac)?;
        getrandom::fill(&mut nonce)?;
        let name = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let directory = std::env::temp_dir().join(format!("runtime-pilot-{name}"));
        fs::create_dir(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        let directory = directory.canonicalize()?;
        if cli {
            provisioning::run(&directory);
        }
        let key_directory = directory.join("custody");
        fs::create_dir(&key_directory)?;
        fs::set_permissions(&key_directory, fs::Permissions::from_mode(0o700))?;
        let mut custody = Custody::acquire(&key_directory, Owner::CurrentUser)?;
        custody.load_or_create(b"synthetic-owned-installation", || {
            KeyMaterial::new(seed, hmac.to_vec(), None, None)
        })?;
        drop(custody);
        let origin = Origin::start();
        let required = |raw_tls, original_h2, collector| json!({"raw_tls":raw_tls,"original_h2":original_h2,"raw_header_order":true,"collector":collector,"rit":true});
        let config = json!({"listen":"127.0.0.1:0","origin":origin.address.to_string(),
          "scope":{"installation":"pilot","tenant":"tenant","origin":"origin","revision":"r1","authority":"owned.test"},
          "routes":[{"method":"POST","path":"/effect","required":required(false,false,false)},
                    {"method":"POST","path":"/tls","required":required(true,false,false)},
                    {"method":"POST","path":"/h2","required":required(false,true,false)},
                    {"method":"POST","path":"/collector","required":required(false,false,true)}],
          "audit":{"directory":directory,"node_id":"process-pilot","keys":{"directory":key_directory,"passphrase":"synthetic-owned-installation"},"allow_new":true,"trusted_head":null},
          "owner":{"kind":"unix","uid":65532},"session_capacity":16,"session_ttl_ms":60000,"timeout_ms":5000,"rate_per_second":1000});
        Ok(Self {
            origin,
            config,
            directory,
            key_directory,
            hmac,
            seed,
        })
    }
    pub fn open_log(&self) -> std::io::Result<Log> {
        Log::open(
            Wal::open(&self.directory, WalOptions::default())?,
            Config {
                node_id: "process-pilot".into(),
                hmac_key: self.hmac.to_vec(),
                signing_seed: self.seed,
                checkpoint_every: 1,
                witness: None,
            },
            None,
            true,
        )
    }
}
impl Drop for Installation {
    fn drop(&mut self) {
        if self.directory.parent() == std::env::temp_dir().canonicalize().ok().as_deref()
            && self
                .directory
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("runtime-pilot-"))
        {
            let _ = fs::remove_dir_all(&self.directory);
        }
    }
}
