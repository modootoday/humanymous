//! Rust-only driver for actual nginx and shared authority on owned loopback/UDS.
mod authority;
mod origin;
use humanymous_authority::client::{self, ClientOptions, ExpectedBinding};
use humanymous_integrity::adapter::AdapterProfile;
use humanymous_nginx_adapter::request::{Configuration, Ingress, nonce};
use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

type Error = Box<dyn std::error::Error>;
const PATHS: &[(&str, &str)] = &[
    ("POST", "/echo"),
    ("GET", "/echo"),
    ("HEAD", "/echo"),
    ("GET", "/cookies"),
    ("GET", "/stream"),
    ("GET", "/redirect"),
    ("GET", "/empty"),
    ("GET", "/deny"),
    ("GET", "/challenge"),
    ("GET", "/production"),
    ("GET", "/audit-fail"),
    ("GET", "/capture"),
    ("GET", "/cancel"),
    ("GET", "/timeout"),
];
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn ensure(condition: bool, label: &'static str) -> Result<(), Error> {
    if condition { Ok(()) } else { Err(label.into()) }
}
fn private(path: &Path) -> Result<(), Error> {
    std::fs::create_dir(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}
fn start_production(startup: &serde_json::Value) -> Result<(Process, serde_json::Value), Error> {
    use std::io::BufRead;
    let mut process = Process(
        Command::new("/opt/authority")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let mut input = process.0.stdin.take().ok_or("private configuration pipe")?;
    input.write_all(&serde_json::to_vec(startup)?)?;
    drop(input);
    let mut line = String::new();
    BufReader::new(process.0.stdout.take().ok_or("private readiness pipe")?)
        .read_line(&mut line)?;
    let ready: serde_json::Value = serde_json::from_str(&line)?;
    ensure(
        ready["protocol"] == "humanymous.adapter/0.1" && ready["credential"].as_str().is_some(),
        "production readiness",
    )?;
    Ok((process, ready))
}

pub fn run() -> Result<(), Error> {
    humanymous_nginx_adapter::report::source_identity(
        option_env!("HUMANYMOUS_SOURCE_REV").unwrap_or(""),
        option_env!("HUMANYMOUS_SOURCE_MANIFEST").unwrap_or(""),
    )?;
    ensure(
        std::env::var("HUMANYMOUS_NGINX_OWNED_DOCKER").as_deref() == Ok("1"),
        "explicit owned Docker marker required",
    )?;
    ensure(
        rustix::process::geteuid().as_raw() != 0,
        "non-root pilot required",
    )?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    runtime.block_on(async_run())
}
async fn async_run() -> Result<(), Error> {
    let base = std::env::temp_dir().join(format!("h-nginx-{}", nonce()?));
    private(&base)?;
    let service = authority::start(&base, PATHS).await?;
    let origin = origin::Origin::start(service.durable.clone())?;
    let config_value = serde_json::json!({"endpoint":base.join("decision.sock"),"credential":service.credential,
        "server":{"kind":"unix","uid":rustix::process::geteuid().as_raw()},"installation":"nginx","tenant":"t","origin":"o","revision":"r","authority":"example.test","timeout_ms":1200,
        "routes":PATHS.iter().map(|(method,path)|serde_json::json!({"method":method,"path":path})).collect::<Vec<_>>()});
    let bytes = serde_json::to_vec(&config_value)?;
    let config = Configuration::parse(&bytes)?;
    let config_path = base.join("provision.json");
    use std::os::unix::fs::OpenOptionsExt;
    let mut secret = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&config_path)?;
    secret.write_all(&bytes)?;
    secret.sync_all()?;
    drop(secret);
    let free = TcpListener::bind(("127.0.0.1", 0))?;
    let port = free.local_addr()?.port();
    drop(free);
    let nginx_conf = base.join("nginx.conf");
    std::fs::write(
        &nginx_conf,
        format!(
            r#"
load_module /opt/nginx/modules/libhumanymous_nginx_adapter.so;
daemon off;
worker_processes 1;
pid {base}/nginx.pid;
error_log {base}/nginx-error.log crit;
events {{worker_connections 64;}}
http {{
 access_log off;
 client_body_temp_path {base}/body;
 proxy_temp_path {base}/proxy;
 client_max_body_size 1m;
 client_body_buffer_size 2m;
 client_body_in_single_buffer on;
 client_body_timeout 3s;
 server {{
  listen 127.0.0.1:{port};
  server_name example.test;
  humanymous_guard {provision};
  location / {{
   proxy_pass http://127.0.0.1:{origin};
   proxy_http_version 1.1;
   proxy_set_header Host $http_host;
   proxy_set_header Connection close;
   proxy_next_upstream off;
   proxy_buffering off;
   proxy_cache off;
   proxy_ignore_headers X-Accel-Redirect X-Accel-Buffering;
  }}
 }}
}}
"#,
            base = base.display(),
            provision = config_path.display(),
            origin = origin.port
        ),
    )?;
    std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o644))?;
    let bad = Command::new("/opt/nginx/sbin/nginx")
        .args(["-t", "-p"])
        .arg(&base)
        .arg("-c")
        .arg(&nginx_conf)
        .output()?;
    ensure(!bad.status.success(), "public credential file accepted")?;
    std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600))?;
    let check = Command::new("/opt/nginx/sbin/nginx")
        .args(["-t", "-p"])
        .arg(&base)
        .arg("-c")
        .arg(&nginx_conf)
        .output()?;
    ensure(
        check.status.success(),
        "nginx configuration/ABI load failed",
    )?;
    let _nginx = Process(
        Command::new("/opt/nginx/sbin/nginx")
            .arg("-p")
            .arg(&base)
            .arg("-c")
            .arg(&nginx_conf)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let mut ready = false;
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    ensure(ready, "nginx startup")?;
    let mut cases = Vec::new();
    cases.push(serde_json::json!({"name":"nginx-startup-private-credential-ACL","public_mode_rejected":true,"origin_delta":0}));
    for (name, method, target, extra, status) in [
        ("deny", "GET", "/deny", "", 403),
        ("challenge", "GET", "/challenge", "", 409),
        (
            "reserved-management",
            "GET",
            "/_humanymous/admin",
            "X-Forwarded-For: 127.0.0.1\r\n",
            400,
        ),
        ("unknown-route", "GET", "/unknown", "", 400),
        ("upgrade", "GET", "/echo", "Upgrade: websocket\r\n", 400),
        ("trailers", "GET", "/echo", "Trailer: digest\r\n", 400),
        (
            "grpc",
            "POST",
            "/echo",
            "Content-Type: application/grpc\r\n",
            400,
        ),
        ("encoded-target", "GET", "/echo%2fadmin", "", 400),
        ("required-capture", "GET", "/capture", "", 503),
        (
            "duplicate-content-length",
            "POST",
            "/echo",
            "Content-Length: 0\r\n",
            400,
        ),
        (
            "transfer-encoding-plus-length",
            "POST",
            "/echo",
            "Transfer-Encoding: chunked\r\n",
            400,
        ),
        ("duplicate-host", "GET", "/echo", "Host: evil.test\r\n", 400),
        ("connect", "CONNECT", "/echo", "", 400),
        ("rewritten-dot-target", "GET", "/echo/../echo", "", 400),
    ] {
        let before = origin.count();
        let result = origin::request(port, method, target, &[], extra)
            .map_err(|error| format!("case {name}: {error}"))?;
        ensure(result.status == status, "negative status")?;
        ensure(origin.count() == before, "negative origin effect")?;
        if name == "challenge" {
            ensure(
                result
                    .values("content-type")
                    .iter()
                    .any(|value| value.starts_with("application/json")),
                "challenge JSON content type",
            )?;
            let json: serde_json::Value = serde_json::from_slice(&result.body)?;
            ensure(
                json["error"] == "challenge_unavailable",
                "challenge JSON body",
            )?;
            // Each keepalive request needs a fresh context/nonce/audit, not a
            // cached result or a response whose pool timer was never finalized.
            let decisions = service.durable.load(Ordering::Acquire);
            let persistent = TcpStream::connect(("127.0.0.1", port))?;
            persistent.set_read_timeout(Some(Duration::from_secs(4)))?;
            let mut reader = BufReader::new(persistent);
            for _ in 0..4 {
                reader.get_mut().write_all(b"GET /challenge HTTP/1.1\r\nHost: example.test\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n")?;
                let reply = origin::read_response(&mut reader, "GET")?;
                let json: serde_json::Value = serde_json::from_slice(&reply.body)?;
                ensure(
                    reply.status == 409 && json["error"] == "challenge_unavailable",
                    "persistent challenge reply",
                )?;
            }
            ensure(
                service.durable.load(Ordering::Acquire) == decisions + 4,
                "persistent request reused decision",
            )?;
            ensure(
                origin.count() == before,
                "persistent challenge reached origin",
            )?;
        }
        if ![
            "duplicate-content-length",
            "transfer-encoding-plus-length",
            "duplicate-host",
            "connect",
        ]
        .contains(&name)
        {
            ensure(
                result.values("cache-control").contains(&"no-store"),
                "module negative cache policy",
            )?;
        }
        cases.push(serde_json::json!({"name":name,"status":result.status,"origin_delta":0}));
    }
    let before = origin.count();
    let result = origin::request(port, "GET", "/production", &[], "")?;
    ensure(
        result.status == 409 || result.status == 403,
        "production absent signals fabricated ALLOW",
    )?;
    ensure(
        origin.count() == before,
        "production unavailable observation origin effect",
    )?;
    let production_status = result.status;
    cases.push(serde_json::json!({"name":"actual-application-http","status":result.status,"origin_delta":0,"human_qualified":false}));
    for (name, body, extra) in [
        ("empty", Vec::new(), ""),
        (
            "binary",
            vec![0, 255, 128, 13, 10],
            "Content-Type: application/octet-stream\r\n",
        ),
        (
            "compressed",
            vec![
                31, 139, 8, 0, 0, 0, 0, 0, 0, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            "Content-Encoding: gzip\r\n",
        ),
        (
            "complete-1MiB",
            vec![42; 1_048_576],
            "Content-Type: application/octet-stream\r\n",
        ),
    ] {
        let before = origin.count();
        let audit = service.durable.load(Ordering::Acquire);
        let response = origin::request(port, "POST", "/echo?q=untouched%20query", &body, extra)?;
        ensure(
            response.status == 200 && response.body == body,
            "authorized binary/body response",
        )?;
        ensure(origin.count() == before + 1, "single origin effect")?;
        let effect = origin.last();
        ensure(
            effect.target == "/echo?q=untouched%20query"
                && effect.method == "POST"
                && effect.body == body,
            "unchanged request binding",
        )?;
        ensure(
            effect.durable_at_effect > audit,
            "durable audit before origin",
        )?;
        cases.push(serde_json::json!({"name":name,"status":200,"origin_delta":1,"evidence":"trusted synthetic fixture, actual authority+WAL"}));
    }
    let response = origin::request(
        port,
        "POST",
        "/echo",
        b"headers",
        "X-Dup: one\r\nx-dup: two\r\nX-Arbitrary: kept\r\nForwarded: for=127.0.0.1\r\nX-Forwarded-For: 127.0.0.1\r\nX-Humanymous-Allow: true\r\nCF-Connecting-IP: 127.0.0.1\r\n",
    )?;
    ensure(response.status == 200, "header passthrough request")?;
    let effect = origin.last();
    let duplicates: Vec<_> = effect
        .headers
        .iter()
        .filter(|h| h.0.eq_ignore_ascii_case("x-dup"))
        .map(|h| h.1.as_str())
        .collect();
    ensure(duplicates == ["one", "two"], "duplicate request order")?;
    ensure(
        effect
            .headers
            .iter()
            .any(|h| h.0.eq_ignore_ascii_case("x-arbitrary") && h.1 == "kept"),
        "ordinary header loss",
    )?;
    ensure(
        !effect
            .headers
            .iter()
            .any(|h| humanymous_nginx_adapter::request::strip_header(&h.0)),
        "spoof header origin leak",
    )?;
    ensure(
        response.values("set-cookie").len() == 2 && response.values("x-repeat").len() == 2,
        "response duplicate loss",
    )?;
    ensure(response.values("etag") == ["\"fixture\""], "ETag changed")?;
    cases.push(serde_json::json!({"name":"duplicate-headers-cookie-and-spoof-strip","status":200}));
    let before = origin.count();
    let redirect = origin::request(port, "GET", "/redirect", &[], "")?;
    ensure(
        redirect.status == 200
            && origin.count() == before + 1
            && origin.last().target == "/redirect",
        "X-Accel rewrite caused extra origin effect",
    )?;
    cases.push(serde_json::json!({"name":"origin-redirect-not-executed","origin_delta":1}));
    let empty = origin::request(port, "GET", "/empty", &[], "")?;
    ensure(
        empty.status == 204 && empty.body.is_empty(),
        "204 semantics",
    )?;
    let head = origin::request(port, "HEAD", "/echo", &[], "")?;
    ensure(head.status == 200 && head.body.is_empty(), "HEAD semantics")?;
    cases.push(serde_json::json!({"name":"HEAD-and-204","passed":2}));
    let mut s = TcpStream::connect(("127.0.0.1", port))?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    let started = Instant::now();
    s.write_all(b"GET /stream HTTP/1.1\r\nHost: example.test\r\nConnection: close\r\n\r\n")?;
    let mut reader = BufReader::new(s);
    let header = origin::head(&mut reader)?;
    ensure(header.starts_with("HTTP/1.1 200"), "SSE response")?;
    use std::io::BufRead;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let length = usize::from_str_radix(line.trim(), 16)?;
    let mut first = vec![0; length];
    reader.read_exact(&mut first)?;
    ensure(
        first == b"data: a\n\n" && started.elapsed() < Duration::from_millis(800),
        "SSE buffered to completion",
    )?;
    cases.push(
        serde_json::json!({"name":"SSE-first-event","latency_ms":started.elapsed().as_millis()}),
    );
    drop(reader);
    // Slow incomplete body: the plugin's total deadline includes reception.
    let before = origin.count();
    let mut slow = TcpStream::connect(("127.0.0.1", port))?;
    slow.set_read_timeout(Some(Duration::from_secs(4)))?;
    slow.write_all(b"POST /echo HTTP/1.1\r\nHost: example.test\r\nContent-Length: 10\r\nConnection: close\r\n\r\nx")?;
    let slow_head = origin::head(&mut BufReader::new(slow))?;
    ensure(slow_head.starts_with("HTTP/1.1 503"), "total body deadline")?;
    ensure(origin.count() == before, "incomplete body reached origin")?;
    cases.push(serde_json::json!({"name":"total-body-deadline","status":503,"origin_delta":0}));
    let before = origin.count();
    let mut cancelled = TcpStream::connect(("127.0.0.1", port))?;
    cancelled
        .write_all(b"GET /cancel HTTP/1.1\r\nHost: example.test\r\nConnection: close\r\n\r\n")?;
    cancelled.shutdown(std::net::Shutdown::Both)?;
    drop(cancelled);
    tokio::time::sleep(Duration::from_millis(700)).await;
    ensure(origin.count() == before, "cancelled request reached origin")?;
    cases.push(serde_json::json!({"name":"client-cancellation","origin_delta":0}));
    let before = origin.count();
    let timed = origin::request(port, "GET", "/timeout", &[], "")?;
    ensure(
        timed.status == 503 && origin.count() == before,
        "IPC timeout reached origin",
    )?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    cases.push(serde_json::json!({"name":"actual-authority-delayed-fixture-timeout","status":503,"origin_delta":0}));
    // Owned concurrent requests: count admitted effects and explicit overload,
    // never count capacity rejection as ordinary-service success.
    let before = origin.count();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(32));
    let mut jobs = Vec::new();
    for n in 0_u32..32 {
        let start = barrier.clone();
        jobs.push(std::thread::spawn(move || {
            start.wait();
            let body = n.to_be_bytes();
            origin::request(port, "POST", "/echo", &body, "").map(|reply| (reply, body))
        }));
    }
    let mut admitted = 0;
    let mut capacity = 0;
    for job in jobs {
        let (reply, body) = job.join().map_err(|_| "load worker panic")??;
        match reply.status {
            200 => {
                ensure(reply.body == body, "concurrent body binding")?;
                admitted += 1;
            }
            503 => capacity += 1,
            _ => return Err("unexpected concurrent status".into()),
        }
    }
    ensure(
        admitted > 0 && admitted + capacity == 32 && origin.count() == before + admitted,
        "concurrent origin accounting",
    )?;
    cases.push(serde_json::json!({"name":"owned-concurrent-service-accounting","requests":32,"allowed":admitted,"capacity_or_deadline_rejected":capacity,"origin_delta":admitted}));
    // Shared actual IPC replay and credential failures, independent of origin.
    let ingress = Ingress {
        method: "POST".into(),
        target: "/echo".into(),
        path: "/echo".into(),
        headers: vec![
            ["Host".into(), "example.test".into()],
            ["Content-Length".into(), "0".into()],
        ],
        body: Vec::new(),
    };
    let id = nonce()?;
    let deadline = humanymous_authority::now_ms().map_err(|_| "clock")? + 1000;
    let envelope = ingress.envelope(
        &config,
        &id,
        deadline,
        humanymous_authority::now_ms().map_err(|_| "clock")?,
    )?;
    let options = ClientOptions {
        endpoint: &config.endpoint,
        credential: &config.credential,
        server: &config.server,
        timeout_ms: 1000,
        adapter: AdapterProfile::Nginx,
    };
    let binding = || ExpectedBinding {
        id: &id,
        tenant: "t",
        origin: "o",
        revision: "r",
        deadline,
    };
    let before = origin.count();
    let first = client::decide(&options, &envelope, binding())
        .await
        .map_err(|_| "direct IPC first")?;
    ensure(first.consume().is_ok(), "direct bound result")?;
    ensure(
        client::decide(&options, &envelope, binding())
            .await
            .is_err(),
        "replay accepted",
    )?;
    ensure(origin.count() == before, "direct IPC invoked origin")?;
    cases.push(serde_json::json!({"name":"actual-authority-replay","second_rejected":true,"origin_delta":0}));
    let other = humanymous_authority::configuration::Peer::Unix {
        uid: rustix::process::geteuid().as_raw() + 1,
    };
    let wrong = ClientOptions {
        server: &other,
        ..options
    };
    ensure(
        client::decide(&wrong, &envelope, binding()).await.is_err(),
        "wrong server UID accepted",
    )?;
    cases.push(serde_json::json!({"name":"actual-client-server-UID-mismatch","rejected":true,"origin_delta":0}));
    service.fail.store(true, Ordering::Release);
    let before = origin.count();
    let failed = origin::request(port, "GET", "/audit-fail", &[], "")?;
    ensure(
        failed.status == 503 && origin.count() == before,
        "audit failure reached origin",
    )?;
    cases.push(serde_json::json!({"name":"injected-durable-audit-dependency-failure","status":503,"origin_delta":0}));
    drop(service);
    tokio::time::sleep(Duration::from_millis(50)).await;
    let before = origin.count();
    let failed = origin::request(port, "GET", "/echo", &[], "")?;
    ensure(
        failed.status == 503 && origin.count() == before,
        "authority outage reached origin",
    )?;
    cases.push(serde_json::json!({"name":"actual-authority-outage","status":503,"origin_delta":0}));
    // Real production executable, private startup/readiness pipes, fresh epoch.
    // This separate audit directory does not qualify same-WAL crash recovery.
    let production = base.join("production");
    private(&production)?;
    let production_audit = production.join("audit");
    private(&production_audit)?;
    let mut key = [0; 32];
    let mut seed = [0; 32];
    getrandom::fill(&mut key)?;
    getrandom::fill(&mut seed)?;
    let startup = serde_json::json!({"adapter":"nginx","endpoint":production.join("decision.sock"),
        "scope":{"installation":"nginx","tenant":"t","origin":"o","revision":"r","authority":"example.test"},
        "peer":{"kind":"unix","uid":rustix::process::geteuid().as_raw()},
        "routes":[{"method":"GET","path":"/production","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":false}}],
        "audit":{"directory":production_audit,"node_id":"nginx-production-pilot","hmac_key_hex":hex::encode(key),"signing_seed_hex":hex::encode(seed),"allow_new":true,"trusted_head":null},
        "capacity":4,"replay_capacity":64,"rate_per_second":100,"timeout_ms":1200,"clock_skew_ms":1000});
    let (mut executable, ready) = start_production(&startup)?;
    let credential = ready["credential"].as_str().ok_or("readiness credential")?;
    ensure(
        ready["protocol"] == "humanymous.adapter/0.1",
        "production readiness protocol",
    )?;
    let mut next = config_value.clone();
    next["credential"] = serde_json::json!(credential);
    next["endpoint"] = startup["endpoint"].clone();
    std::fs::write(&config_path, serde_json::to_vec(&next)?)?;
    let reload = Command::new("/opt/nginx/sbin/nginx")
        .args(["-s", "reload", "-p"])
        .arg(&base)
        .arg("-c")
        .arg(&nginx_conf)
        .output()?;
    ensure(reload.status.success(), "nginx credential reload command")?;
    let before = origin.count();
    let mut observed = Vec::new();
    let mut live = false;
    for _ in 0..50 {
        let response = origin::request(port, "GET", "/production", &[], "")?;
        observed.push(response.status);
        if response.status == 409 {
            live = true;
            break;
        }
        ensure(response.status == 503, "unexpected reload transition")?;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    ensure(
        live && origin.count() == before,
        "production executable fabricated ALLOW or reload failed",
    )?;
    cases.push(serde_json::json!({"name":"actual-production-executable-private-pipes-and-nginx-reload","statuses":observed,"origin_delta":0,"human_qualified":false,"audit_directory":"fresh separate fixture directory"}));
    let deadline = humanymous_authority::now_ms().map_err(|_| "clock")? + 1000;
    let id = nonce()?;
    let input = Ingress {
        method: "GET".into(),
        path: "/production".into(),
        target: "/production".into(),
        headers: vec![["Host".into(), "example.test".into()]],
        body: Vec::new(),
    };
    let wire = input.envelope(
        &config,
        &id,
        deadline,
        humanymous_authority::now_ms().map_err(|_| "clock")?,
    )?;
    let endpoint = next["endpoint"].as_str().ok_or("production endpoint")?;
    let stale = ClientOptions {
        endpoint,
        credential: &config.credential,
        server: &config.server,
        timeout_ms: 1000,
        adapter: AdapterProfile::Nginx,
    };
    ensure(
        client::decide(
            &stale,
            &wire,
            ExpectedBinding {
                id: &id,
                tenant: "t",
                origin: "o",
                revision: "r",
                deadline,
            },
        )
        .await
        .is_err(),
        "stale credential crossed epoch",
    )?;
    ensure(origin.count() == before, "stale credential origin effect")?;
    cases.push(serde_json::json!({"name":"authority-epoch-stale-credential","rejected":true,"origin_delta":0}));
    executable.0.kill()?;
    executable.0.wait()?;
    // Only the trusted supervisor removes its own stale socket after confirmed
    // process exit and owner/type checks; the authority never unlinks on startup.
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let socket = production.join("decision.sock");
    let meta = std::fs::symlink_metadata(&socket)?;
    ensure(
        meta.file_type().is_socket()
            && meta.uid() == rustix::process::geteuid().as_raw()
            && meta.mode() & 0o777 == 0o600,
        "stale socket ownership",
    )?;
    std::fs::remove_file(socket)?;
    use humanymous_audit::{
        log::{Config, Log},
        storage::{Wal, WalOptions},
    };
    let audit_config = || Config {
        node_id: "nginx-production-pilot".into(),
        hmac_key: key.to_vec(),
        signing_seed: seed,
        checkpoint_every: 1,
        witness: None,
    };
    let recovered = Log::open(
        Wal::open(&production_audit, WalOptions::default())?,
        audit_config(),
        None,
        false,
    )?;
    let sequence = recovered.sequence();
    ensure(sequence > 0, "production audit empty")?;
    let anchor = recovered
        .checkpoints()?
        .last()
        .cloned()
        .ok_or("production checkpoint")?;
    drop(recovered);
    let mut restart = startup.clone();
    restart["audit"]["allow_new"] = serde_json::json!(false);
    restart["audit"]["trusted_head"] = serde_json::to_value(&anchor)?;
    let (mut successor, ready2) = start_production(&restart)?;
    let credential2 = ready2["credential"].as_str().ok_or("successor readiness")?;
    ensure(
        credential2 != credential,
        "authority restart reused credential",
    )?;
    let old_worker = origin::request(port, "GET", "/production", &[], "")?;
    ensure(
        old_worker.status == 503,
        "old nginx worker credential survived restart",
    )?;
    next["credential"] = serde_json::json!(credential2);
    std::fs::write(&config_path, serde_json::to_vec(&next)?)?;
    ensure(
        Command::new("/opt/nginx/sbin/nginx")
            .args(["-s", "reload", "-p"])
            .arg(&base)
            .arg("-c")
            .arg(&nginx_conf)
            .output()?
            .status
            .success(),
        "successor reload",
    )?;
    let mut statuses = vec![old_worker.status];
    let mut live = false;
    for _ in 0..50 {
        let result = origin::request(port, "GET", "/production", &[], "")?;
        statuses.push(result.status);
        if result.status == 409 {
            live = true;
            break;
        }
        ensure(result.status == 503, "successor transition")?;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    ensure(
        live && origin.count() == before,
        "successor production origin effect",
    )?;
    successor.0.kill()?;
    successor.0.wait()?;
    let verified = Log::open(
        Wal::open(&production_audit, WalOptions::default())?,
        audit_config(),
        Some(&anchor),
        false,
    )?;
    ensure(
        verified.sequence() > sequence,
        "same-WAL restart lost audit continuity",
    )?;
    cases.push(serde_json::json!({"name":"actual-process-same-WAL-restart-and-nginx-reload","statuses":statuses,"origin_delta":0,"records_before":sequence,"records_after":verified.sequence(),"independent_witness":false}));
    let version = Command::new("/opt/nginx/sbin/nginx").arg("-V").output()?;
    use sha2::Digest;
    let artifact = std::fs::read("/opt/nginx/modules/libhumanymous_nginx_adapter.so")?;
    let engine = std::fs::read("/opt/nginx/sbin/nginx")?;
    let lock = std::fs::read("/opt/nginx/source-Cargo.lock")?;
    let package_bytes = std::fs::read("/opt/nginx/modules/package-manifest.json")?;
    let package = humanymous_nginx_adapter::package::Manifest::parse(&package_bytes)?;
    let module_identity = humanymous_nginx_adapter::package::artifact(
        std::path::Path::new("/opt/nginx/modules/libhumanymous_nginx_adapter.so"),
        64 * 1024 * 1024,
    )?;
    let engine_identity = humanymous_nginx_adapter::package::artifact(
        std::path::Path::new("/opt/nginx/sbin/nginx"),
        64 * 1024 * 1024,
    )?;
    package.matches(
        &module_identity,
        &engine_identity,
        &hex::encode(sha2::Sha256::digest(&lock)),
    )?;
    ensure(
        package.source_revision == option_env!("HUMANYMOUS_SOURCE_REV").unwrap_or("")
            && package.source_manifest == option_env!("HUMANYMOUS_SOURCE_MANIFEST").unwrap_or("")
            && package.engine_build.as_bytes() == version.stderr,
        "package build binding",
    )?;
    let groups = humanymous_nginx_adapter::report::completed_groups(&mut cases)?;
    println!(
        "{}",
        serde_json::json!({"schema":humanymous_nginx_adapter::report::SCHEMA,"status":"passed",
        "qualification":"component nginx+actual authority library; trusted synthetic ALLOW fixture",
        "source_revision":option_env!("HUMANYMOUS_SOURCE_REV"),"source_manifest":option_env!("HUMANYMOUS_SOURCE_MANIFEST"),
        "nginx":String::from_utf8_lossy(&version.stderr),"module_sha256":hex::encode(sha2::Sha256::digest(artifact)),
        "engine_sha256":hex::encode(sha2::Sha256::digest(engine)),"cargo_lock_sha256":hex::encode(sha2::Sha256::digest(lock)),
        "package_manifest_sha256":hex::encode(sha2::Sha256::digest(package_bytes)),"signed_release":false,
        "uid":rustix::process::geteuid().as_raw(),"production_profile_status":production_status,
        "cases":cases,"group_count":groups,"passed":groups,"failed":0,"ignored":0,"not_run":0,"full_product_qualified":false,
        "unsupported":["original TLS/H2/H3","streaming upload/upgrade/gRPC","collector/browser recovery/human/AT","non-Linux ABI"],
        "remaining":["independent witness","cross-user runtime matrix","load/backpressure and independent rollback-anchor qualification"]})
    );
    let _ = std::fs::remove_file(config_path);
    Ok(())
}
