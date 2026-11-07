//! Bounded subprocess transport; never invokes a shell or arbitrary script.
use serde_json::{Value, json};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub struct Docker {
    pub root: PathBuf,
    executable: PathBuf,
    pub transcript: Vec<Value>,
}
pub struct Output {
    pub success: bool,
    pub stdout: String,
}
impl Docker {
    pub fn new(root: &Path) -> Self {
        let windows = PathBuf::from("C:/Program Files/Docker/Docker/resources/bin/docker.exe");
        Self {
            root: root.into(),
            executable: if cfg!(windows) && windows.is_file() {
                windows
            } else {
                "docker".into()
            },
            transcript: Vec::new(),
        }
    }
    pub fn invoke(
        &mut self,
        args: &[String],
        environment: &[(String, String)],
        required: bool,
    ) -> Result<Output> {
        let mut command = Command::new(&self.executable);
        command
            .args(args)
            .current_dir(&self.root)
            .envs(environment.iter().cloned());
        if let Some(parent) = self
            .executable
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            let mut paths = vec![parent.to_path_buf()];
            paths.extend(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            ));
            command.env("PATH", std::env::join_paths(paths)?);
        }
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()?;
        let stdout = drain(child.stdout.take().ok_or("stdout unavailable")?);
        let stderr = drain(child.stderr.take().ok_or("stderr unavailable")?);
        let deadline = Instant::now() + Duration::from_secs(600);
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill()?;
                let _ = child.wait();
                return Err("Docker command deadline exceeded".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let stdout = stdout.join().map_err(|_| "stdout reader failed")??;
        let stderr = stderr.join().map_err(|_| "stderr reader failed")??;
        self.transcript
            .push(json!({"argv":args,"exit":status.code(),"stdout":stdout,"stderr":stderr}));
        if required && !status.success() {
            return Err(format!(
                "Docker {} failed, exit {:?}; see private transcript",
                args.first().map_or("", String::as_str),
                status.code()
            )
            .into());
        }
        Ok(Output {
            success: status.success(),
            stdout,
        })
    }
    pub fn call(&mut self, args: &[&str]) -> Result<String> {
        Ok(self
            .invoke(
                &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
                &[],
                true,
            )?
            .stdout)
    }
}
fn drain(input: impl Read + Send + 'static) -> std::thread::JoinHandle<std::io::Result<String>> {
    std::thread::spawn(move || {
        let mut retained = Vec::new();
        let mut input = input;
        let mut buffer = [0; 8192];
        loop {
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            if retained.len() + count <= 4 * 1024 * 1024 {
                retained.extend_from_slice(&buffer[..count]);
            }
        }
        Ok(String::from_utf8_lossy(&retained).into_owned())
    })
}
