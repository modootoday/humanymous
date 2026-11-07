//! Read-only authored documentation review verifier. Never writes review evidence.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    reviews: Vec<Review>,
    surfaces: Vec<Surface>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Surface {
    id: String,
    sources: Vec<String>,
    pages: Vec<String>,
    reviewer: String,
    notes: String,
    checks: Vec<String>,
}
const SURFACES: &[(&str, &str)] = &[
    ("cli-doctor", "native/doctor"),
    ("cli-install", "native/install-dry-run-no-mutation"),
    ("configuration", "native/configuration-invalid"),
    ("server-api", "browser/missing-signals-closed"),
    ("gate-proxy", "docker/gate"),
    ("local-management", "native/local-only-admin"),
    ("audit", "docker/audit-restart"),
    ("browser-wasm", "browser/collector-parity"),
    ("accessible-recovery", "browser/challenge-assistive"),
    ("node-adapter", "node/native-host-process"),
    ("nginx-adapter", "docker/deployment"),
    ("capability-boundaries", "docker/raw-tls-h2"),
];
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    source: String,
    sha256: String,
    reviewer: String,
    notes: String,
    pages: Vec<String>,
}

fn inventory(root: &Path, path: &Path, files: &mut BTreeSet<String>) -> Result<(), String> {
    if !path.exists() {
        return Err(format!(
            "required governed source missing: {}",
            path.display()
        ));
    }
    if path
        .symlink_metadata()
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err(format!("symlink in governed source: {}", path.display()));
    }
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if [
                ".git",
                "node_modules",
                "target",
                ".bundle",
                "vendor",
                ".build-cache",
            ]
            .contains(&entry.file_name().to_string_lossy().as_ref())
            {
                continue;
            }
            inventory(root, &entry.path(), files)?;
        }
    } else {
        files.insert(
            path.strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/"),
        );
    }
    Ok(())
}

