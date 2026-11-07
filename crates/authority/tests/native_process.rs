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
        let child = Command::new(env!("CARGO_BIN_EXE_humanymous-authority"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // Own the child before any fallible provisioning/readiness operation so
        // unwinding always kills and reaps it through Process::drop.
        let mut process = Self {
            child,
            endpoint: String::new(),
            credential: String::new(),
        };
        process
            .child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(config).unwrap())
            .unwrap();
        let stdout = process.child.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line);
            let _ = tx.send((result, line));
        });
        let (_, line) = rx
            .recv_timeout(Duration::from_secs(15))
            .unwrap_or_else(|_| {
                panic!("authority readiness timed out");
            });
        if line.is_empty() {
            let _ = process.child.kill();
            let mut error = String::new();
            std::io::Read::read_to_string(&mut process.child.stderr.take().unwrap(), &mut error)
                .unwrap();
            panic!("authority startup rejected: {error}");
        }
        let ready: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(ready["qualification"], "experimental-local-authority");
        assert_eq!(ready["profile"]["id"], "application-http");
        assert_eq!(ready["profile"]["raw_tls"], false);
        let provisioned: humanymous_authority::configuration::Configuration =
            serde_json::from_value(config.clone()).unwrap();
        assert_eq!(ready["profile"], provisioned.capability_manifest());
        process.endpoint = ready["endpoint"].as_str().unwrap().into();
        process.credential = ready["credential"].as_str().unwrap().into();
        process
    }
    async fn send(&self, credential: &str, request: &Value) -> (u16, Value) {
        let body = envelope(request);
        let header = format!(
            "POST /adapter/v0.1/decision HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {credential}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
            body.len()
        );
        let wire = [header.as_bytes(), &body].concat();
        let response = self.exchange(&wire).await;
        assert!(
            response
                .headers
                .iter()
                .any(|(n, v)| n.eq_ignore_ascii_case("connection") && v == "close")
        );
        assert!(
            response
                .headers
                .iter()
                .any(|(n, v)| n.eq_ignore_ascii_case("cache-control") && v == "no-store")
        );
        (
            response.status,
            serde_json::from_slice(&response.body).unwrap(),
        )
    }
    async fn exchange(&self, wire: &[u8]) -> protocol::Response {
        let mut stream = connect(&self.endpoint).await;
        tokio::time::timeout(Duration::from_secs(5), async {
            stream.write_all(wire).await.unwrap();
            let response = protocol::read_response(&mut stream).await.unwrap();
            #[cfg(windows)]
            stream.assume_flushed();
            drop(stream);
            tokio::time::sleep(Duration::from_millis(10)).await;
            response
        })
        .await
        .expect("bounded response")
    }
}

