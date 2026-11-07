use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    net::SocketAddr,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

pub struct Runtime(Child);
impl Runtime {
    pub fn reject(config: &Value) {
        let mut child = Command::new("/usr/local/bin/humanymous-runtime")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        serde_json::to_writer(&mut input, config).unwrap();
        drop(input);
        let mut guard = Self(child);
        let until = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = guard.0.try_wait().unwrap() {
                assert!(!status.success(), "unsafe startup accepted");
                break;
            }
            assert!(std::time::Instant::now() < until, "refusal deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut output = String::new();
        std::io::Read::read_to_string(&mut guard.0.stdout.take().unwrap(), &mut output).unwrap();
        assert!(output.is_empty(), "refused process reported readiness");
    }
    pub fn start(config: &Value) -> (Self, SocketAddr) {
        let mut child = Command::new("/usr/local/bin/humanymous-runtime")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        serde_json::to_writer(&mut input, config).unwrap();
        input.flush().unwrap();
        drop(input);
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line);
            let _ = tx.send((result, line));
        });
        let guard = Self(child);
        let (result, line) = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("bounded readiness");
        assert!(result.unwrap() > 0, "runtime did not become ready");
        let ready: Value = serde_json::from_str(&line).unwrap();
        let address = ready["listen"].as_str().unwrap().parse().unwrap();
        (guard, address)
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
