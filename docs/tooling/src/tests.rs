use crate::{check_output, check_source};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "pages-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("docs")).unwrap();
        fs::create_dir(root.join("site")).unwrap();
        fs::create_dir_all(root.join("docs/_data/nav")).unwrap();
        fs::write(
            root.join("docs/_data/nav/main.yml"),
            "- {title: Home, url: /}\n",
        )
        .unwrap();
        let f = Self(root);
        f.page("index.md", "/", "# Example\n");
        f.html("index.html", "https://humanymous.net/", "<h1>Example</h1>");
        f
    }
    fn page(&self, name: &str, url: &str, body: &str) {
        fs::write(self.0.join("docs").join(name), format!("---\nlayout: doc\ntitle: Example\npermalink: {url}\nreviewed: 2026-11-07\n---\n**Diátaxis quadrant:** Reference. **Audience:** reviewers.\n{body}")).unwrap();
    }
    fn html(&self, name: &str, canonical: &str, body: &str) {
        fs::write(self.0.join("site").join(name), format!("<!doctype html><html lang=\"en\"><head><title>Example</title><link rel=\"canonical\" href=\"{canonical}\"></head><body><a class=\"mt-skip\" href=\"#content\">Skip to content</a><main id=\"content\" tabindex=\"-1\">{body}</main></body></html>")).unwrap();
    }
    fn errors(&self, base: &str) -> Vec<String> {
        check_output(&self.0.join("docs"), &self.0.join("site"), base)
            .unwrap()
            .errors
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn valid_source_and_output_are_read_only() {
    let f = Fixture::new();
    let path = f.0.join("docs/index.md");
    let before = fs::read(&path).unwrap();
    assert!(check_source(&f.0.join("docs")).unwrap().errors.is_empty());
    assert!(f.errors("").is_empty());
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn public_archive_roundtrip_preserves_hidden_disclosure_and_rejects_tampering() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("site/.well-known")).unwrap();
    let disclosure = b"Contact: mailto:security@example.invalid\r\n";
    fs::write(f.0.join("site/.well-known/security.txt"), disclosure).unwrap();
    let archive = f.0.join("artifact.tar");
    let manifest = f.0.join("manifest.json");
    let restored = f.0.join("restored");
    let args = |site: bool, output: &std::path::Path| {
        let mut args = vec![
            "--source".into(),
            f.0.join("docs").to_str().unwrap().into(),
            "--archive".into(),
            archive.to_str().unwrap().into(),
            "--manifest".into(),
            manifest.to_str().unwrap().into(),
            "--restored".into(),
            output.to_str().unwrap().into(),
        ];
        if site {
            args.extend(["--site".into(), f.0.join("site").to_str().unwrap().into()]);
        }
        args.into_iter()
    };
    crate::package::command("package", args(true, &restored)).unwrap();
    assert_eq!(
        fs::read(restored.join(".well-known/security.txt")).unwrap(),
        disclosure
    );
    let downloaded = f.0.join("downloaded");
    crate::package::command("archive-check", args(false, &downloaded)).unwrap();
    assert_eq!(
        fs::read(downloaded.join("index.html")).unwrap(),
        fs::read(f.0.join("site/index.html")).unwrap()
    );
    let before = fs::read(&archive).unwrap();
    assert!(crate::package::command("package", args(true, &f.0.join("overwrite"))).is_err());
    assert_eq!(fs::read(&archive).unwrap(), before);
    let original_manifest = fs::read(&manifest).unwrap();
    let mut changed: serde_json::Value = serde_json::from_slice(&original_manifest).unwrap();
    changed["inventory_sha256"] = "00".repeat(32).into();
    fs::write(&manifest, serde_json::to_vec(&changed).unwrap()).unwrap();
    let bad_manifest = f.0.join("bad-manifest");
    assert!(crate::package::command("archive-check", args(false, &bad_manifest)).is_err());
    assert!(!bad_manifest.exists());
    fs::write(&manifest, original_manifest).unwrap();
    let mut tampered = before;
    tampered[512] ^= 1;
    fs::write(&archive, tampered).unwrap();
    let rejected = f.0.join("rejected");
    assert!(crate::package::command("archive-check", args(false, &rejected)).is_err());
    assert!(!rejected.exists());
}

