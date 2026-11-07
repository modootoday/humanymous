use crate::{Report, check_output, check_source};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const SHA: &str = "ba2b23f0e1639f92ccefbb882f43092bfe5b16b7";

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn execute(program: &str, args: &[&str], cwd: &Path) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env("BUNDLE_FROZEN", "true")
        .status()
        .map_err(|e| format!("{program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed: {status}"))
    }
}

pub(crate) fn validate(root: &Path, destination: &Path, baseurl: &str) -> Result<Report, String> {
    crate::output::validate_baseurl(baseurl)?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if !destination.is_absolute()
        || destination
            .components()
            .any(|c| matches!(c, Component::ParentDir))
        || destination.parent().is_none()
        || root.starts_with(destination)
        || destination.starts_with(&root)
    {
        return Err(
            "destination must be an absolute dedicated path outside the authored source".into(),
        );
    }
    if destination.exists() {
        let resolved = destination.canonicalize().map_err(|e| e.to_string())?;
        if resolved.starts_with(&root) || root.starts_with(&resolved) {
            return Err("destination resolves into or above the authored source".into());
        }
    }
    let source = check_source(&root)?;
    if !source.errors.is_empty() {
        return Ok(source);
    }
    let config = fs::read_to_string(root.join("_config.yml")).map_err(|e| e.to_string())?;
    if !config
        .lines()
        .any(|s| s.trim() == format!("remote_theme: modootoday/jekyll-theme-modootoday@{SHA}"))
    {
        return Err("remote_theme must use the reviewed v1.5.0 commit".into());
    }
    if !root.join("Gemfile.lock").is_file() {
        return Err("locked Ruby bundle required".into());
    }
    execute("ruby", &["-S", "bundle", "check"], &root)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temporary = Temporary(
        std::env::temp_dir().join(format!("humanymous-pages-{}-{nonce}", std::process::id())),
    );
    fs::create_dir(&temporary.0).map_err(|e| e.to_string())?;
    let theme = temporary.0.join("theme");
    let theme_arg = theme.to_str().ok_or("theme path must be UTF-8")?;
    execute(
        "git",
        &[
            "clone",
            "--quiet",
            "--depth",
            "1",
            "--branch",
            "v1.5.0",
            "https://github.com/modootoday/jekyll-theme-modootoday.git",
            theme_arg,
        ],
        &root,
    )?;
    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&theme)
        .output()
        .map_err(|e| e.to_string())?;
    if !revision.status.success() || String::from_utf8_lossy(&revision.stdout).trim() != SHA {
        return Err("theme tag does not resolve to the reviewed commit".into());
    }
    let destination = destination.to_str().ok_or("destination must be UTF-8")?;
    execute(
        "ruby",
        &[
            "-S",
            "bundle",
            "exec",
            "jekyll",
            "build",
            "--destination",
            destination,
            "--baseurl",
            baseurl,
            "--strict_front_matter",
        ],
        &root,
    )?;
    let checker = theme.join("exe/modootoday-check");
    execute(
        "ruby",
        &[
            checker.to_str().ok_or("checker path must be UTF-8")?,
            ".",
            destination,
            "--strict",
        ],
        &root,
    )?;
    check_output(&root, Path::new(destination), baseurl)
}
