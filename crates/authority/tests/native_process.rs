//! Actual child-process IPC security pilots; separate from Docker product e2e.
mod support;
use humanymous_authority::{now_ms, protocol};
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    time::Duration,
};
use support::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Process {
    child: Child,
    endpoint: String,
    credential: String,
}
impl Process {
    fn start(config: &Value) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_humanymous-authority"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(config).unwrap())
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line);
            let _ = tx.send((result, line));
        });
        let (_, line) = rx
            .recv_timeout(Duration::from_secs(15))
            .unwrap_or_else(|_| {
                let _ = child.kill();
                panic!("authority readiness timed out");
            });
        if line.is_empty() {
            let _ = child.kill();
            let mut error = String::new();
            std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut error).unwrap();
            panic!("authority startup rejected: {error}");
        }
        let ready: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(ready["qualification"], "experimental-local-authority");
        assert_eq!(ready["profile"]["id"], "application-http");
        assert_eq!(ready["profile"]["raw_tls"], false);
        assert_eq!(ready["profile"]["routes"][0]["node_raw"], true);
        assert_eq!(ready["profile"]["routes"][0]["fetch_normalized"], false);
        assert_eq!(ready["profile"]["routes"][1]["node_raw"], false);
        Self {
            child,
            endpoint: ready["endpoint"].as_str().unwrap().into(),
            credential: ready["credential"].as_str().unwrap().into(),
        }
    }
    async fn send(&self, credential: &str, request: &Value) -> (u16, Value) {
        let body = envelope(request);
        let header = format!(
            "POST /adapter/v0.1/decision HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {credential}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
            body.len()
        );
        let wire = [header.as_bytes(), &body].concat();
        let bytes = self.exchange(&wire).await;
        let text = std::str::from_utf8(&bytes).unwrap();
        let (head, body) = text.split_once("\r\n\r\n").unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        assert!(head.contains("Connection: close"));
        assert!(head.contains("Cache-Control: no-store"));
        (status, serde_json::from_str(body).unwrap())
    }
    async fn exchange(&self, wire: &[u8]) -> Vec<u8> {
        let mut stream = connect(&self.endpoint).await;
        tokio::time::timeout(Duration::from_secs(5), async {
            stream.write_all(wire).await.unwrap();
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).await.unwrap();
            bytes
        })
        .await
        .expect("bounded response")
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(&self.endpoint);
        }
    }
}
#[cfg(unix)]
async fn connect(endpoint: &str) -> tokio::net::UnixStream {
    tokio::net::UnixStream::connect(endpoint).await.unwrap()
}
#[cfg(windows)]
async fn connect(
    endpoint: &str,
) -> interprocess::os::windows::named_pipe::tokio::DuplexPipeStream<
    interprocess::os::windows::named_pipe::pipe_mode::Bytes,
> {
    interprocess::os::windows::named_pipe::tokio::DuplexPipeStream::connect_by_path(endpoint)
        .await
        .unwrap()
}

