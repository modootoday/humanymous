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
            let script = r#"$ErrorActionPreference='Stop'; $s=[System.Security.Principal.WindowsIdentity]::GetCurrent().User; $a=New-Object System.Security.AccessControl.DirectorySecurity; $a.SetOwner($s); $a.SetAccessRuleProtection($true,$false); $r=New-Object System.Security.AccessControl.FileSystemAccessRule($s,'FullControl','ContainerInherit,ObjectInherit','None','Allow'); $a.AddAccessRule($r); [System.IO.Directory]::SetAccessControl($env:HUMANYMOUS_TEST_PATH,$a)"#;
            assert!(
                std::process::Command::new("powershell.exe")
                    .args(["-NoProfile", "-NonInteractive", "-Command", script])
                    .env("HUMANYMOUS_TEST_PATH", &path)
                    .status()
                    .unwrap()
                    .success()
            );
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
    let result = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value",
        ])
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().trim().into()
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
