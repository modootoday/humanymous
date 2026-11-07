#![allow(dead_code)]
use humanymous_authority::{configuration::Configuration, decision::NativePeer};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
#[cfg(unix)]
pub async fn connect(endpoint: &str) -> tokio::net::UnixStream {
    tokio::net::UnixStream::connect(endpoint).await.unwrap()
}
#[cfg(windows)]
pub async fn connect(
    endpoint: &str,
) -> interprocess::os::windows::named_pipe::tokio::DuplexPipeStream<
    interprocess::os::windows::named_pipe::pipe_mode::Bytes,
> {
    interprocess::os::windows::named_pipe::tokio::DuplexPipeStream::connect_by_path(endpoint)
        .await
        .unwrap()
}
pub struct Directory(pub PathBuf);
impl Directory {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "h-auth-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        #[cfg(windows)]
        {
            set_private_acl(&path, "");
        }
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[cfg(windows)]
pub fn sid() -> String {
    let sid = humanymous_native_security::process::current_identity()
        .unwrap()
        .sid()
        .to_owned();
    assert!(humanymous_authority::configuration::valid_sid(&sid));
    sid
}
#[cfg(windows)]
pub fn set_private_acl(path: &std::path::Path, extra_ace: &str) {
    use windows_permissions::{
        LocalBox, SecurityDescriptor, Sid,
        constants::{SeObjectType, SecurityInformation},
        wrappers,
    };
    let owner: LocalBox<Sid> = sid().parse().unwrap();
    let sd: LocalBox<SecurityDescriptor> = format!("D:P(A;OICI;FA;;;{}){extra_ace}", sid())
        .parse()
        .unwrap();
    wrappers::SetNamedSecurityInfo(
        path.as_os_str(),
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        Some(&owner),
        None,
        sd.dacl(),
        None,
    )
    .unwrap();
}

pub fn peer() -> NativePeer {
    #[cfg(unix)]
    {
        NativePeer::Unix(rustix::process::geteuid().as_raw())
    }
    #[cfg(windows)]
    {
        NativePeer::Windows(std::process::id())
    }
}
pub fn config_value(directory: &Directory) -> Value {
    #[cfg(unix)]
    let (endpoint, peer) = (
        directory
            .0
            .join("decision.sock")
            .to_string_lossy()
            .into_owned(),
        json!({"kind":"unix", "uid":rustix::process::geteuid().as_raw()}),
    );
    #[cfg(windows)]
    let (endpoint, peer) = (
        format!(
            r"\\.\pipe\h-auth-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ),
        json!({"kind":"windows", "sid":sid(), "pid":std::process::id()}),
    );
    json!({"endpoint":endpoint,"peer":peer,
        "scope":{"installation":"i","tenant":"t","origin":"o","revision":"r","authority":"example.test"},
        "routes":[{"method":"POST","path":"/api","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":false}},
            {"method":"POST","path":"/capture","required":{"raw_tls":true,"original_h2":false,"raw_header_order":false,"collector":false,"rit":false}}],
        "audit":{"directory":directory.0,"node_id":"authority-test","hmac_key_hex":"aa".repeat(32),"signing_seed_hex":"bb".repeat(32),"allow_new":true,"trusted_head":null},
        "capacity":4,"replay_capacity":8,"rate_per_second":100,"timeout_ms":1000,"clock_skew_ms":1000})
}
pub fn config(directory: &Directory) -> Configuration {
    serde_json::from_value(config_value(directory)).unwrap()
}
pub fn request(now: u64, id: u32) -> Value {
    json!({"protocol":"humanymous.adapter/0.1","id":format!("01234567-89ab-4cde-8fab-{id:012x}"),
        "tenant":"t","origin":"o","revision":"r","deadline":now+5000,"method":"POST","target":"/api?q=1","authority":"example.test","peer":"127.0.0.1",
        "headers":[["Host","example.test"],["Content-Length","3"],["User-Agent","Mozilla/5.0 Chrome/126.0 Safari/537.36"]],
        "headerMode":"node-raw","body":{"layer":"transfer-decoded-content-encoded","complete":true,"length":3,"sha256":hex::encode(Sha256::digest(b"abc")),"base64":"YWJj"},
        "capabilities":{"rawTls":false,"originalH2":false,"rawHeaderOrder":true,"completeBody":true},"stream":null,"capture":null,
        "connectionId":"01234567-89ab-4cde-8fab-0123456789ab","adapter":{"id":"humanymous.node","version":"0.1.0-experimental.1","installation":"i"}})
}
pub fn envelope(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
