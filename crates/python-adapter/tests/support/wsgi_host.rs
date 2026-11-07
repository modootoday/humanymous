//! Owned Unix WSGI host lifecycle and bounded HTTP test transport.
use super::http;
use serde_json::json;
use std::{
    fs,
    io::Write,
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
#[path = "authority_environment.rs"]
mod environment;
pub(super) use environment::{Directory, configuration};
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
pub(super) fn host(
    directory: &Directory,
    framework: &str,
    endpoint: &str,
    credential: &str,
    asgi: bool,
) -> (Host, u16) {
    if asgi {
        super::component_lifecycle::require_safe_lifecycle()
            .expect("ASGI server lifecycle is not accepted");
        let version = humanymous_platform::testing::process::capture_with_stderr(
            Command::new(executable("HUMANYMOUS_UVICORN")).arg("--version"),
            b"",
            Duration::from_secs(5),
            1024,
        )
        .expect("bounded upstream server identity required");
        let text = std::str::from_utf8(&version.stdout).unwrap().trim();
        assert!(
            version.status.success()
                && text.starts_with("Running uvicorn 0.54.0 with CPython 3.12.")
                && text.ends_with("on Linux"),
            "pinned Linux CPython3.12 Uvicorn0.54.0 profile required"
        );
        fs::write(
            directory.0.join("server-version.private.txt"),
            &version.stdout,
        )
        .unwrap();
    }
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
    let port_arg = format!("--port={port}");
    let args = if asgi {
        vec![
            "--host=127.0.0.1",
            &port_arg,
            "--factory",
            "--workers=1",
            "--interface=asgi3",
            "--loop=asyncio",
            "--http=h11",
            "--ws=none",
            "--no-proxy-headers",
            "--no-access-log",
            "--limit-concurrency=8",
            "--timeout-graceful-shutdown=2",
            if framework == "django" {
                "--lifespan=off"
            } else {
                "--lifespan=on"
            },
            "humanymous_python:pilot_asgi",
        ]
    } else {
        vec![
            "--host=127.0.0.1",
            &port_arg,
            "--channel-request-lookahead=1",
            "--channel-timeout=2",
            "--cleanup-interval=1",
            "--max-request-body-size=2048",
            "--call",
            "humanymous_python:pilot_wsgi",
        ]
    };
    let mut command = Command::new(executable(if asgi {
        "HUMANYMOUS_UVICORN"
    } else {
        "HUMANYMOUS_WAITRESS_SERVE"
    }));
    if asgi {
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("UVICORN_") {
                command.env_remove(name);
            }
        }
        command.env_remove("WEB_CONCURRENCY");
    }
    let mut child = Host(
        command
            .args(args)
            .env("PYTHONPATH", module)
            .env("PYTHONNOUSERSITE", "1")
            .env("HUMANYMOUS_HOSTED_NATIVE_PILOT", "1")
            .env_remove("PYTHONSTARTUP")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(fs::File::create(directory.0.join("server-stderr.private.log")).unwrap())
            .spawn()
            .expect("actual provisioned server process required"),
    );
    let options = json!({"installation":"i","tenant":"t","origin":"o","revision":"r", "endpoint":endpoint,"credential":credential,
        "server":{"kind":"unix","uid":humanymous_runtime::native_security::process::current_uid()},"routes":["/api"],"timeoutMs":1000,"maxBody":1024,"syncInputDeadlineEnforced":!asgi});
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