#[test]
fn public_archive_missing_disclosure_and_private_files_refuse_before_archive_creation() {
    let f = Fixture::new();
    let args = || {
        vec![
            "--source".into(),
            f.0.join("docs").to_str().unwrap().into(),
            "--site".into(),
            f.0.join("site").to_str().unwrap().into(),
            "--archive".into(),
            f.0.join("artifact.tar").to_str().unwrap().into(),
            "--manifest".into(),
            f.0.join("manifest.json").to_str().unwrap().into(),
            "--restored".into(),
            f.0.join("restored").to_str().unwrap().into(),
        ]
        .into_iter()
    };
    assert!(crate::package::command("package", args()).is_err());
    assert!(!f.0.join("artifact.tar").exists());
    fs::create_dir(f.0.join("site/.well-known")).unwrap();
    fs::write(
        f.0.join("site/.well-known/security.txt"),
        b"Contact: synthetic",
    )
    .unwrap();
    fs::write(f.0.join("site/.env"), b"synthetic private content").unwrap();
    assert!(crate::package::command("package", args()).is_err());
    assert!(!f.0.join("artifact.tar").exists());
    fs::remove_file(f.0.join("site/.env")).unwrap();
    fs::create_dir(f.0.join("site/assets")).unwrap();
    fs::write(
        f.0.join("site/assets/data.json"),
        b"{\"note\":\"-----BEGIN PRIVATE KEY-----\"}",
    )
    .unwrap();
    assert!(crate::package::command("package", args()).is_err());
    assert!(!f.0.join("artifact.tar").exists());
}

#[cfg(any(unix, windows))]
#[test]
fn public_archive_refuses_linked_site_roots_before_creating_outputs() {
    let f = Fixture::new();
    let link = f.0.join("linked-site");
    #[cfg(unix)]
    std::os::unix::fs::symlink(f.0.join("site"), &link).unwrap();
    #[cfg(windows)]
    junction::create(f.0.join("site"), &link).unwrap();
    let args = vec![
        "--source".into(),
        f.0.join("docs").to_str().unwrap().into(),
        "--site".into(),
        link.to_str().unwrap().into(),
        "--archive".into(),
        f.0.join("artifact.tar").to_str().unwrap().into(),
        "--manifest".into(),
        f.0.join("manifest.json").to_str().unwrap().into(),
        "--restored".into(),
        f.0.join("restored").to_str().unwrap().into(),
    ];
    assert!(crate::package::command("package", args.into_iter()).is_err());
    assert!(!f.0.join("artifact.tar").exists());
    #[cfg(windows)]
    junction::delete(&link).unwrap();
    #[cfg(unix)]
    fs::remove_file(&link).unwrap();
}

#[test]
fn static_theme_skip_target_and_title_are_not_inferred_from_landmarks() {
    use scraper::Html;
    let valid = "<head><title>Example</title></head><body><a class='mt-skip' href='#doc'>Skip</a><main><div id='doc' tabindex='-1'><h1>Title</h1></div></main></body>";
    assert!(crate::accessibility::check(&Html::parse_document(valid)).is_empty());
    for changed in [
        valid.replace("class='mt-skip'", "class='other'"),
        valid.replace("href='#doc'", "href='#absent'"),
        valid.replace("tabindex='-1'", "tabindex='1'"),
        valid.replace("<main>", "<main hidden>"),
        valid.replace("<main>", "<main aria-hidden='true'>"),
        valid.replace(">Skip</a>", "> </a>"),
        valid.replace("<title>Example</title>", "<title> </title>"),
        valid.replace(
            "<title>Example</title>",
            "<title>One</title><title>Two</title>",
        ),
    ] {
        assert!(
            !crate::accessibility::check(&Html::parse_document(&changed)).is_empty(),
            "{changed}"
        );
    }
    let outside = "<head><title>Example</title></head><body><a class='mt-skip' href='#nav'>Skip</a><nav id='nav' tabindex='-1'>Navigation</nav><main><h1>Title</h1></main></body>";
    assert!(!crate::accessibility::check(&Html::parse_document(outside)).is_empty());
}

#[test]
fn reader_controls_fail_without_rejecting_unicode_or_layout_whitespace() {
    let f = Fixture::new();
    for control in ['\0', '\u{7}', '\u{7f}', '\u{85}', '\r'] {
        f.page("index.md", "/", &format!("Visible{control}text"));
        assert!(
            check_source(&f.0.join("docs"))
                .unwrap()
                .errors
                .iter()
                .any(|e| e.contains("unexpected control character"))
        );
    }
    f.page(
        "index.md",
        "/",
        "한국어 café العربية \u{200f}\n\tIndented text\n",
    );
    assert!(check_source(&f.0.join("docs")).unwrap().errors.is_empty());
    let path = f.0.join("docs/index.md");
    let text = fs::read_to_string(&path).unwrap().replace('\n', "\r\n");
    fs::write(path, text).unwrap();
    assert!(check_source(&f.0.join("docs")).unwrap().errors.is_empty());
}

