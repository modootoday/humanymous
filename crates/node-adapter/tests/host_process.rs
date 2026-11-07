//! Real Node/Bun/framework processes. Component and production-authority
//! outcomes are counted separately; no JS/TS source is authored by this test.
use serde_json::{Value, json};
#[path = "support/http.rs"]
mod http;

// Synthetic browser evidence exists only in this test target. It never enters
// native exports, the production authority executable, or adapter wire fields.
struct Observed {
    headers: Vec<[String; 2]>,
    mode: String,
}
type Observations = std::sync::Arc<std::sync::Mutex<Vec<Observed>>>;
struct SyntheticEvidence(Observations);
impl humanymous_authority::decision::Evidence for SyntheticEvidence {
    fn report(
        &mut self,
        request: &humanymous_integrity::adapter::ValidatedRequest,
        _: bool,
    ) -> Result<humanymous_signals::report::SessionReport, humanymous_authority::Rejection> {
        self.0.lock().unwrap().push(Observed {
            headers: request.headers().to_vec(),
            mode: request.header_mode().to_owned(),
        });
        let fixture: Value = serde_json::from_str(include_str!(
            "../../scoring/tests/fixtures/frozen/human-chrome-allow.json"
        ))
        .unwrap();
        Ok(serde_json::from_value(fixture["report"].clone()).unwrap())
    }
}
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};