fn verify(root: &Path) -> Result<usize, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let manifest: Manifest = serde_json::from_slice(
        &fs::read(root.join("docs/reviews.json"))
            .map_err(|e| format!("authored docs/reviews.json missing: {e}"))?,
    )
    .map_err(|e| e.to_string())?;
    if manifest.schema != 1 {
        return Err("documentation review schema must be 1".into());
    }
    let mut expected = BTreeSet::new();
    for source in [
        "crates",
        "adapters",
        "web",
        "build",
        "Cargo.toml",
        "Cargo.lock",
    ] {
        inventory(&root, &root.join(source), &mut expected)?;
    }
    // Generated caches are not authored review evidence. A tracked file may
    // never hide under that exclusion, however: the Git source inventory wins.
    if root.join(".git").exists() {
        let tracked = std::process::Command::new("git")
            .args([
                "ls-files",
                "-z",
                "--",
                "crates",
                "adapters",
                "web",
                "build",
                "Cargo.toml",
                "Cargo.lock",
            ])
            .current_dir(&root)
            .output()
            .map_err(|e| e.to_string())?;
        if !tracked.status.success() {
            return Err("Git source inventory unavailable".into());
        }
        for path in String::from_utf8(tracked.stdout)
            .map_err(|e| e.to_string())?
            .split('\0')
            .filter(|path| !path.is_empty())
        {
            if Path::new(path)
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Err("unsafe tracked review path".into());
            }
            if root.join(path).is_file() {
                expected.insert(path.replace('\\', "/"));
            }
        }
    }
    let mut reviews = BTreeMap::new();
    for review in manifest.reviews {
        if reviews.insert(review.source.clone(), review).is_some() {
            return Err("duplicate source review".into());
        }
    }
    if expected != reviews.keys().cloned().collect() {
        let missing: Vec<_> = expected
            .iter()
            .filter(|s| !reviews.contains_key(*s))
            .take(10)
            .collect();
        return Err(format!(
            "review inventory differs from governed source; missing (first 10): {missing:?}"
        ));
    }
    for (source, review) in &reviews {
        let source_bytes = fs::read(root.join(source)).map_err(|e| e.to_string())?;
        if source.ends_with(".rs") && source_bytes.contains(&b'\r') {
            return Err(format!("authored Rust source is not LF: {source}"));
        }
        let digest = format!("{:x}", Sha256::digest(source_bytes));
        if digest != review.sha256 {
            return Err(format!("stale source review: {source}"));
        }
        if review.reviewer.trim().len() < 3
            || review.notes.trim().len() < 30
            || review.pages.is_empty()
        {
            return Err(format!("missing substantive authored review: {source}"));
        }
        for page in &review.pages {
            let lexical = Path::new(page);
            if !page.starts_with("docs/")
                || lexical.extension().and_then(|x| x.to_str()) != Some("md")
                || lexical
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err(format!("unsafe reader page: {page}"));
            }
            let file = root
                .join(page)
                .canonicalize()
                .map_err(|e| format!("missing reader page {page}: {e}"))?;
            if !file.starts_with(
                root.join("docs")
                    .canonicalize()
                    .map_err(|e| e.to_string())?,
            ) {
                return Err(format!("reader page escapes docs: {page}"));
            }
            let text = fs::read_to_string(file).map_err(|e| e.to_string())?;
            if !text.replace("\r\n", "\n").starts_with("---\n") || !text.contains("layout: doc") {
                return Err(format!("reader page lacks theme front matter: {page}"));
            }
            if [
                "go run ./cmd/",
                "go build ./cmd/",
                "wasm_exec.js",
                "go install github.com/modootoday/humanymous",
            ]
            .iter()
            .any(|old| text.contains(old))
            {
                return Err(format!("stale Go reader instructions: {page}"));
            }
        }
    }
    let mut surface_ids = BTreeSet::new();
    for surface in manifest.surfaces {
        if !surface_ids.insert(surface.id.clone()) {
            return Err(format!("duplicate capability review: {}", surface.id));
        }
        let required = SURFACES
            .iter()
            .find(|(id, _)| *id == surface.id)
            .ok_or_else(|| format!("unknown release surface: {}", surface.id))?;
        if surface.sources.is_empty()
            || surface.pages.is_empty()
            || surface.reviewer.trim().len() < 3
            || surface.notes.trim().len() < 30
            || !surface.checks.iter().any(|check| check == required.1)
        {
            return Err(format!(
                "incomplete capability/example review: {}",
                surface.id
            ));
        }
        for source in &surface.sources {
            let review = reviews
                .get(source)
                .ok_or_else(|| format!("unreviewed capability source: {source}"))?;
            if !surface.pages.iter().all(|page| review.pages.contains(page)) {
                return Err(format!(
                    "capability pages absent from source review: {}",
                    surface.id
                ));
            }
        }
    }
    if surface_ids != SURFACES.iter().map(|(id, _)| (*id).to_string()).collect() {
        return Err("required command/API/configuration/capability review missing".into());
    }
    Ok(reviews.len())
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    match verify(&root) {
        Ok(count) => println!("authored documentation reviews current: {count} sources"),
        Err(error) => {
            eprintln!("documentation release blocked: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SERIAL: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            assert!(
                self.0
                    .canonicalize()
                    .unwrap()
                    .starts_with(std::env::temp_dir().canonicalize().unwrap())
            );
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "docs-review-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["docs", "crates", "adapters", "web", "build"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        fs::write(root.join("Cargo.toml"), "source").unwrap();
        fs::write(root.join("Cargo.lock"), "lock").unwrap();
        fs::write(
            root.join("docs/guide.md"),
            "---\nlayout: doc\ntitle: Guide\n---\nReviewed instructions\n",
        )
        .unwrap();
        let reviews: Vec<_> = ["Cargo.toml", "Cargo.lock"].iter().map(|source| serde_json::json!({"source": source, "sha256": format!("{:x}", Sha256::digest(fs::read(root.join(source)).unwrap())), "reviewer": "maintainer", "notes": "Reviewed package configuration and reader installation instructions.", "pages": ["docs/guide.md"]})).collect();
        let surfaces: Vec<_> = SURFACES.iter().map(|(id, check)| serde_json::json!({
            "id": id, "sources": ["Cargo.toml"], "pages": ["docs/guide.md"],
            "reviewer": "maintainer", "notes": "Reviewed the capability limits and associated executable example.", "checks": [check]
        })).collect();
        fs::write(
            root.join("docs/reviews.json"),
            serde_json::to_vec(
                &serde_json::json!({"schema": 1, "reviews": reviews, "surfaces": surfaces}),
            )
            .unwrap(),
        )
        .unwrap();
        Fixture(root)
    }
    #[test]
    fn current_reviews_pass_without_writes() {
        let f = fixture();
        let before = fs::read(f.0.join("docs/reviews.json")).unwrap();
        assert_eq!(verify(&f.0).unwrap(), 2);
        assert_eq!(before, fs::read(f.0.join("docs/reviews.json")).unwrap());
    }
    #[test]
    fn changed_source_requires_review_even_when_prose_changes() {
        let f = fixture();
        fs::write(f.0.join("Cargo.toml"), "changed").unwrap();
        fs::write(
            f.0.join("docs/guide.md"),
            "---\nlayout: doc\n---\nUnrelated edit\n",
        )
        .unwrap();
        assert!(verify(&f.0).unwrap_err().contains("stale source"));
    }
    #[test]
    fn new_source_cannot_be_omitted() {
        let f = fixture();
        fs::write(f.0.join("crates/new.rs"), "new surface").unwrap();
        assert!(verify(&f.0).unwrap_err().contains("inventory differs"));
    }
    #[test]
    fn missing_page_blocks_release() {
        let f = fixture();
        fs::remove_file(f.0.join("docs/guide.md")).unwrap();
        assert!(verify(&f.0).unwrap_err().contains("missing reader page"));
    }
    #[test]
    fn missing_review_is_not_success() {
        let f = fixture();
        fs::remove_file(f.0.join("docs/reviews.json")).unwrap();
        assert!(verify(&f.0).is_err());
    }
    #[test]
    fn source_hashes_without_capability_example_reviews_fail() {
        let f = fixture();
        let path = f.0.join("docs/reviews.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["surfaces"] = serde_json::json!([]);
        fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(
            verify(&f.0)
                .unwrap_err()
                .contains("capability review missing")
        );
    }
    #[test]
    fn unrelated_check_cannot_cover_a_capability() {
        let f = fixture();
        let path = f.0.join("docs/reviews.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["surfaces"][0]["checks"] = serde_json::json!(["unrelated"]);
        fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(verify(&f.0).unwrap_err().contains("incomplete capability"));
    }
    #[test]
    fn generated_cache_is_excluded_but_tracked_source_cannot_hide_in_it() {
        let f = fixture();
        let directory = f.0.join("adapters/.build-cache");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("generated.rs"), "generated build output\n").unwrap();
        assert!(verify(&f.0).is_ok());
        assert!(
            std::process::Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&f.0)
                .status()
                .unwrap()
                .success()
        );
        assert!(
            std::process::Command::new("git")
                .args(["add", "-f", "adapters/.build-cache/generated.rs"])
                .current_dir(&f.0)
                .status()
                .unwrap()
                .success()
        );
        assert!(verify(&f.0).unwrap_err().contains("inventory differs"));
    }
}