#[tokio::test]
async fn real_ipc_auth_replay_body_scope_capture_and_restart_epoch() {
    let d = Directory::new();
    let c = config_value(&d);
    let p = Process::start(&c);
    let request = request(now_ms().unwrap(), 1);
    for _ in 0..3 {
        drop(connect(&p.endpoint).await);
    }
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(p.send("wrong", &request).await.0, 401);
    let (status, response) = p.send(&p.credential, &request).await;
    assert_eq!(status, 200);
    assert_ne!(response["verdict"], "ALLOW");
    assert_eq!(response.as_object().unwrap().len(), 7);
    assert_eq!(response["id"], request["id"]);
    assert_eq!(response["tenant"], "t");
    assert_eq!(p.send(&p.credential, &request).await.0, 409);
    let mut body = request.clone();
    body["id"] = support::request(now_ms().unwrap(), 2)["id"].clone();
    body["body"]["base64"] = "YWJk".into();
    assert_eq!(p.send(&p.credential, &body).await.0, 422);
    let mut tenant = request.clone();
    tenant["tenant"] = "other".into();
    assert_eq!(p.send(&p.credential, &tenant).await.0, 403);
    let mut capture = request.clone();
    capture["target"] = "/capture".into();
    assert_eq!(p.send(&p.credential, &capture).await.0, 422);
    assert_eq!(
        p.exchange(b"POST /admin/erase HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n")
            .await[..12],
        *b"HTTP/1.1 403"
    );
    let old = p.credential.clone();
    drop(p);
    let p = Process::start(&c);
    assert_ne!(old, p.credential);
    let request = support::request(now_ms().unwrap(), 1);
    assert_eq!(p.send(&old, &request).await.0, 401);
    assert_eq!(p.send(&p.credential, &request).await.0, 200);
}
#[tokio::test]
async fn concurrent_nonce_reservation_and_durable_failure() {
    let d = Directory::new();
    let p = Process::start(&config_value(&d));
    let v = request(now_ms().unwrap(), 1);
    let (a, b) = tokio::join!(p.send(&p.credential, &v), p.send(&p.credential, &v));
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    drop(p);
    let d = Directory::new();
    let p = Process::start(&config_value(&d));
    std::fs::create_dir(d.0.join("checkpoints.jsonl")).unwrap();
    assert_eq!(
        p.send(&p.credential, &request(now_ms().unwrap(), 1))
            .await
            .0,
        503
    );
    assert_eq!(
        p.send(&p.credential, &request(now_ms().unwrap(), 2))
            .await
            .0,
        503
    );
}
#[tokio::test]
async fn native_capacity_and_whole_connection_timeout() {
    let d = Directory::new();
    let mut c = config_value(&d);
    c["capacity"] = 1.into();
    c["timeout_ms"] = 500.into();
    let p = Process::start(&c);
    let mut slow = connect(&p.endpoint).await;
    slow.write_all(b"POST ").await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        p.send(&p.credential, &request(now_ms().unwrap(), 1))
            .await
            .0,
        429
    );
    let mut closed = Vec::new();
    let result = tokio::time::timeout(Duration::from_secs(3), slow.read_to_end(&mut closed)).await;
    assert!(result.is_ok());
    assert!(closed.is_empty());
    assert_eq!(
        p.send(&p.credential, &request(now_ms().unwrap(), 2))
            .await
            .0,
        200
    );
}
#[cfg(windows)]
#[tokio::test]
async fn actual_windows_client_pid_is_required() {
    let d = Directory::new();
    let mut c = config_value(&d);
    c["peer"]["pid"] = (std::process::id() + 1).into();
    let p = Process::start(&c);
    assert_eq!(
        p.send(&p.credential, &request(now_ms().unwrap(), 1))
            .await
            .0,
        403
    );
}
#[test]
fn private_directory_and_endpoint_configuration_fail_closed() {
    let d = Directory::new();
    let mut c = config(&d);
    c.endpoint = "tcp://127.0.0.1:8080".into();
    // Listener construction runs inside a runtime on both supported platforms.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();
    assert!(humanymous_authority::transport::Listener::bind(&c).is_err());
    #[cfg(windows)]
    {
        c.endpoint = r"\\remote\pipe\authority".into();
        assert!(humanymous_authority::transport::Listener::bind(&c).is_err());
        c.endpoint = r"\\.\pipe\authority\child".into();
        assert!(humanymous_authority::transport::Listener::bind(&c).is_err());
    }
    c.routes[0].path = "/_humanymous/admin".into();
    assert!(c.validate().is_err());
    let d2 = Directory::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&d2.0, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    #[cfg(windows)]
    {
        set_private_acl(&d2.0, "(A;;FR;;;WD)");
    }
    assert!(humanymous_authority::transport::private_directory(&d2.0, &c.peer).is_err());
    assert_eq!(protocol::MAX_BODY, 1_600_000);
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "explicit privileged Docker-only multi-user qualification"]
fn unix_other_user_is_denied_by_actual_socket_acl() {
    assert_eq!(
        rustix::process::geteuid().as_raw(),
        0,
        "requires privileged test UID, never silently skip"
    );
    let d = Directory::new();
    let c = config_value(&d);
    let p = Process::start(&c);
    let child = Command::new("setpriv")
        .args(["--reuid", "65534", "--regid", "65534", "--clear-groups"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", "unix_unprivileged_connect_probe", "--ignored"])
        .env("HUMANYMOUS_SOCKET_PROBE", &p.endpoint)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "actual other-user connection must fail: {}",
        String::from_utf8_lossy(&child.stderr)
    );
    let receipt = String::from_utf8_lossy(&child.stdout);
    assert!(
        receipt
            .lines()
            .any(|line| line == "test unix_unprivileged_connect_probe ... ok"),
        "required other-user helper did not execute"
    );
    assert!(
        receipt.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "empty/skipped helper cannot qualify socket denial"
    );
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "helper only for explicit Docker multi-user parent test"]
fn unix_unprivileged_connect_probe() {
    assert_eq!(rustix::process::geteuid().as_raw(), 65534);
    let path = std::env::var("HUMANYMOUS_SOCKET_PROBE").expect("parent supplies owned socket path");
    let result = std::os::unix::net::UnixStream::connect(path);
    assert_eq!(
        result
            .expect_err("unprivileged peer must not connect")
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
}
