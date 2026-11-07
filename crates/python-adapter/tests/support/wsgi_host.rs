//! Owned Unix WSGI host lifecycle and bounded HTTP test transport.
use super::http;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(super) struct Directory(pub(super) PathBuf);
impl Directory {
    pub(super) fn new() -> Self {
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
use std::os::unix::fs::DirBuilderExt;
pub(super) struct Host(pub(super) Child);
impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
pub(super) fn executable(variable: &str) -> PathBuf {
    let path =
        PathBuf::from(std::env::var_os(variable).expect("provisioned actual executable required"));
    assert!(
        path.is_absolute() && path.is_file(),
        "absolute provisioned executable required"
    );
    path
}
pub(super) fn configuration(directory: &Directory, node: &str) -> Value {
    json!({"adapter":"python-wsgi","endpoint":directory.0.join("authority.sock"),
        "peer":{"kind":"unix","uid":humanymous_native_security::process::current_uid()},
        "scope":{"installation":"i","tenant":"t","origin":"o","revision":"r","authority":"example.test"},
        "routes":[{"method":"POST","path":"/api","required":{"raw_tls":false,"original_h2":false,"raw_header_order":false,"collector":false,"rit":false}}],
        "audit":{"directory":directory.0,"node_id":node,"hmac_key_hex":"aa".repeat(32),"signing_seed_hex":"bb".repeat(32),"allow_new":true,"trusted_head":null},
        "capacity":8,"replay_capacity":32,"rate_per_second":100,"timeout_ms":2000,"clock_skew_ms":1000})
}
pub(super) fn host(
    directory: &Directory,
    framework: &str,
    endpoint: &str,
    credential: &str,
) -> (Host, u16) {
    let port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let module = PathBuf::from(
        std::env::var_os("HUMANYMOUS_PYTHON_MODULE_DIR")
            .expect("actual host-pilot extension directory required"),
    );
    assert!(module.is_absolute() && module.is_dir());
    let mut child = Host(
        Command::new(executable("HUMANYMOUS_WAITRESS_SERVE"))
            .args([
                "--host=127.0.0.1",
                &format!("--port={port}"),
                "--channel-request-lookahead=1",
                "--channel-timeout=2",
                "--cleanup-interval=1",
                "--max-request-body-size=2048",
                "--call",
                "humanymous_python:pilot_wsgi",
            ])
            .env("PYTHONPATH", module)
            .env("PYTHONNOUSERSITE", "1")
            .env("HUMANYMOUS_HOSTED_NATIVE_PILOT", "1")
            .env_remove("PYTHONSTARTUP")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(fs::File::create(directory.0.join("server-stderr.private.log")).unwrap())
            .spawn()
            .expect("actual Waitress process required"),
    );
    let options = json!({"installation":"i","tenant":"t","origin":"o","revision":"r", "endpoint":endpoint,"credential":credential,
        "server":{"kind":"unix","uid":humanymous_native_security::process::current_uid()},"routes":["/api"],"timeoutMs":1000,"maxBody":1024,"syncInputDeadlineEnforced":true});
    child.0.stdin.take().unwrap().write_all(&serde_json::to_vec(&json!({"framework":framework,"configuration":options.to_string(),"effect_file":directory.0.join("effects.jsonl")})).unwrap()).unwrap();
    let began = Instant::now();
    loop {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "actual WSGI server exited before readiness; inspect private log"
        );
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        assert!(
            began.elapsed() < Duration::from_secs(15),
            "actual WSGI server readiness deadline"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    (child, port)
}
pub(super) fn request(port: u16, target: &str, body: &[u8], extra: &str) -> Vec<u8> {
    let mut socket = TcpStream::connect(("127.0.0.1", port)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    write!(socket, "POST {target} HTTP/1.1\r\nHost: example.test\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\n{extra}Connection: close\r\n\r\n", body.len()).unwrap();
    socket.write_all(body).unwrap();
    http::receive(&mut socket).unwrap()
}
pub(super) fn stop(mut host: Host, port: u16) {
    host.0.kill().unwrap();
    host.0.wait().unwrap();
    // Do not confuse accepted sockets in TIME_WAIT with a live listener.
    assert!(
        TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_millis(200))
            .is_err(),
        "owned server listener remained"
    );
}