#[test]
fn navigation_rejects_misindentation_missing_targets_and_empty_groups() {
    let f = Fixture::new();
    let menu = f.0.join("docs/_data/nav/main.yml");
    fs::write(
        &menu,
        "- title: Reference\n  items:\n    - {title: Home, url: /}\n",
    )
    .unwrap();
    assert!(check_source(&f.0.join("docs")).unwrap().errors.is_empty());
    for text in [
        "- title: Reference\n  items:\n      - {title: Home, url: /}\n",
        "- {title: Missing, url: /missing.html}\n",
        "- title: Reference\n  items:\n",
        "- title: Reference\n    - {title: Home, url: /}\n",
        "    - {title: Home, url: /}\n",
        "- title: Reference\n  items:\n  items:\n",
        "- {title: Home, url: /}\n\t- {title: Home, url: /}\n",
    ] {
        fs::write(&menu, text).unwrap();
        assert!(
            check_source(&f.0.join("docs"))
                .unwrap()
                .errors
                .iter()
                .any(|e| e.starts_with("navigation:")),
            "{text}"
        );
    }
    fs::remove_file(&menu).unwrap();
    assert!(
        check_source(&f.0.join("docs"))
            .unwrap()
            .errors
            .iter()
            .any(|e| e.contains("navigation:"))
    );
}

#[test]
fn build_input_commitment_covers_navigation_tokens_binary_assets_and_inventory() {
    let f = Fixture::new();
    let root = f.0.join("docs");
    fs::create_dir_all(root.join("_data")).unwrap();
    fs::create_dir(root.join("_sass")).unwrap();
    fs::create_dir(root.join("assets")).unwrap();
    fs::create_dir(root.join("tooling")).unwrap();
    let initial = crate::inputs::digest(&root).unwrap();
    fs::write(root.join("tooling/ignored.rs"), "private verifier").unwrap();
    assert_eq!(initial, crate::inputs::digest(&root).unwrap());
    let mut previous = initial;
    for (path, bytes) in [
        ("_data/nav.yml", b"navigation".as_slice()),
        ("_sass/tokens.scss", b"tokens".as_slice()),
        ("assets/image.png", b"\x89PNG\r\n\0".as_slice()),
        ("assets/reviews.json", b"published nested asset".as_slice()),
        ("added.md", b"new reader page".as_slice()),
    ] {
        fs::write(root.join(path), bytes).unwrap();
        let current = crate::inputs::digest(&root).unwrap();
        assert_ne!(previous, current, "{path}");
        previous = current;
    }
    fs::remove_file(root.join("added.md")).unwrap();
    assert_ne!(previous, crate::inputs::digest(&root).unwrap());
}
#[test]
fn missing_escaping_reference_links_and_images_fail() {
    let f = Fixture::new();
    f.page(
        "index.md",
        "/",
        "[missing][ref]\n\n[ref]: ../outside.md\n\n![alt](absent.png)",
    );
    let e = check_source(&f.0.join("docs")).unwrap().errors;
    assert_eq!(e.iter().filter(|s| s.contains("source link")).count(), 2);
}
#[test]
fn private_identifiers_missing_metadata_and_duplicate_urls_fail() {
    let f = Fixture::new();
    f.page("another.md", "/", "sot:root/private");
    fs::write(f.0.join("docs/bare.md"), "# Bare").unwrap();
    let e = check_source(&f.0.join("docs")).unwrap().errors;
    for expected in ["private identifier", "duplicate permalink", "front matter"] {
        assert!(e.iter().any(|s| s.contains(expected)), "{e:?}");
    }
}
#[test]
fn index_suffix_is_not_directory_index_and_decoded_anchors_work() {
    let f = Fixture::new();
    f.page("other-index.md", "/other-index.html", "# Other\n");
    f.html(
        "other-index.html",
        "https://humanymous.net/other-index.html",
        "<h1 id=\"café\">Other</h1>",
    );
    f.html(
        "index.html",
        "https://humanymous.net/",
        "<h1>Home</h1><a href=\"other-index.html#caf%C3%A9\">go</a>",
    );
    assert!(f.errors("").is_empty());
}
#[test]
fn canonical_baseurl_escape_duplicate_id_and_missing_anchor_fail() {
    let f = Fixture::new();
    f.html("index.html", "https://humanymous.net/", "<h1 id=\"x\">Home</h1><p id=\"x\">dup</p><a href=\"/#absent\">escape</a><a href=\"#absent\">absent</a>");
    let e = f.errors("/humanymous");
    for expected in ["canonical", "baseurl", "duplicate id", "missing anchor"] {
        assert!(e.iter().any(|s| s.contains(expected)), "{e:?}");
    }
}