struct Host(Child);
const PATHS: &[&str] = &["/api", "/iter", "/sse", "/redirect", "/empty"];
impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn read_line(child: &mut Child) -> String {
    let stdout = child.stdout.take().expect("host stdout");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line);
        let _ = tx.send((result, line));
    });
    let (_, line) = rx
        .recv_timeout(Duration::from_secs(20))
        .expect("native host readiness deadline");
    if line.is_empty() {
        let mut error = String::new();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut error)
            .unwrap();
        panic!("native host startup: {error}");
    }
    line
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
fn addon() -> PathBuf {
    let path=std::env::var_os("HUMANYMOUS_NODE_ADDON").map(PathBuf::from).expect("build host-pilot cdylib and provision HUMANYMOUS_NODE_ADDON; missing runner is not success");
    assert_eq!(path.extension().unwrap(), "node");
    assert!(path.is_absolute() && path.is_file());
    path
}
fn process(
    runtime: &str,
    framework: &str,
    endpoint: &str,
    credential: &str,
    server: Value,
) -> (Host, u16) {
    let child = spawn_host(runtime);
    configure_host(child, framework, endpoint, credential, server, None)
}
fn spawn_host(runtime: &str) -> Child {
    let preload = if runtime == "bun" {
        let path = PathBuf::from(std::env::var_os("HUMANYMOUS_BUN_PRELOAD").expect(
            "Bun requires pinned upstream-generated CommonJS preload, not direct .node import",
        ));
        assert!(path.is_absolute() && path.is_file());
        path
    } else {
        addon()
    };
    Command::new(runtime)
        .arg("--require")
        .arg(preload)
        // Bun requires an actual entrypoint. This unchanged upstream module is
        // only the host entrypoint; native Rust callbacks own every pilot.
        .arg(root().join("adapters/node/node_modules/express/index.js"))
        .env("HUMANYMOUS_NODE_HOST_PILOT", "1")
        .env("HUMANYMOUS_NODE_TRACE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("actual runtime required")
}
fn configure_host(
    child: Child,
    framework: &str,
    endpoint: &str,
    credential: &str,
    server: Value,
    effect_file: Option<PathBuf>,
) -> (Host, u16) {
    let mut host = Host(child);
    let config = json!({"framework":framework,"package_json":root().join("adapters/node/package.json"),"effect_file":effect_file,"config":{"installation":"i","tenant":"t","origin":"o","revision":"r","endpoint":endpoint,"credential":credential,"server":server,"routes":PATHS,"timeoutMs":1000,"maxBody":1024}});
    host.0
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
        .unwrap();
    let ready: Value = serde_json::from_str(&read_line(&mut host.0)).unwrap();
    assert_eq!(ready["kind"], "rust-node-api-host");
    (host, ready["port"].as_u64().unwrap() as u16)
}

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-node-{}-{}",
            std::process::id(),
            humanymous_authority::now_ms().unwrap()
        ));
        std::fs::create_dir(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        #[cfg(windows)]
        {
            use windows_permissions::{
                LocalBox, SecurityDescriptor, Sid,
                constants::{SeObjectType, SecurityInformation},
                wrappers,
            };
            let identity = humanymous_native_security::process::current_identity().unwrap();
            let sid = identity.sid();
            let owner: LocalBox<Sid> = sid.parse().unwrap();
            let sd: LocalBox<SecurityDescriptor> =
                format!("D:P(A;OICI;FA;;;{sid})").parse().unwrap();
            wrappers::SetNamedSecurityInfo(
                path.as_os_str(),
                SeObjectType::SE_FILE_OBJECT,
                SecurityInformation::Owner
                    | SecurityInformation::Dacl
                    | SecurityInformation::ProtectedDacl,
                Some(&owner),
                None,
                sd.dacl(),
                None,
            )
            .unwrap();
        }
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn service_config(directory: &Directory, endpoint: &str, peer: Value, node_id: &str) -> Value {
    let mut routes=PATHS.iter().map(|path|json!({"method":"POST","path":path,"required":{"raw_tls":false,"original_h2":false,"raw_header_order":false,"collector":false,"rit":false}})).collect::<Vec<_>>();
    routes.push(json!({"method":"HEAD","path":"/empty","required":{"raw_tls":false,"original_h2":false,"raw_header_order":false,"collector":false,"rit":false}}));
    json!({"endpoint":endpoint,"peer":peer,"scope":{"installation":"i","tenant":"t","origin":"o","revision":"r","authority":"example.test"},"routes":routes,"audit":{"directory":directory.0,"node_id":node_id,"hmac_key_hex":"aa".repeat(32),"signing_seed_hex":"bb".repeat(32),"allow_new":true,"trusted_head":null},"capacity":8,"replay_capacity":32,"rate_per_second":100,"timeout_ms":2000,"clock_skew_ms":1000})
}

#[test]
#[ignore = "explicit actual Node/Bun plus real production authority native process pilot"]
fn actual_authority_decisions_are_enforced_before_origin() {
    let executable = std::env::var_os("HUMANYMOUS_AUTHORITY_BIN")
        .map(PathBuf::from)
        .expect("actual authority binary required");
    assert!(executable.is_file());
    for runtime in ["node", "bun"] {
        for framework in ["express", "hono"] {
            let directory = Directory::new();
            let node = spawn_host(runtime);
            #[cfg(windows)]
            let (endpoint, peer) = (
                format!(
                    r"\\.\pipe\humanymous-node-{}-{}",
                    std::process::id(),
                    node.id()
                ),
                json!({"kind":"windows","sid":humanymous_native_security::process::current_identity().unwrap().sid(),"pid":node.id()}),
            );
            #[cfg(unix)]
            let (endpoint, peer) = (
                directory
                    .0
                    .join("decision.sock")
                    .to_str()
                    .unwrap()
                    .to_owned(),
                json!({"kind":"unix","uid":humanymous_native_security::process::current_uid()}),
            );
            let config = service_config(&directory, &endpoint, peer, "node-host-pilot");
            let mut authority = Host(
                Command::new(&executable)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap(),
            );
            authority
                .0
                .stdin
                .take()
                .unwrap()
                .write_all(&serde_json::to_vec(&config).unwrap())
                .unwrap();
            let ready: Value = serde_json::from_str(&read_line(&mut authority.0)).unwrap();
            #[cfg(windows)]
            let server = json!({"kind":"windows","sid":humanymous_native_security::process::current_identity().unwrap().sid(),"pid":authority.0.id()});
            #[cfg(unix)]
            let server =
                json!({"kind":"unix","uid":humanymous_native_security::process::current_uid()});
            let effect_file = directory.0.join("effects.log");
            let (_node, port) = configure_host(
                node,
                framework,
                &endpoint,
                ready["credential"].as_str().unwrap(),
                server,
                Some(effect_file.clone()),
            );
            let bytes = request(port, "/api", &[0, 255, 42]);
            let wire = String::from_utf8_lossy(&bytes);
            assert!(
                wire.starts_with("HTTP/1.1 409") || wire.starts_with("HTTP/1.1 403"),
                "real authority outcome must be visible: {wire}"
            );
            assert!(!wire.contains("x-origin-effect"));
            assert!(!effect_file.exists());
            assert!(
                std::fs::read_dir(&directory.0)
                    .unwrap()
                    .filter_map(Result::ok)
                    .any(|p| p.path().is_file() && p.metadata().unwrap().len() > 0)
            );
            println!(
                "{}",
                json!({"runtime":runtime,"framework":framework,"authority":"production-application-http","originEffects":0,"status":"passed","fullProduct":false})
            );
        }
    }
}

#[test]
#[ignore = "explicit real Node/Bun compatibility with synthetic evidence in the real Rust engine and audit, not production browser qualification"]
fn actual_framework_allow_preserves_binary_body_and_headers() {
    use humanymous_audit::{
        log::{Config, Log},
        storage::{Wal, WalOptions},
    };
    use humanymous_authority::{
        configuration::Configuration, decision::Authority, transport::Listener,
    };
    for runtime_name in ["node", "bun"] {
        for framework in ["express", "hono"] {
            let directory = Directory::new();
            let child = spawn_host(runtime_name);
            #[cfg(windows)]
            let (endpoint, peer, server) = {
                let sid = humanymous_native_security::process::current_identity()
                    .unwrap()
                    .sid()
                    .to_owned();
                (
                    format!(
                        r"\\.\pipe\humanymous-allow-{}-{}",
                        std::process::id(),
                        child.id()
                    ),
                    json!({"kind":"windows","sid":sid,"pid":child.id()}),
                    json!({"kind":"windows","sid":sid,"pid":std::process::id()}),
                )
            };
            #[cfg(unix)]
            let (endpoint, peer, server) = {
                let uid = humanymous_native_security::process::current_uid();
                (
                    directory
                        .0
                        .join("decision.sock")
                        .to_string_lossy()
                        .into_owned(),
                    json!({"kind":"unix","uid":uid}),
                    json!({"kind":"unix","uid":uid}),
                )
            };
            let config_value = service_config(&directory, &endpoint, peer, "node-synthetic-pilot");
            let config: Configuration = serde_json::from_value(config_value.clone()).unwrap();
            let log = Log::open(
                Wal::open(&directory.0, WalOptions::default()).unwrap(),
                Config {
                    node_id: "node-synthetic-pilot".into(),
                    hmac_key: vec![0xaa; 32],
                    signing_seed: [0xbb; 32],
                    checkpoint_every: 1,
                    witness: None,
                },
                None,
                true,
            )
            .unwrap();
            let observed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let authority = Authority::new(
                config,
                Default::default(),
                Box::new(SyntheticEvidence(observed.clone())),
                Box::new(log),
            )
            .unwrap();
            let credential = authority.credential().to_owned();
            let config: Configuration = serde_json::from_value(config_value).unwrap();
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            let listener = {
                let _guard = runtime.enter();
                Listener::bind(&config).unwrap()
            };
            let service = runtime.spawn(listener.serve(authority, 8, 100, 2000));
            let effect_file = directory.0.join("effects.log");
            let (host, port) = configure_host(
                child,
                framework,
                &endpoint,
                &credential,
                server,
                Some(effect_file.clone()),
            );
            let body = [0, 255, 13, 10, 42];
            let wire = request(port, "/api?q=1", &body);
            let separator = wire
                .windows(4)
                .position(|v| v == b"\r\n\r\n")
                .expect("response headers");
            let headers = String::from_utf8_lossy(&wire[..separator]).to_ascii_lowercase();
            assert!(
                headers.starts_with("http/1.1 201"),
                "{runtime_name}/{framework}: {headers}"
            );
            assert!(headers.contains("x-origin-effect: one"));
            let observations = observed.lock().unwrap();
            assert_eq!(
                observations[0].mode,
                if framework == "express" && runtime_name == "node" {
                    "node-raw"
                } else {
                    "fetch-normalized"
                }
            );
            let captured = &observations[0].headers;
            assert!(!captured.iter().any(|[name, _]| {
                name.to_ascii_lowercase().starts_with("x-forwarded-")
                    || name.to_ascii_lowercase().starts_with("x-humanymous-")
            }));
            assert!(
                captured
                    .iter()
                    .any(|[name, value]| name.eq_ignore_ascii_case("authorization")
                        && value == "Bearer application-fixture")
            );
            if framework == "express" && runtime_name == "node" {
                let ordered: Vec<_> = captured
                    .iter()
                    .filter(|[name, _]| name.to_ascii_lowercase().starts_with("x-order-"))
                    .cloned()
                    .collect();
                assert_eq!(
                    ordered,
                    vec![
                        ["X-Order-B".to_owned(), "first".to_owned()],
                        ["x-order-a".to_owned(), "second".to_owned()],
                        ["X-Order-B".to_owned(), "third".to_owned()]
                    ],
                    "raw provenance {runtime_name}/{framework}"
                );
            }
            if framework == "express" {
                assert_eq!(headers.matches("set-cookie:").count(), 2);
                assert_eq!(http::body(&wire), body);
            } else {
                assert_eq!(http::body(&wire), body);
            }
            drop(observations);
            let iter = request(port, "/iter", &body);
            assert!(iter.starts_with(b"HTTP/1.1 201"));
            assert_eq!(http::body(&iter), body);
            let sse = request(port, "/sse", b"");
            assert!(sse.starts_with(b"HTTP/1.1 200"));
            assert!(
                String::from_utf8_lossy(&sse)
                    .to_ascii_lowercase()
                    .contains("content-type: text/event-stream")
            );
            assert_eq!(http::body(&sse), b"data: first\n\ndata: second\n\n");
            let redirect = request(port, "/redirect", b"");
            assert!(redirect.starts_with(b"HTTP/1.1 302"));
            assert!(
                String::from_utf8_lossy(&redirect)
                    .to_ascii_lowercase()
                    .contains("location: /api?q=kept")
            );
            let empty = request(port, "/empty", b"");
            assert!(empty.starts_with(b"HTTP/1.1 204"));
            assert!(http::body(&empty).is_empty());
            let head = try_request_method(port, "HEAD", "/empty", b"").unwrap();
            assert!(head.starts_with(b"HTTP/1.1 204"));
            assert!(http::body(&head).is_empty());
            let cached = request(port, "/empty?not-modified", b"");
            assert!(cached.starts_with(b"HTTP/1.1 304"));
            assert!(
                String::from_utf8_lossy(&cached)
                    .to_ascii_lowercase()
                    .contains("etag: \"unchanged\"")
            );
            assert!(http::body(&cached).is_empty());
            assert_eq!(
                std::fs::read_to_string(&effect_file)
                    .unwrap()
                    .lines()
                    .count(),
                7
            );
            // A complete body is mandatory even if the peer closes or stalls.
            // Errors remain visible cases; they cannot produce an origin effect.
            if let Ok(wire) = try_request(port, "/api", &[7u8; 1025]) {
                assert!(wire.starts_with(b"HTTP/1.1 503"));
            }
            let mut aborted = TcpStream::connect(("127.0.0.1", port)).unwrap();
            aborted
                .write_all(
                    b"POST /api HTTP/1.1\r\nHost: example.test\r\nContent-Length: 1000\r\n\r\nx",
                )
                .unwrap();
            aborted.shutdown(Shutdown::Both).unwrap();
            drop(aborted);
            let mut stalled = TcpStream::connect(("127.0.0.1", port)).unwrap();
            stalled
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stalled
                .write_all(
                    b"POST /api HTTP/1.1\r\nHost: example.test\r\nContent-Length: 1000\r\n\r\nx",
                )
                .unwrap();
            let began = std::time::Instant::now();
            let outcome = http::receive(&mut stalled);
            assert!(began.elapsed() < Duration::from_secs(3));
            if let Ok(wire) = outcome {
                assert!(wire.starts_with(b"HTTP/1.1 503"));
            }
            drop(stalled);
            let mut negative_cases = 3;
            if framework == "express" {
                for wire in [b"GET /api HTTP/1.1\r\nHost: example.test\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\r\n".as_slice(),b"CONNECT example.test:443 HTTP/1.1\r\nHost: example.test\r\n\r\n".as_slice()]{let mut socket=TcpStream::connect(("127.0.0.1",port)).unwrap();socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();socket.write_all(wire).unwrap();if let Ok(response)=http::receive(&mut socket){assert!(response.starts_with(b"HTTP/1.1 503")||response.starts_with(b"HTTP/1.1 405")||response.starts_with(b"HTTP/1.1 400"));}negative_cases+=1;}
            }
            assert_eq!(
                std::fs::read_to_string(&effect_file)
                    .unwrap()
                    .lines()
                    .count(),
                7
            );
            drop(host);
            service.abort();
            runtime.shutdown_timeout(Duration::from_secs(2));
            let log = Log::open(
                Wal::open(&directory.0, WalOptions::default()).unwrap(),
                Config {
                    node_id: "node-synthetic-pilot".into(),
                    hmac_key: vec![0xaa; 32],
                    signing_seed: [0xbb; 32],
                    checkpoint_every: 1,
                    witness: None,
                },
                None,
                false,
            )
            .unwrap();
            assert_eq!(log.records().unwrap().len(), 7);
            println!(
                "{}",
                json!({"runtime":runtime_name,"framework":framework,"authority":"real-engine-synthetic-evidence","auditRecords":7,"originEffects":7,"allowCases":7,"negativeCases":negative_cases,"binaryBody":"preserved","sse":"preserved","asyncIterator":"preserved","fullProduct":false})
            );
        }
    }
}
fn request(port: u16, path: &str, body: &[u8]) -> Vec<u8> {
    try_request(port, path, body).unwrap()
}
fn try_request(port: u16, path: &str, body: &[u8]) -> std::io::Result<Vec<u8>> {
    try_request_method(port, "POST", path, body)
}
fn try_request_method(
    port: u16,
    method: &str,
    path: &str,
    body: &[u8],
) -> std::io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(stream,"{method} {path} HTTP/1.1\r\nHost: example.test\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nX-Order-B: first\r\nx-order-a: second\r\nX-Order-B: third\r\nAuthorization: Bearer application-fixture\r\nX-Forwarded-For: 127.0.0.1\r\nX-Humanymous-Verdict: ALLOW\r\nConnection: close\r\n\r\n",body.len()).unwrap();
    stream.write_all(body).unwrap();
    let response = http::receive(&mut stream)?;
    let _ = stream.shutdown(Shutdown::Both);
    Ok(response)
}