fn server_peer(p: &Process, c: &Value) -> humanymous_authority::configuration::Peer {
    let mut peer: humanymous_authority::configuration::Peer =
        serde_json::from_value(c["peer"].clone()).unwrap();
    if let humanymous_authority::configuration::Peer::Windows { pid, .. } = &mut peer {
        *pid = p.child.id();
    }
    peer
}
async fn client_call(
    p: &Process,
    server: &humanymous_authority::configuration::Peer,
    v: &Value,
    profile: humanymous_integrity::adapter::AdapterProfile,
) -> Result<humanymous_authority::client::BoundResponse, humanymous_authority::Rejection> {
    use humanymous_authority::client::{ClientOptions, ExpectedBinding, decide};
    let options = ClientOptions {
        endpoint: &p.endpoint,
        credential: &p.credential,
        server,
        timeout_ms: 2000,
        adapter: profile,
    };
    let expected = ExpectedBinding {
        id: v["id"].as_str().unwrap(),
        tenant: v["tenant"].as_str().unwrap(),
        origin: v["origin"].as_str().unwrap(),
        revision: v["revision"].as_str().unwrap(),
        deadline: v["deadline"].as_u64().unwrap(),
    };
    decide(&options, &envelope(v), expected).await
}
#[tokio::test]
async fn shared_client_checks_real_server_and_profile_bound_credentials() {
    use humanymous_authority::client::Verdict;
    use humanymous_integrity::adapter::AdapterProfile;
    let d = Directory::new();
    let c = config_value(&d);
    let p = Process::start(&c);
    let server = server_peer(&p, &c);
    let v = request(now_ms().unwrap(), 31);
    let bound = client_call(&p, &server, &v, AdapterProfile::Node)
        .await
        .expect("real verified server response");
    assert_ne!(bound.consume().unwrap(), Verdict::Allow);
    assert!(
        client_call(&p, &server, &v, AdapterProfile::Node)
            .await
            .is_err()
    );
    let mut wrong = server_peer(&p, &c);
    match &mut wrong {
        humanymous_authority::configuration::Peer::Unix { uid } => *uid = uid.wrapping_add(1),
        humanymous_authority::configuration::Peer::Windows { pid, .. } => {
            *pid = pid.wrapping_add(1)
        }
    }
    assert!(
        client_call(
            &p,
            &wrong,
            &request(now_ms().unwrap(), 32),
            AdapterProfile::Node
        )
        .await
        .is_err()
    );
    let mut nginx = request(now_ms().unwrap(), 33);
    nginx["adapter"]["id"] = "humanymous.nginx".into();
    nginx["headerMode"] = "nginx-raw".into();
    nginx["connectionId"] = Value::Null;
    assert!(
        client_call(&p, &server, &nginx, AdapterProfile::Nginx)
            .await
            .is_err()
    );
    drop(p);
    let mut c = c;
    c["adapter"] = "nginx".into();
    let p = Process::start(&c);
    let server = server_peer(&p, &c);
    nginx["deadline"] = (now_ms().unwrap() + 5000).into();
    assert_ne!(
        client_call(&p, &server, &nginx, AdapterProfile::Nginx)
            .await
            .unwrap()
            .consume()
            .unwrap(),
        Verdict::Allow
    );
}

