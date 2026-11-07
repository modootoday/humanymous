//! Native service managers; commands are argument arrays, never executable scripts.
use super::{Generation, Result};
use crate::release::{Descriptor, error};
use serde::{Deserialize, Serialize};
#[cfg(not(windows))]
use std::process::{Command, Stdio};
use std::{
    io::{Read, Write},
    path::Path,
    time::{Duration, Instant},
};

/// Native registration evidence retained to detect unexpected manager changes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Identity {
    pub configuration: String,
    pub service_sid: Option<String>,
}

pub(crate) trait Manager {
    fn absent(&self, descriptor: &Descriptor) -> Result<()>;
    fn register(
        &self,
        generation: &Generation,
        root: &Path,
        descriptor: &Descriptor,
    ) -> Result<Identity>;
    fn verify(&self, generation: &Generation, root: &Path, descriptor: &Descriptor) -> Result<()>;
    fn start(&self, descriptor: &Descriptor) -> Result<()>;
    fn running(&self, descriptor: &Descriptor) -> Result<bool>;
    fn unregister(&self, descriptor: &Descriptor) -> Result<()>;
}

pub(crate) struct Native;

/// Generate declarative OS configuration, without executing the artifact.
pub(crate) fn definition(d: &Descriptor, executable: &Path) -> Result<Vec<u8>> {
    let executable = executable
        .to_str()
        .ok_or_else(|| error("UTF-8 service executable required"))?;
    if executable.chars().any(char::is_control) {
        return Err(error("service path control character"));
    }
    match d.os.as_str() {
        "windows" => Ok(Vec::new()),
        "linux" => {
            let quote = |s: &str| {
                format!(
                    "\"{}\"",
                    s.replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('%', "%%")
                        .replace('$', "$$")
                )
            };
            let mut argv = vec![quote(executable)];
            argv.extend(d.arguments.iter().map(|a| quote(a)));
            Ok(format!("[Unit]\nDescription=Humanymous owned lab service\n[Service]\nType=exec\nExecStart={}\nRestart=no\nNoNewPrivileges=yes\nUMask=0077\nTimeoutStopSec=5\n", argv.join(" ")).into_bytes())
        }
        "macos" => {
            let xml = |s: &str| {
                s.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
                    .replace('\'', "&apos;")
            };
            let mut argv = vec![executable.to_owned()];
            argv.extend(d.arguments.clone());
            Ok(format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\"><dict><key>Label</key><string>{}</string><key>ProgramArguments</key><array>{}</array><key>RunAtLoad</key><true/><key>KeepAlive</key><false/><key>ProcessType</key><string>Background</string><key>ExitTimeOut</key><integer>5</integer></dict></plist>\n", xml(&d.service_name), argv.iter().map(|a| format!("<string>{}</string>", xml(a))).collect::<String>()).into_bytes())
        }
        _ => Err(error("unsupported native service manager")),
    }
}

#[cfg(not(windows))]
fn command(arguments: &[&str]) -> Result<(bool, String)> {
    let executable = if cfg!(target_os = "macos") {
        "/bin/launchctl"
    } else {
        "/usr/bin/systemctl"
    };
    let mut child = Command::new(executable)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let output = child
        .stdout
        .take()
        .ok_or_else(|| error("manager output missing"))?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        output.take(16_385).read_to_end(&mut bytes).map(|_| bytes)
    });
    let until = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(error("native service manager timeout"));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let bytes = reader
        .join()
        .map_err(|_| error("manager reader failed"))??;
    if bytes.len() > 16_384 {
        return Err(error("native manager output budget"));
    }
    Ok((
        status.success(),
        String::from_utf8(bytes).map_err(|_| error("manager UTF-8"))?,
    ))
}