#[test]
#[ignore = "explicit actual Node/Bun host pilot; requires built addon and installed upstream modules"]
fn actual_framework_failure_paths_never_reach_origin() {
    let endpoint = if cfg!(windows) {
        r"\\.\pipe\humanymous-intentionally-missing"
    } else {
        "/tmp/humanymous-intentionally-missing/authority.sock"
    };
    let server = if cfg!(windows) {
        json!({"kind":"windows","sid":"S-1-5-18","pid":1})
    } else {
        json!({"kind":"unix","uid":0})
    };
    for runtime in ["node", "bun"] {
        for framework in ["express", "hono"] {
            let (mut host, port) = process(
                runtime,
                framework,
                endpoint,
                &"a".repeat(64),
                server.clone(),
            );
            for path in ["/api", "/_humanymous/admin", "/unknown"] {
                let wire = try_request(port, path, &[0, 255, 42]).unwrap_or_else(|error| {
                    let _ = host.0.kill();
                    let _ = host.0.wait();
                    let mut log = String::new();
                    host.0
                        .stderr
                        .take()
                        .unwrap()
                        .read_to_string(&mut log)
                        .unwrap();
                    panic!("{runtime}/{framework}{path}: {error}; {log}");
                });
                let head = String::from_utf8_lossy(&wire);
                if !head.starts_with("HTTP/1.1 503") {
                    let _ = host.0.kill();
                    let _ = host.0.wait();
                    let mut errors = String::new();
                    host.0
                        .stderr
                        .take()
                        .unwrap()
                        .read_to_string(&mut errors)
                        .unwrap();
                    panic!("{runtime}/{framework}: {head}; host stderr: {errors}");
                }
                assert!(!head.contains("x-origin-effect"));
            }
            println!(
                "{}",
                json!({"runtime":runtime,"framework":framework,"cases":3,"status":"passed","authority":"unavailable-negative","originEffects":0})
            );
        }
    }
}