#[test]
fn same_origin_absolute_and_protocol_relative_links_are_checked_locally() {
    let f = Fixture::new();
    f.html("index.html", "https://humanymous.net/", "<h1 id=\"home\">Home</h1><a href=\"https://humanymous.net:443/#home\">absolute</a><a href=\"//humanymous.net/#home\">protocol relative</a><a href=\"https://example.invalid/absent#missing\">external</a><a href=\"mailto:reviewer@example.invalid\">mail</a>");
    assert!(f.errors("").is_empty());
    f.html("index.html", "https://humanymous.net/", "<h1>Home</h1><a href=\"https://humanymous.net/absent.html\">missing</a><a href=\"https://humanymous.net/#missing\">anchor</a><a href=\"//humanymous.net/#other-missing\">other anchor</a>");
    let errors = f.errors("");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("missing or escaping built target"))
    );
    assert_eq!(
        errors
            .iter()
            .filter(|e| e.contains("missing anchor"))
            .count(),
        2
    );
}

#[test]
fn absolute_same_origin_links_cannot_escape_repository_baseurl() {
    let f = Fixture::new();
    f.html("index.html", "https://humanymous.net/humanymous/", "<h1 id=\"home\">Home</h1><a href=\"https://humanymous.net/humanymous/#home\">valid</a><a href=\"//humanymous.net/humanymous/#home\">also valid</a>");
    assert!(f.errors("/humanymous").is_empty());
    f.html("index.html", "https://humanymous.net/humanymous/", "<h1>Home</h1><a href=\"https://humanymous.net/\">escape</a><a href=\"//humanymous.net/humanymous-other/\">prefix escape</a>");
    assert_eq!(
        f.errors("/humanymous")
            .iter()
            .filter(|e| e.contains("link escapes baseurl"))
            .count(),
        2
    );
}
#[test]
fn explicit_directory_index_links_validate_the_same_document_anchors() {
    let f = Fixture::new();
    f.html("index.html", "https://humanymous.net/", "<h1 id='home'>Home</h1><a href='index.html#home'>valid</a><a href='https://humanymous.net/index.html#absent'>missing</a>");
    let errors = f.errors("");
    assert_eq!(
        errors
            .iter()
            .filter(|e| e.contains("missing anchor"))
            .count(),
        1,
        "{errors:?}"
    );
}
#[test]
fn generated_html_is_checked_without_an_authored_markdown_counterpart() {
    let f = Fixture::new();
    f.html(
        "generated.html",
        "https://humanymous.net/generated.html",
        "<h1>Generated</h1><a href='/absent.html'>broken</a>",
    );
    let errors = f.errors("");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("/generated.html: missing or escaping built target")),
        "{errors:?}"
    );
    f.html(
        "generated.html",
        "https://humanymous.net/generated.html",
        "<h1>Generated</h1><a href='/'>Home</a>",
    );
    let report = check_output(&f.0.join("docs"), &f.0.join("site"), "").unwrap();
    assert!(report.errors.is_empty());
    assert_eq!(report.pages, 1);
    assert_eq!(report.rendered_html, Some(2));
    assert_eq!(check_source(&f.0.join("docs")).unwrap().rendered_html, None);
}
#[test]
fn maps_private_output_and_missing_alt_fail() {
    let f = Fixture::new();
    fs::write(f.0.join("site/theme.css.map"), "{}").unwrap();
    fs::write(f.0.join("site/reviews.json"), "{}").unwrap();
    f.html(
        "index.html",
        "https://humanymous.net/",
        "<h1>Home</h1><img src=\"missing.png\">",
    );
    let e = f.errors("");
    for expected in ["source map", "non-public", "without alt", "built target"] {
        assert!(e.iter().any(|s| s.contains(expected)), "{e:?}");
    }
}
#[test]
fn invalid_baseurl_and_empty_reader_tree_fail() {
    let f = Fixture::new();
    assert!(check_output(&f.0.join("docs"), &f.0.join("site"), "//external").is_err());
    fs::remove_file(f.0.join("docs/index.md")).unwrap();
    assert!(
        check_source(&f.0.join("docs"))
            .unwrap()
            .errors
            .iter()
            .any(|s| s.contains("no authored"))
    );
}
#[test]
fn build_refuses_relative_or_source_ancestor_destination_before_processes() {
    let f = Fixture::new();
    assert!(crate::validate(&f.0.join("docs"), &f.0.join("output"), "//external").is_err());
    for destination in [PathBuf::from("relative"), f.0.join("docs"), f.0.clone()] {
        assert!(crate::validate(&f.0.join("docs"), &destination, "").is_err());
    }
}
