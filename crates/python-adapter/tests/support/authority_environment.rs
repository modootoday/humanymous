//! Existing private Unix component Authority configuration and directory owner.
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) struct Directory(pub(crate) PathBuf);
impl Directory {
    pub(crate) fn new() -> Self {
        assert_eq!(
            std::env::var("HUMANYMOUS_HOSTED_NATIVE_PILOT").as_deref(),
            Ok("1")
        );
        let path = std::env::temp_dir().join(format!(
            "hpy-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub(crate) fn configuration(directory: &Directory, node: &str) -> Value {
    json!({"adapter":"python-wsgi","endpoint":directory.0.join("authority.sock"),
        "peer":{"kind":"unix","uid":humanymous_native_security::process::current_uid()},
        "scope":{"installation":"i","tenant":"t","origin":"o","revision":"r","authority":"example.test"},
        "routes":[{"method":"POST","path":"/api","required":{"raw_tls":false,"original_h2":false,"raw_header_order":false,"collector":false,"rit":false}}],
        "audit":{"directory":directory.0,"node_id":node,"hmac_key_hex":"aa".repeat(32),"signing_seed_hex":"bb".repeat(32),"allow_new":true,"trusted_head":null},
        "capacity":8,"replay_capacity":32,"rate_per_second":100,"timeout_ms":2000,"clock_skew_ms":1000})
}
