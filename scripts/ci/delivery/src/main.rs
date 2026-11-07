//! Source-bound delivery validation. No authored script-language implementation.
mod archive_cache;
mod command;
mod components;
mod dependency_summary;
mod docker_source;
mod evidence;
mod go_product;
mod image_candidates;
mod image_scan;
mod lab_diagnostics;
mod lab_receipts;
mod metrics;
mod mount_probe;
mod nginx;
mod package;
mod pages;
mod pilot;
mod policy;
mod registry;
mod remote;
mod source;
mod supply;
mod workspace;

use std::path::{Path, PathBuf};
pub type Result<T> = std::result::Result<T, String>;

pub fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
pub fn read_json(path: &Path) -> Result<serde_json::Value> {
    serde_json::from_slice(&std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| e.to_string())
}
pub fn write_json(path: &Path, value: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn run() -> Result<()> {
    let location = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| e.to_string())?;
    require(
        location.status.success(),
        "delivery binary requires its actual Git checkout",
    )?;
    let root = PathBuf::from(
        String::from_utf8(location.stdout)
            .map_err(|e| e.to_string())?
            .trim(),
    );
    require(
        root.join(".github/workflows/rust-validation.yml").is_file(),
        "delivery checkout contract missing",
    )?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    let argument = |index: usize| {
        args.get(index)
            .map(String::as_str)
            .ok_or_else(|| "missing argument".to_string())
    };
    match argument(0)? {
        "policy" => policy::check(&root),
        "component" => components::run(&root, argument(1)?),
        "native-component" => components::native(&root, argument(1)?),
        "action-pins" => policy::pins(&root),
        "jobs" => policy::jobs(&std::env::var("NEEDS_JSON").map_err(|e| e.to_string())?),
        "package" => package::check(&root),
        "image-sources" => package::image_sources(&root),
        "image-candidates" => image_candidates::check(&root),
        "image-scan" => image_scan::run(&root, argument(1)?),
        "pages-evidence" => pages::run(&root, argument(1)?),
        "go-product" => go_product::run(&root),
        "mount-probe" => mount_probe::run(&root),
        "native-artifacts" => package::native_artifacts(&root),
        "node" => package::node(&root).map(|_| ()),
        "readiness" => registry::readiness(&root),
        "release" => registry::release(&root),
        "crate-waves" => registry::wave_output(&root),
        "publish-wave" => registry::publish_wave(&root, argument(1)?.parse::<usize>().map_err(|e| e.to_string())?),
        "publish-npm" => registry::publish_npm(&root),
        "pilot" => pilot::run(&root, argument(1)?),
        "snapshot" => { println!("{}", source::snapshot(&root)?); Ok(()) },
        "observe-run" => remote::observe(&root, argument(1)?, argument(2)?, Path::new(argument(3)?)),
        "audit-closure" => package::audit_closure(&root),
        "archive-cache" => archive_cache::run(&root),
        "register-evidence" => evidence::register(Path::new(argument(1)?), argument(2)?),
        "verify-images" => supply::verify_images(&root),
        "probe" => workspace::run(&root,argument(1)?,argument(2)?),
        "nginx-report" => evidence::emit_public_summary("Source-bound nginx scenario groups", &nginx::validate(&read_json(Path::new(argument(1)?))?, argument(2)?, argument(3)?, argument(4)?)?),
        "nginx" => nginx::run(&root),
        _ => Err("usage: humanymous-delivery policy|action-pins|jobs|package|image-sources|native-artifacts|node|readiness|release|crate-waves|publish-wave N|publish-npm|pilot native|browser|browser-product|docker|snapshot|observe-run ID SHA OUTPUT|audit-closure".into()),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("delivery blocked: {error}");
        std::process::exit(1);
    }
}
