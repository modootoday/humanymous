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
        fs::write(self.0.join("site").join(name), format!("<!doctype html><html lang=\"en\"><head><link rel=\"canonical\" href=\"{canonical}\"></head><body><main>{body}</main></body></html>")).unwrap();
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
