//! Hosted component pilot: actual httpd plus shared authority/WAL and an owned
//! effect witness. Frozen synthetic ALLOW evidence is never product provenance.
mod authority;
mod origin;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

pub type Error = Box<dyn std::error::Error>;
pub const ROUTES: &[&str] = &[
    "/echo",
    "/deny",
    "/challenge",
    "/production",
    "/slow",
    "/capture",
    "/cancel",
    "/probe-subrequest",
    "/probe-redirect",
];
const CASES: &[&str] = &[
    "binary-body-replay",
    "spoofed-provenance",
    "subrequest-rejection",
    "internal-redirect-rejection",
    "cache-bypass-denial",
    "deny",
    "challenge",
    "required-capture",
    "unknown-route",
    "request-cancellation",
    "graceful-reload",
    "deadline-expiry",
    "origin-witness-denial",
    "authority-outage",
    "cleanup",
];
struct Server {
    child: Child,
    config: std::path::PathBuf,
    stopped: bool,
}
struct RequestWorker(Option<std::thread::JoinHandle<Result<u16, &'static str>>>);
impl RequestWorker {
    fn join(&mut self) -> Result<u16, Error> {
        self.0
            .take()
            .ok_or("request worker already joined")?
            .join()
            .map_err(|_| "inflight worker panic")?
            .map_err(Into::into)
    }
}
impl Drop for RequestWorker {
    fn drop(&mut self) {
        // Every early return waits for the owned worker before server/authority
        // resources can be torn down. Connect/read/write each have fixed bounds.
        if let Some(worker) = self.0.take() {
            let _ = worker.join();
        }
    }
}
impl Server {
    fn children(&self) -> Result<Vec<u32>, Error> {
        let pid = self.child.id();
        let text = std::fs::read_to_string(format!("/proc/{pid}/task/{pid}/children"))?;
        text.split_whitespace()
            .map(|value| value.parse().map_err(Into::into))
            .collect()
    }
    fn reload(&mut self) -> Result<(), Error> {
        let before = self.children()?;
        ensure(!before.is_empty(), "MPM children required")?;
        let status = Command::new("/opt/httpd/bin/httpd")
            .arg("-f")
            .arg(&self.config)
            .args(["-k", "graceful"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        ensure(status.success(), "graceful reload command")?;
        let until = Instant::now() + Duration::from_secs(4);
        while Instant::now() < until {
            let after = self.children()?;
            if !after.is_empty() && after.iter().all(|pid| !before.contains(pid)) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err("new MPM generation not observed".into())
    }
    fn stop(&mut self) -> Result<(), Error> {
        if self.stopped {
            return Ok(());
        }
        let status = Command::new("/opt/httpd/bin/httpd")
            .arg("-f")
            .arg(&self.config)
            .args(["-k", "stop"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        ensure(status.success(), "httpd stop command")?;
        let until = Instant::now() + Duration::from_secs(3);
        while Instant::now() < until {
            if let Some(status) = self.child.try_wait()? {
                self.stopped = true;
                return ensure(status.success(), "httpd stop status");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err("httpd stop deadline".into())
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if !self.stopped && self.stop().is_err() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
fn ensure(value: bool, label: &'static str) -> Result<(), Error> {
    if value { Ok(()) } else { Err(label.into()) }
}
fn hash(path: &Path) -> Result<String, Error> {
    Ok(format!("{:x}", Sha256::digest(std::fs::read(path)?)))
}
fn source_value(value: Option<&str>, size: usize) -> Result<String, Error> {
    let value = value.ok_or("source identity must be embedded at build time")?;
    ensure(
        value.len() == size && value.bytes().all(|b| b.is_ascii_hexdigit()),
        "source identity required",
    )?;
    Ok(value.to_owned())
}
fn request(
    port: u16,
    path: &str,
    body: &[u8],
    extra: &str,
) -> Result<(u16, Vec<u8>, String), Error> {
    let mut stream = TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_secs(1),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: example.test\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    let mut bytes = Vec::new();
    stream.take(2_097_153).read_to_end(&mut bytes)?;
    ensure(bytes.len() <= 2_097_152, "response bound")?;
    let split = bytes
        .windows(4)
        .position(|v| v == b"\r\n\r\n")
        .ok_or("response framing")?;
    let header = std::str::from_utf8(&bytes[..split])?.to_owned();
    let status = header
        .split_whitespace()
        .nth(1)
        .ok_or("response status")?
        .parse()?;
    Ok((status, bytes[split + 4..].to_vec(), header))
}
fn startup(
    base: &Path,
    mpm: &str,
    origin: u16,
    private: &Path,
    cache: bool,
    probes: bool,
) -> Result<(Server, u16), Error> {
    let lease = TcpListener::bind(("127.0.0.1", 0))?;
    let port = lease.local_addr()?.port();
    drop(lease);
    let path = base.join("httpd.conf");
    let uid = rustix::process::geteuid().as_raw();
    let gid = rustix::process::getegid().as_raw();
    // Apache configuration only; no authored executable foreign host glue.
    std::fs::write(
        &path,
        format!(
            "ServerRoot /opt/httpd\nDefaultRuntimeDir {base}\nServerName example.test\nListen 127.0.0.1:{port}\nLoadModule mpm_{mpm}_module modules/mod_mpm_{mpm}.so\nLoadModule unixd_module modules/mod_unixd.so\nLoadModule authz_core_module modules/mod_authz_core.so\nLoadModule proxy_module modules/mod_proxy.so\nLoadModule proxy_http_module modules/mod_proxy_http.so\nLoadModule humanymous_module modules/libhumanymous_apache_native.so\nUser #{uid}\nGroup #{gid}\nPidFile {base}/httpd.pid\nErrorLog {base}/httpd-error.log\nLogLevel error\nTimeout 2\nKeepAlive Off\nHumanymousConfig {private}\nProxyRequests Off\nProxyAddHeaders Off\nProxyPass / http://127.0.0.1:{origin}/\n<Location />\nRequire all granted\n</Location>\n",
            base = base.display(),
            private = private.display()
        ),
    )?;
    if probes {
        let config = std::fs::read_to_string(&path)?;
        std::fs::write(
            &path,
            config.replace(
                "libhumanymous_apache_native.so",
                "libhumanymous_apache_probe.so",
            ),
        )?;
    }
    if cache {
        let mut file = std::fs::OpenOptions::new().append(true).open(&path)?;
        file.write_all(b"LoadModule cache_module modules/mod_cache.so\n")?;
        let output = Command::new("/opt/httpd/bin/httpd")
            .arg("-f")
            .arg(&path)
            .arg("-M")
            .output()?;
        ensure(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("cache_module"),
            "cache module graph observed",
        )?;
    }
    let modules = Command::new("/opt/httpd/bin/httpd")
        .arg("-f")
        .arg(&path)
        .arg("-M")
        .output()?;
    ensure(
        modules.status.success()
            && String::from_utf8_lossy(&modules.stdout).contains(&format!("mpm_{mpm}_module")),
        "selected MPM observed",
    )?;
    let child = Command::new("/opt/httpd/bin/httpd")
        .arg("-f")
        .arg(&path)
        .arg("-DFOREGROUND")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut server = Server {
        child,
        config: path,
        stopped: false,
    };
    let until = Instant::now() + Duration::from_secs(4);
    while Instant::now() < until {
        if server.child.try_wait()?.is_some() {
            return Err("httpd startup failed".into());
        }
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok((server, port));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("httpd readiness expired".into())
}
fn case(
    cases: &mut Vec<serde_json::Value>,
    mpm: &str,
    name: &'static str,
    test: impl FnOnce() -> Result<(), Error>,
) {
    let result = test();
    if let Err(error) = &result {
        let label = crate::coordinator::case_failure_label(&error.to_string());
        eprintln!("APACHE_CASE_FAILURE {mpm}/{name} {label}");
    }
    cases.push(serde_json::json!({"id":format!("{mpm}/{name}"),"mpm":mpm,"name":name,"result":if result.is_ok(){"passed"}else{"failed"}}));
}
fn suite(mpm: &str, cases: &mut Vec<serde_json::Value>) -> Result<(), Error> {
    let base = std::env::temp_dir().join(format!(
        "h-apache-{}",
        humanymous_framework_adapter::uuid()?
    ));
    std::fs::create_dir(&base)?;
    std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o700))?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let mut service = runtime.block_on(authority::start(&base))?;
    let origin = origin::Origin::start()?;
    let private = base.join("private.json");
    let config = serde_json::json!({"installation":"apache","tenant":"t","origin":"o","revision":"r","endpoint":base.join("decision.sock"),
        "credential":service.credential,"server":{"kind":"unix","uid":rustix::process::geteuid().as_raw()},"routes":ROUTES,
        "timeoutMs":600,"maxBody":1048576,"syncInputDeadlineEnforced":true});
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&private)?;
    file.write_all(&serde_json::to_vec(&config)?)?;
    file.sync_all()?;
    drop(file);
    let (mut server, port) = startup(&base, mpm, origin.port, &private, false, false)?;
    case(cases, mpm, "binary-body-replay", || {
        let before = origin.count.load(Ordering::Acquire);
        let body = [0, 255, 128, 10];
        let (status, observed, _) = request(port, "/echo", &body, "")?;
        ensure(
            status == 200 && observed == body && origin.count.load(Ordering::Acquire) == before + 1,
            "binary replay/effect",
        )?;
        std::thread::scope(|scope| -> Result<(), Error> {
            let workers: Vec<_> = (0u8..16)
                .map(|index| {
                    scope.spawn(move || {
                        let bytes = [index, 0, 255, 128];
                        request(port, "/echo", &bytes, "")
                            .map(|(status, body, _)| status == 200 && body == bytes)
                            .map_err(|_| "concurrent request")
                    })
                })
                .collect();
            for worker in workers {
                ensure(
                    worker
                        .join()
                        .map_err(|_| "concurrent worker panic")?
                        .map_err(|_| "concurrent request")?,
                    "concurrent body isolation",
                )?;
            }
            Ok(())
        })?;
        ensure(
            origin.count.load(Ordering::Acquire) == before + 17,
            "exact concurrent origin effects",
        )
    });
    case(cases, mpm, "spoofed-provenance", || {
        let (status, _, headers) = request(
            port,
            "/echo",
            b"exact",
            "X-Humanymous-Allow: forged\r\nX-Forwarded-For: forged\r\n",
        )?;
        ensure(
            status == 200
                && headers
                    .to_ascii_lowercase()
                    .contains("x-pilot-spoof-present: 0"),
            "spoof stripping",
        )
    });
    for (name, path, probe) in [
        ("subrequest-rejection", "/probe-subrequest", "subrequest"),
        ("internal-redirect-rejection", "/probe-redirect", "redirect"),
    ] {
        case(cases, mpm, name, || {
            let probe_base = base.join(format!("probe-{name}"));
            std::fs::create_dir(&probe_base)?;
            std::fs::set_permissions(&probe_base, std::fs::Permissions::from_mode(0o700))?;
            let (mut probe_server, probe_port) =
                startup(&probe_base, mpm, origin.port, &private, false, true)?;
            let before = origin.count.load(Ordering::Acquire);
            let (status, _, headers) = request(probe_port, path, b"", "")?;
            ensure(
                status != 200
                    && headers
                        .to_ascii_lowercase()
                        .contains(&format!("x-pilot-probe-ran: {probe}"))
                    && origin.count.load(Ordering::Acquire) == before,
                "actual rejected nested request",
            )?;
            probe_server.stop()
        });
    }
    case(cases, mpm, "cache-bypass-denial", || {
        let cachebase = base.join("cache");
        std::fs::create_dir(&cachebase)?;
        std::fs::set_permissions(&cachebase, std::fs::Permissions::from_mode(0o700))?;
        let (mut cached, cacheport) = startup(&cachebase, mpm, origin.port, &private, true, false)?;
        let before = origin.count.load(Ordering::Acquire);
        let (status, _, _) = request(cacheport, "/echo", b"", "")?;
        ensure(
            status == 503 && origin.count.load(Ordering::Acquire) == before,
            "cache graph must fail closed",
        )?;
        cached.stop()
    });
    for (name, path, status) in [
        ("deny", "/deny", 403),
        ("challenge", "/challenge", 403),
        ("required-capture", "/capture", 503),
        ("unknown-route", "/unknown", 503),
    ] {
        case(cases, mpm, name, || {
            let before = origin.count.load(Ordering::Acquire);
            let (actual, _, _) = request(port, path, b"", "")?;
            ensure(
                actual == status && origin.count.load(Ordering::Acquire) == before,
                "denied origin effect",
            )
        });
    }
    case(cases, mpm, "request-cancellation", || {
        let before = origin.count.load(Ordering::Acquire);
        let mut stream = TcpStream::connect(("127.0.0.1", port))?;
        stream.set_write_timeout(Some(Duration::from_secs(1)))?;
        stream.write_all(b"POST /cancel HTTP/1.1\r\nHost: example.test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")?;
        stream.shutdown(std::net::Shutdown::Both)?;
        drop(stream);
        std::thread::sleep(Duration::from_millis(450));
        ensure(
            origin.count.load(Ordering::Acquire) == before,
            "cancelled origin effect",
        )
    });
    case(cases, mpm, "graceful-reload", || {
        let pending = service.pending.load(Ordering::Acquire);
        let before = origin.count.load(Ordering::Acquire);
        let mut active = RequestWorker(Some(std::thread::spawn(move || {
            request(port, "/cancel", b"", "")
                .map(|reply| reply.0)
                .map_err(|_| "inflight response")
        })));
        let until = Instant::now() + Duration::from_millis(150);
        while service.pending.load(Ordering::Acquire) == pending && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(2));
        }
        ensure(
            service.pending.load(Ordering::Acquire) > pending,
            "authority observed inflight request before reload",
        )?;
        server.reload()?;
        ensure(
            active.join()? == 503 && origin.count.load(Ordering::Acquire) == before,
            "old generation must not reach origin",
        )?;
        let (status, body, _) = request(port, "/echo", b"new-generation", "")?;
        ensure(
            status == 200 && body == b"new-generation",
            "post-reload module request",
        )
    });
    case(cases, mpm, "deadline-expiry", || {
        let before = origin.count.load(Ordering::Acquire);
        let (status, _, _) = request(port, "/slow", b"", "")?;
        ensure(
            status == 503 && origin.count.load(Ordering::Acquire) == before,
            "expired origin effect",
        )
    });
    case(cases, mpm, "origin-witness-denial", || {
        let before = origin.count.load(Ordering::Acquire);
        let (status, _, _) = request(port, "/production", b"", "")?;
        ensure(
            status != 200 && origin.count.load(Ordering::Acquire) == before,
            "production missing capability",
        )
    });
    runtime.block_on(service.stop());
    drop(service);
    case(cases, mpm, "authority-outage", || {
        let before = origin.count.load(Ordering::Acquire);
        let (status, _, _) = request(port, "/echo", b"", "")?;
        ensure(
            status == 503 && origin.count.load(Ordering::Acquire) == before,
            "outage origin effect",
        )
    });
    case(cases, mpm, "cleanup", || server.stop());
    drop(server);
    drop(origin);
    drop(runtime);
    // The generated UUID path is an owned direct child of the canonical temp
    // directory. Failure artifacts are retained until the hosted job tears down.
    if !cases
        .iter()
        .any(|case| case["mpm"] == mpm && case["result"] == "failed")
    {
        ensure(
            base.parent() == Some(std::env::temp_dir().as_path()),
            "owned cleanup path",
        )?;
        std::fs::remove_dir_all(&base)?;
    }
    Ok(())
}
pub fn run() -> Result<(), Error> {
    ensure(
        std::env::var("HUMANYMOUS_APACHE_OWNED_DOCKER").as_deref() == Ok("1"),
        "owned hosted Docker marker required",
    )?;
    ensure(
        rustix::process::geteuid().as_raw() != 0,
        "non-root pilot required",
    )?;
    let source = source_value(option_env!("HUMANYMOUS_SOURCE_REV"), 40)?;
    let source_manifest = source_value(option_env!("HUMANYMOUS_SOURCE_MANIFEST_SHA256"), 64)?;
    let image = std::env::var("HUMANYMOUS_APACHE_IMAGE")?;
    ensure(
        image
            .strip_prefix("sha256:")
            .is_some_and(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())),
        "actual inspected image digest required",
    )?;
    let version = Command::new("/opt/httpd/bin/httpd").arg("-V").output()?;
    ensure(version.status.success(), "httpd version observation")?;
    let version = String::from_utf8(version.stdout)?;
    ensure(version.contains("Apache/2.4.70"), "pinned engine version")?;
    let mut cases = Vec::new();
    for mpm in ["event", "worker", "prefork"] {
        if let Err(error) = suite(mpm, &mut cases) {
            let label = crate::coordinator::case_failure_label(&error.to_string());
            eprintln!("APACHE_CASE_FAILURE {mpm}/infrastructure {label}");
            cases.push(serde_json::json!({"id":format!("{mpm}/infrastructure"),"mpm":mpm,"name":"suite-infrastructure","result":"failed"}));
        }
        for name in CASES {
            let id = format!("{mpm}/{name}");
            if !cases.iter().any(|case| case["id"] == id) {
                cases.push(serde_json::json!({"id":id,"mpm":mpm,"name":name,"result":"not_run"}));
            }
        }
    }
    let failed = cases
        .iter()
        .filter(|case| case["result"] == "failed")
        .count();
    let passed = cases
        .iter()
        .filter(|case| case["result"] == "passed")
        .count();
    let not_run = cases
        .iter()
        .filter(|case| case["result"] == "not_run")
        .count();
    let report = serde_json::json!({"schema":"humanymous.apache.pilot/0.1","status":if failed==0&&not_run==0{"passed"}else{"incomplete"},
        "qualification":"actual-httpd/shared-authority-WAL; synthetic frozen ALLOW fixture; component only",
        "sourceCommit":source,"sourceManifestSha256":source_manifest,"imageDigest":image,"moduleSha256":hash(Path::new("/opt/httpd/modules/libhumanymous_apache_native.so"))?,
        "probeModuleSha256":hash(Path::new("/opt/httpd/modules/libhumanymous_apache_probe.so"))?,
        "httpdSha256":hash(Path::new("/opt/httpd/bin/httpd"))?,"abiReceiptSha256":hash(Path::new("/opt/httpd/modules/apache-abi-production.json"))?,
        "lockSha256":hash(Path::new("/opt/httpd/source-Cargo.lock"))?,
        "engineVersionObservation":version,"cases":cases,"requiredScenarioCount":45,"passed":passed,"failed":failed,"not_run":not_run,
        "fullProductSuccess":false,"cleanupObserved":failed==0&&cases.iter().filter(|case|case["name"]=="cleanup"&&case["result"]=="passed").count()==3});
    println!("{}", serde_json::to_string(&report)?);
    ensure(
        failed == 0 && not_run == 0,
        "Apache pilot remains incomplete",
    )
}
