//! Public-only deterministic archives. The caller owns a quiescent private tree.
//! This is neither a general tar extractor nor independent provenance attestation.
mod tar;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const MAX_FILES: usize = 4096;
const MAX_FILE: usize = 16 * 1024 * 1024;
const MAX_TOTAL: usize = 64 * 1024 * 1024;
const MAX_ARCHIVE: usize = MAX_TOTAL + MAX_FILES * 1024 + 1024;
type Files = BTreeMap<String, Vec<u8>>;
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileDigest {
    path: String,
    bytes: usize,
    sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    baseurl: String,
    source_input_sha256: String,
    archive_sha256: String,
    inventory_sha256: String,
    files: Vec<FileDigest>,
}
fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn inventory(files: &Files) -> Vec<FileDigest> {
    files
        .iter()
        .map(|(path, bytes)| FileDigest {
            path: path.clone(),
            bytes: bytes.len(),
            sha256: hash(bytes),
        })
        .collect()
}
fn inventory_hash(files: &[FileDigest]) -> Result<String, String> {
    Ok(hash(&serde_json::to_vec(files).map_err(|e| e.to_string())?))
}
fn public_path(path: &str, directory: bool) -> bool {
    if path.is_empty()
        || path.len() > 255
        || !path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._/".contains(&b))
    {
        return false;
    }
    let parts: Vec<_> = path.split('/').collect();
    if parts
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return false;
    }
    if parts.iter().any(|part| {
        let stem = part.split('.').next().unwrap_or("").to_ascii_lowercase();
        part.ends_with('.')
            || matches!(stem.as_str(), "con" | "prn" | "aux" | "nul")
            || (stem.len() == 4
                && (stem.starts_with("com") || stem.starts_with("lpt"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    }) {
        return false;
    }
    if parts.iter().any(|part| part.starts_with('.')) {
        return path == ".well-known/security.txt" && !directory
            || path == ".well-known" && directory;
    }
    if parts.len() == 1 && !directory {
        return matches!(
            path,
            "CNAME"
                | "index.html"
                | "404.html"
                | "robots.txt"
                | "sitemap.xml"
                | "feed.xml"
                | "search.json"
                | "style-guide.html"
        );
    }
    if !matches!(
        parts[0],
        "assets"
            | "concepts"
            | "explanation"
            | "help"
            | "how-to"
            | "reference"
            | "runbooks"
            | "standards"
            | "start-here"
            | "tutorials"
    ) {
        return false;
    }
    if ["assets/llms", "assets/og", "assets/screenshots"]
        .iter()
        .any(|root| path == *root || path.starts_with(&format!("{root}/")))
    {
        return false;
    }
    if directory {
        return true;
    }
    let extension = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if parts[0] != "assets" {
        return matches!(extension, "html" | "json" | "txt" | "xml");
    }
    matches!(
        extension,
        "css"
            | "html"
            | "js"
            | "json"
            | "png"
            | "svg"
            | "txt"
            | "webp"
            | "xml"
            | "ico"
            | "jpg"
            | "jpeg"
            | "gif"
            | "woff"
            | "woff2"
            | "ttf"
            | "eot"
    )
}
fn regular(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    metadata.is_file() || metadata.is_dir()
}
fn existing(path: &Path) -> Result<PathBuf, String> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("parent alias in archive path".into());
    }
    for ancestor in path.ancestors() {
        if !regular(&fs::symlink_metadata(ancestor).map_err(|e| e.to_string())?) {
            return Err("link or special archive path component".into());
        }
    }
    path.canonicalize().map_err(|e| e.to_string())
}
fn read_bound(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !regular(&metadata) || !metadata.is_file() || metadata.len() > limit as u64 {
        return Err("nonregular or oversized archive input".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit || bytes.len() as u64 != metadata.len() {
        return Err("archive input changed or exceeded budget".into());
    }
    Ok(bytes)
}
fn verify_files(files: &Files) -> Result<(), String> {
    if files.is_empty()
        || files.len() > MAX_FILES
        || !files.contains_key("index.html")
        || !files.contains_key(".well-known/security.txt")
    {
        return Err("public archive inventory incomplete".into());
    }
    let mut total = 0usize;
    let mut folded = std::collections::BTreeSet::new();
    for (path, bytes) in files {
        if !public_path(path, false)
            || bytes.len() > MAX_FILE
            || !folded.insert(path.to_ascii_lowercase())
        {
            return Err("unapproved public archive path, duplicate or file size".into());
        }
        total = total
            .checked_add(bytes.len())
            .ok_or("archive byte overflow")?;
        if total > MAX_TOTAL {
            return Err("public archive total budget".into());
        }
        if let Ok(text) = std::str::from_utf8(bytes)
            && crate::source::private(text)
        {
            return Err("private marker in public archive".into());
        }
    }
    Ok(())
}
fn collect(root: &Path) -> Result<Files, String> {
    fn visit(root: &Path, path: &Path, files: &mut Files, count: &mut usize) -> Result<(), String> {
        *count += 1;
        if *count > MAX_FILES * 2 {
            return Err("public tree entry budget".into());
        }
        let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !regular(&metadata) {
            return Err("public tree contains link or special entry".into());
        }
        let name = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("non UTF-8 public path")?
            .replace('\\', "/");
        if !name.is_empty() && !public_path(&name, metadata.is_dir()) {
            return Err("public tree path outside allowlist".into());
        }
        if metadata.is_dir() {
            for child in fs::read_dir(path).map_err(|e| e.to_string())? {
                visit(
                    root,
                    &child.map_err(|e| e.to_string())?.path(),
                    files,
                    count,
                )?;
            }
        } else {
            if files.len() >= MAX_FILES {
                return Err("public file count budget".into());
            }
            let bytes = read_bound(path, MAX_FILE)?;
            let used: usize = files.values().map(Vec::len).sum();
            if used + bytes.len() > MAX_TOTAL {
                return Err("public tree total byte budget".into());
            }
            files.insert(name, bytes);
        }
        Ok(())
    }
    let mut files = Files::new();
    visit(root, root, &mut files, &mut 0)?;
    verify_files(&files)?;
    Ok(files)
}
fn fresh(path: &Path, forbidden: &[&Path]) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || fs::symlink_metadata(path).is_ok()
    {
        return Err("fresh absolute archive output required".into());
    }
    let parent = existing(path.parent().ok_or("archive output parent absent")?)?;
    let resolved = parent.join(path.file_name().ok_or("archive output filename absent")?);
    if forbidden
        .iter()
        .any(|root| resolved.starts_with(root) || root.starts_with(&resolved))
    {
        return Err("archive output overlaps protected input".into());
    }
    Ok(resolved)
}
fn create(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}
fn restore(files: &Files, destination: &Path) -> Result<(), String> {
    fs::create_dir(destination).map_err(|e| e.to_string())?;
    for (name, bytes) in files {
        let target = destination.join(name);
        fs::create_dir_all(target.parent().ok_or("restore parent absent")?)
            .map_err(|e| e.to_string())?;
        create(&target, bytes)?;
    }
    if collect(destination)? != *files {
        return Err("restored public inventory differs".into());
    }
    Ok(())
}
fn checked(source: &Path, site: &Path, baseurl: &str) -> Result<crate::Report, String> {
    let report = crate::check_output(source, site, baseurl)?;
    if !report.errors.is_empty() {
        return Err(format!(
            "public archive rendered validation failed: {:?}",
            report.errors
        ));
    }
    Ok(report)
}

/// Run `package` or `archive-check` with explicit private output paths.
/// Failures preserve partial outputs for inspection and never overwrite/retry them.
pub fn command(mode: &str, arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let mut options = BTreeMap::new();
    let mut arguments = arguments;
    while let Some(flag) = arguments.next() {
        if ![
            "--source",
            "--site",
            "--archive",
            "--manifest",
            "--restored",
            "--baseurl",
        ]
        .contains(&flag.as_str())
        {
            return Err("unknown public archive option".into());
        }
        let value = arguments.next().ok_or("missing public archive option")?;
        if value.starts_with("--") || options.insert(flag, value).is_some() {
            return Err("duplicate or missing public archive option".into());
        }
    }
    let source = existing(Path::new(
        &options
            .remove("--source")
            .ok_or("archive requires --source")?,
    ))?;
    let baseurl = options.remove("--baseurl").unwrap_or_default();
    crate::output::validate_baseurl(&baseurl)?;
    let input = crate::inputs::digest(&source)?;
    let archive = PathBuf::from(
        options
            .remove("--archive")
            .ok_or("archive requires --archive")?,
    );
    let manifest_path = PathBuf::from(
        options
            .remove("--manifest")
            .ok_or("archive requires --manifest")?,
    );
    let restored = PathBuf::from(
        options
            .remove("--restored")
            .ok_or("archive requires --restored")?,
    );
    let (files, manifest, archive_bytes, restored) = if mode == "package" {
        let site_path = PathBuf::from(options.remove("--site").ok_or("package requires --site")?);
        let site = existing(&site_path)?;
        if !site_path.is_absolute()
            || !regular(&fs::symlink_metadata(&site_path).map_err(|e| e.to_string())?)
            || site.starts_with(&source)
            || source.starts_with(&site)
        {
            return Err("dedicated public site root required".into());
        }
        checked(&source, &site, &baseurl)?;
        let files = collect(&site)?;
        let archive = fresh(&archive, &[&source, &site])?;
        if archive.file_name().and_then(|s| s.to_str()) != Some("artifact.tar") {
            return Err("public archive must be named artifact.tar".into());
        }
        let manifest_path = fresh(&manifest_path, &[&source, &site, &archive])?;
        let restored = fresh(&restored, &[&source, &site, &archive, &manifest_path])?;
        let archive_bytes = tar::encode(&files)?;
        let manifest = Manifest {
            schema: "humanymous.public-pages-archive/1".into(),
            baseurl: baseurl.clone(),
            source_input_sha256: input.clone(),
            archive_sha256: hash(&archive_bytes),
            inventory_sha256: inventory_hash(&inventory(&files))?,
            files: inventory(&files),
        };
        if collect(&site)? != files {
            return Err("public site changed during packaging".into());
        }
        create(&archive, &archive_bytes)?;
        let written = read_bound(&archive, MAX_ARCHIVE)?;
        let decoded = tar::decode(&written)?;
        if decoded != files {
            return Err("written public archive differs from original inventory".into());
        }
        // The receipt is created only after restore/check, below.
        (decoded, (manifest, Some(manifest_path)), written, restored)
    } else if mode == "archive-check" && !options.contains_key("--site") {
        if !archive.is_absolute() || !manifest_path.is_absolute() {
            return Err("absolute downloaded archive and manifest required".into());
        }
        let archive = existing(&archive)?;
        let manifest_input = existing(&manifest_path)?;
        let restored = fresh(&restored, &[&source, &archive, &manifest_input])?;
        let bytes = read_bound(&archive, MAX_ARCHIVE)?;
        let manifest: Manifest =
            serde_json::from_slice(&read_bound(&manifest_input, 2 * 1024 * 1024)?)
                .map_err(|e| e.to_string())?;
        let files = tar::decode(&bytes)?;
        (files, (manifest, None), bytes, restored)
    } else {
        return Err("invalid archive command or incompatible --site".into());
    };
    if !options.is_empty() {
        return Err("unused archive option".into());
    }
    let (manifest, receipt) = manifest;
    verify_files(&files)?;
    if manifest.schema != "humanymous.public-pages-archive/1"
        || manifest.baseurl != baseurl
        || manifest.source_input_sha256 != input
        || manifest.archive_sha256 != hash(&archive_bytes)
        || manifest.files != inventory(&files)
        || manifest.inventory_sha256 != inventory_hash(&manifest.files)?
    {
        return Err("public archive does not match trusted manifest and source".into());
    }
    restore(&files, &restored)?;
    let report = checked(&source, &restored, &baseurl)?;
    if crate::inputs::digest(&source)? != input {
        return Err("authored inputs changed during archive validation".into());
    }
    if let Some(path) = receipt {
        create(
            &path,
            &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
        )?;
    }
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.public-pages-roundtrip/1", "archive_sha256":manifest.archive_sha256,"inventory_sha256":manifest.inventory_sha256,"files":files.len(),"pages":report.pages,"rendered_html":report.rendered_html,"errors":report.errors,"source_input_sha256":input,"baseurl":baseurl,"browser_qualified":false,"assistive_technology_qualified":false,"deployment_performed":false})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_private_and_escaping_paths_never_enter_public_inventory() {
        for path in [
            "../index.html",
            "/index.html",
            "assets/../x.css",
            "assets/.env",
            ".git/config",
            ".well-known/other.txt",
            "reviews.json",
            "tooling/a.html",
            "assets/a.map",
            "assets/og/a.png",
            "reference/a.rs",
            "assets/a:ads.css",
        ] {
            assert!(!public_path(path, false), "{path}");
        }
        for path in [
            "index.html",
            ".well-known/security.txt",
            "assets/css/site.css",
            "standards/draft-v0/validation/fixtures/adapter-parser-v1.json",
        ] {
            assert!(public_path(path, false), "{path}");
        }
    }
    #[test]
    fn security_disclosure_and_exact_bytes_are_required() {
        let mut files = Files::from([("index.html".into(), b"hello".to_vec())]);
        assert!(verify_files(&files).is_err());
        files.insert(
            ".well-known/security.txt".into(),
            b"Contact: synthetic\r\n".to_vec(),
        );
        verify_files(&files).unwrap();
        let bytes = tar::encode(&files).unwrap();
        assert_eq!(tar::decode(&bytes).unwrap(), files);
        assert_eq!(tar::encode(&files).unwrap(), bytes);
        let mut changed = files.clone();
        changed.get_mut("index.html").unwrap().push(0);
        assert_ne!(inventory(&files), inventory(&changed));
    }
    #[test]
    fn cross_platform_case_collisions_and_inventory_budget_refuse() {
        let base = Files::from([
            ("index.html".into(), Vec::new()),
            (".well-known/security.txt".into(), Vec::new()),
        ]);
        let mut files = base.clone();
        files.insert("assets/A.css".into(), Vec::new());
        files.insert("assets/a.css".into(), Vec::new());
        assert!(verify_files(&files).is_err());
        let mut files = base;
        for index in 0..MAX_FILES {
            files.insert(format!("reference/{index}.html"), Vec::new());
        }
        assert!(verify_files(&files).is_err());
        for path in ["assets/CON.txt", "assets/com1.css", "assets/a./b.css"] {
            assert!(!public_path(path, false));
        }
    }
    #[test]
    fn aggregate_content_budget_refuses_individually_valid_files() {
        let mut files = Files::from([
            ("index.html".into(), Vec::new()),
            (".well-known/security.txt".into(), vec![b'x']),
        ]);
        for index in 0..MAX_TOTAL / MAX_FILE {
            files.insert(format!("assets/{index}.txt"), vec![b'x'; MAX_FILE]);
        }
        assert!(verify_files(&files).is_err());
        assert!(tar::encode(&files).is_err());
    }
}
