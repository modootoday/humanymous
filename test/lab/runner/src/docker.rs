//! Bounded subprocess transport; never invokes a shell or arbitrary script.
use humanymous_test_support::process::{ManagedChild, capture_with_stderr};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
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
    pub stderr: String,
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
        self.invoke_input(args, environment, required, b"")
    }
    pub fn command(&self, args: &[String], environment: &[(String, String)]) -> Result<Command> {
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
        Ok(command)
    }
    pub fn invoke_input(
        &mut self,
        args: &[String],
        environment: &[(String, String)],
        required: bool,
        input: &[u8],
    ) -> Result<Output> {
        let mut command = self.command(args, environment)?;
        let captured = capture_with_stderr(
            &mut command,
            input,
            Duration::from_secs(if input.is_empty() { 600 } else { 30 }),
            4 * 1024 * 1024,
        )?;
        let status = captured.status;
        let stdout = String::from_utf8(captured.stdout)?;
        let stderr = String::from_utf8_lossy(&captured.stderr).into_owned();
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
            stderr,
        })
    }
    pub fn duplex(
        &mut self,
        args: &[String],
    ) -> Result<(
        ManagedChild,
        std::process::ChildStdin,
        std::process::ChildStdout,
    )> {
        self.transcript
            .push(json!({"argv":args,"transport":"private-duplex","input_recorded":false}));
        Ok(ManagedChild::spawn_duplex(&mut self.command(args, &[])?)?)
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
