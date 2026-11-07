//! Literal argument subprocesses, never shell source or runtime script strings.
use crate::Result;
use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

pub fn output(root: &Path, program: &str, args: &[&str]) -> Result<Output> {
    Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot execute {program}: {e}"))
}
pub fn text(root: &Path, program: &str, args: &[&str]) -> Result<String> {
    let result = output(root, program, args)?;
    if !result.status.success() {
        return Err(format!(
            "{program} failed ({:?}): {}",
            result.status.code(),
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    String::from_utf8(result.stdout).map_err(|e| e.to_string())
}
pub fn status(root: &Path, program: &str, args: &[&str]) -> Result<()> {
    let result = Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    crate::require(
        result.success(),
        format!("{program} failed: {:?}", result.code()),
    )
}
pub fn log(root: &Path, program: &str, args: &[&str], destination: &Path) -> Result<i32> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = std::fs::File::create(destination).map_err(|e| e.to_string())?;
    let error = file.try_clone().map_err(|e| e.to_string())?;
    let result = Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(error)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(result.code().unwrap_or(-1))
}
pub fn json_log(root: &Path, program: &str, args: &[&str], destination: &Path) -> Result<i32> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let output = std::fs::File::create(destination).map_err(|e| e.to_string())?;
    let error = std::fs::File::create(destination.with_extension("stderr.log"))
        .map_err(|e| e.to_string())?;
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(output)
        .stderr(error)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(-1))
}