#[tokio::test]
async fn normalized_producers_are_bound_by_native_authority_credentials() {
    use humanymous_integrity::adapter::AdapterProfile;
    for (name, profile, mode, wrong_mode) in [
        (
            "python-asgi",
            AdapterProfile::PythonAsgi,
            "asgi-normalized",
            "wsgi-normalized",
        ),
        (
            "python-wsgi",
            AdapterProfile::PythonWsgi,
            "wsgi-normalized",
            "asgi-normalized",
        ),
        (
            "ruby-rack",
            AdapterProfile::RubyRack,
            "rack-normalized",
            "asgi-normalized",
        ),
    ] {
        let d = Directory::new();
        let mut c = config_value(&d);
        c["adapter"] = name.into();
        c["routes"][0]["required"]["raw_header_order"] = false.into();
        let p = Process::start(&c);
        let server = server_peer(&p, &c);
        let mut v = request(now_ms().unwrap(), 100);
        v["adapter"]["id"] = profile.expected().id.into();
        v["headerMode"] = mode.into();
        v["capabilities"]["rawHeaderOrder"] = false.into();
        v["peer"] = Value::Null;
        v["connectionId"] = Value::Null;
        let bound = client_call(&p, &server, &v, profile)
            .await
            .expect("profile-bound native reply");
        bound.consume().expect("live one-use native decision");
        assert!(
            client_call(&p, &server, &v, profile).await.is_err(),
            "replay refused"
        );
        v["id"] = "01234567-89ab-4cde-8fab-000000000101".into();
        v["headerMode"] = wrong_mode.into();
        assert_eq!(
            p.send(&p.credential, &v).await.0,
            400,
            "wire mode cannot select profile"
        );
        v["headerMode"] = mode.into();
        v["capture"] = serde_json::json!({"forged":true});
        assert_ne!(
            p.send(&p.credential, &v).await.0,
            200,
            "forged evidence refused"
        );
        v["capture"] = Value::Null;
        v["target"] = "/capture".into();
        assert_eq!(
            p.send(&p.credential, &v).await.0,
            422,
            "missing raw plane refused"
        );
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
    let admin = format!(
        "POST /admin/erase HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{{}}",
        p.credential
    );
    assert_eq!(p.exchange(admin.as_bytes()).await.status, 403);
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

#[cfg(windows)]
#[tokio::test]
async fn untrusted_pipe_dacl_is_rejected_before_credential_disclosure() {
    use humanymous_authority::{
        client::{ClientOptions, ExpectedBinding, decide},
        configuration::Peer,
    };
    use interprocess::os::windows::{
        named_pipe::{PipeListenerOptions, pipe_mode},
        security_descriptor::SecurityDescriptor,
    };
    let d = Directory::new();
    let c = config_value(&d);
    let endpoint = c["endpoint"].as_str().unwrap();
    let owner = sid();
    let sddl = widestring::U16CString::from_str(format!(
        "O:{owner}G:{owner}D:P(A;;GA;;;{owner})(A;;GRGW;;;WD)"
    ))
    .unwrap();
    let listener = PipeListenerOptions::new()
        .path(endpoint)
        .accept_remote(false)
        .security_descriptor(Some(SecurityDescriptor::deserialize(&sddl).unwrap()))
        .create_tokio_duplex::<pipe_mode::Bytes>()
        .unwrap();
    let observed = tokio::spawn(async move {
        let mut stream = listener.accept().await.unwrap();
        let mut byte = [0];
        let got = stream.read(&mut byte).await.unwrap();
        stream.assume_flushed();
        got
    });
    let v = request(now_ms().unwrap(), 71);
    let server = Peer::Windows {
        sid: owner,
        pid: std::process::id(),
    };
    let token = "A".repeat(64);
    let options = ClientOptions {
        endpoint,
        credential: &token,
        server: &server,
        timeout_ms: 1000,
        adapter: humanymous_integrity::adapter::AdapterProfile::Node,
    };
    let expected = ExpectedBinding {
        id: v["id"].as_str().unwrap(),
        tenant: "t",
        origin: "o",
        revision: "r",
        deadline: v["deadline"].as_u64().unwrap(),
    };
    assert_eq!(
        decide(&options, &envelope(&v), expected)
            .await
            .err()
            .unwrap()
            .code,
        "client_peer_or_transport"
    );
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), observed)
            .await
            .unwrap()
            .unwrap(),
        0
    );
}

#[cfg(windows)]
#[tokio::test]
async fn nonreading_pipe_clients_do_not_accumulate_flush_workers() {
    let d = Directory::new();
    let mut c = config_value(&d);
    c["capacity"] = 4.into();
    c["timeout_ms"] = 125.into();
    c["replay_capacity"] = 512.into();
    c["rate_per_second"] = 10_000.into();
    let p = Process::start(&c);
    let mut abandoned = Vec::new();
    // More than the service's 128 blocking-thread ceiling. Previously each
    // nonreader retained an uncancelled FlushFileBuffers worker after timeout.
    for batch in 0..36 {
        for offset in 0..4 {
            let v = request(now_ms().unwrap(), 1000 + batch * 4 + offset);
            let body = envelope(&v);
            let head = format!(
                "POST /adapter/v0.1/decision HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                p.credential,
                body.len()
            );
            let mut stream = connect(&p.endpoint).await;
            let frame = [head.as_bytes(), &body].concat();
            // An overload/expired prior connection may refuse transport, but
            // no successful response is read by this deliberately hostile peer.
            let _ = stream.write_all(&frame).await;
            abandoned.push(stream);
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    assert_eq!(
        p.send(&p.credential, &request(now_ms().unwrap(), 2000))
            .await
            .0,
        200
    );
    drop(abandoned);
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