#[cfg(not(windows))]
fn require(arguments: &[&str]) -> Result<()> {
    if !command(arguments)?.0 {
        return Err(error(format!(
            "native manager refused {}",
            arguments.first().unwrap_or(&"operation")
        )));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
impl Manager for Native {
    fn absent(&self, d: &Descriptor) -> Result<()> {
        let (_, state) = command(&[
            "--user",
            "--no-ask-password",
            "show",
            &format!("{}.service", d.service_name),
            "--property=LoadState",
            "--value",
        ])?;
        if state.trim() != "not-found" {
            return Err(error("service exists or user manager unavailable"));
        }
        Ok(())
    }
    fn register(&self, g: &Generation, root: &Path, d: &Descriptor) -> Result<Identity> {
        self.absent(d)?;
        let path = g.definition_path(root, d);
        let path = path.to_str().ok_or_else(|| error("unit path"))?;
        require(&["--user", "--no-ask-password", "link", "--runtime", path])?;
        require(&["--user", "--no-ask-password", "daemon-reload"])?;
        Ok(Identity {
            configuration: path.into(),
            service_sid: None,
        })
    }
    fn verify(&self, g: &Generation, root: &Path, d: &Descriptor) -> Result<()> {
        let expected = g.definition_path(root, d).to_string_lossy().into_owned();
        let (success, path) = command(&[
            "--user",
            "--no-ask-password",
            "show",
            &format!("{}.service", d.service_name),
            "--property=FragmentPath",
            "--value",
        ])?;
        if !success
            || path.trim() != expected
            || g.identity
                .as_ref()
                .is_none_or(|i| i.configuration != expected)
        {
            return Err(error("native unit registration changed"));
        }
        Ok(())
    }
    fn start(&self, d: &Descriptor) -> Result<()> {
        require(&[
            "--user",
            "--no-ask-password",
            "start",
            &format!("{}.service", d.service_name),
        ])
    }
    fn running(&self, d: &Descriptor) -> Result<bool> {
        Ok(command(&[
            "--user",
            "--no-ask-password",
            "is-active",
            &format!("{}.service", d.service_name),
        ])?
        .0)
    }
    fn unregister(&self, d: &Descriptor) -> Result<()> {
        require(&[
            "--user",
            "--no-ask-password",
            "stop",
            &format!("{}.service", d.service_name),
        ])?;
        require(&[
            "--user",
            "--no-ask-password",
            "disable",
            "--runtime",
            &format!("{}.service", d.service_name),
        ])?;
        require(&["--user", "--no-ask-password", "daemon-reload"])
    }
}

#[cfg(target_os = "macos")]
fn domain() -> String {
    format!("gui/{}", humanymous_runtime::native_security::process::current_uid())
}
#[cfg(target_os = "macos")]
impl Manager for Native {
    fn absent(&self, d: &Descriptor) -> Result<()> {
        if !command(&["print", &domain()])?.0 {
            return Err(error("user launchd domain unavailable"));
        }
        if command(&["print", &format!("{}/{}", domain(), d.service_name)])?.0 {
            return Err(error("launchd job already exists"));
        }
        Ok(())
    }
    fn register(&self, g: &Generation, root: &Path, d: &Descriptor) -> Result<Identity> {
        self.absent(d)?;
        let path = g.definition_path(root, d).to_string_lossy().into_owned();
        require(&["bootstrap", &domain(), &path])?;
        Ok(Identity {
            configuration: path,
            service_sid: None,
        })
    }
    fn verify(&self, g: &Generation, root: &Path, d: &Descriptor) -> Result<()> {
        let (success, output) = command(&["print", &format!("{}/{}", domain(), d.service_name)])?;
        let path = g.definition_path(root, d).to_string_lossy().into_owned();
        if !success
            || !output
                .lines()
                .any(|line| line.trim() == format!("path = {path}"))
            || g.identity.as_ref().is_none_or(|i| i.configuration != path)
        {
            return Err(error("launchd registration changed"));
        }
        Ok(())
    }
    fn start(&self, d: &Descriptor) -> Result<()> {
        require(&["kickstart", &format!("{}/{}", domain(), d.service_name)])
    }
    fn running(&self, d: &Descriptor) -> Result<bool> {
        let (success, out) = command(&["print", &format!("{}/{}", domain(), d.service_name)])?;
        Ok(success && out.lines().any(|line| line.trim() == "state = running"))
    }
    fn unregister(&self, d: &Descriptor) -> Result<()> {
        require(&["bootout", &format!("{}/{}", domain(), d.service_name)])
    }
}

#[cfg(windows)]
mod windows;

pub(crate) fn ready(d: &Descriptor) -> Result<()> {
    // This is a diagnostic challenge, never a management authorization channel.
    let mut stream =
        std::net::TcpStream::connect_timeout(&d.readiness, Duration::from_millis(200))?;
    stream.set_read_timeout(Some(Duration::from_millis(200)))?;
    stream.set_write_timeout(Some(Duration::from_millis(200)))?;
    stream.write_all(b"HUMANYMOUS-READY/1\n")?;
    let mut bytes = Vec::new();
    stream.take(514).read_to_end(&mut bytes)?;
    if bytes != d.ready_response.as_bytes() {
        return Err(error("service readiness response mismatch"));
    }
    Ok(())
}

pub(crate) fn wait_ready(manager: &impl Manager, d: &Descriptor) -> Result<()> {
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        if manager.running(d)? && ready(d).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Err(error("service did not become diagnostically ready"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declarative_configs_escape_arguments_without_a_shell() {
        let mut d = crate::release::tests::descriptor(1);
        d.os = "linux".into();
        d.arguments = vec!["a\"$VAR%name\\x".into()];
        let text = String::from_utf8(definition(&d, Path::new("/private/root/artifact")).unwrap())
            .unwrap();
        assert!(text.contains("a\\\"$$VAR%%name\\\\x"));
        assert!(!text.contains("/bin/sh"));
        d.os = "macos".into();
        d.arguments = vec!["<&>".into()];
        let text = String::from_utf8(definition(&d, Path::new("/private/root/artifact")).unwrap())
            .unwrap();
        assert!(text.contains("&lt;&amp;&gt;"));
    }
}
