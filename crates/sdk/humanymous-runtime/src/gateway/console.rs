//! Operator console page (legacy internal/gate/console.go at baseline 981d25c),
//! served at `/__hmn/admin/console`.
//!
//! The page markup is `assets/console/console.html`; its stylesheet is the
//! SoT-33/SoT-34 SRP partials under `assets/console/styles/`, concatenated in
//! file-name order (so the `@layer` order declared in `00-layers.css` is
//! deterministic) and injected into the `<style id="app-css">` placeholder.
//! The served page is one self-contained document with no external stylesheet
//! request, which keeps it CSP-clean.
use std::sync::LazyLock;

/// Route the operator console is served on.
pub const PATH: &str = "/__hmn/admin/console";

const PLACEHOLDER: &str = r#"<style id="app-css"></style>"#;

/// Page markup with the empty stylesheet placeholder.
pub const MARKUP: &str = include_str!("../../assets/console/console.html");

/// The SRP partials in file-name order.
pub const PARTIALS: [(&str, &str); 7] = [
    (
        "00-layers.css",
        include_str!("../../assets/console/styles/00-layers.css"),
    ),
    (
        "10-tokens.css",
        include_str!("../../assets/console/styles/10-tokens.css"),
    ),
    (
        "20-base.css",
        include_str!("../../assets/console/styles/20-base.css"),
    ),
    (
        "30-layout.css",
        include_str!("../../assets/console/styles/30-layout.css"),
    ),
    (
        "40-content.css",
        include_str!("../../assets/console/styles/40-content.css"),
    ),
    (
        "50-overlays.css",
        include_str!("../../assets/console/styles/50-overlays.css"),
    ),
    (
        "90-responsive.css",
        include_str!("../../assets/console/styles/90-responsive.css"),
    ),
];

static STYLESHEET: LazyLock<String> =
    LazyLock::new(|| PARTIALS.iter().flat_map(|(_, css)| [*css, "\n"]).collect());

static PAGE: LazyLock<String> = LazyLock::new(|| {
    MARKUP.replacen(
        PLACEHOLDER,
        &format!(r#"<style id="app-css">{}</style>"#, stylesheet()),
        1,
    )
});

/// The assembled stylesheet.
pub fn stylesheet() -> &'static str {
    &STYLESHEET
}

/// The served page with the stylesheet injected.
pub fn page() -> &'static str {
    &PAGE
}

/// The page's single inline script, if the markup has one.
pub fn inline_script() -> Option<&'static str> {
    let start = MARKUP.find("<script>")? + "<script>".len();
    let end = MARKUP.rfind("</script>")?;
    (start <= end).then(|| &MARKUP[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn modular_console_css_is_assembled_into_the_page() {
        assert!(!page().contains(PLACEHOLDER), "placeholder not filled");
        for want in [
            "@layer tokens, base, layout, content, overlays, responsive;",
            "--accent:#35d0ba",
            "--deny:#f0556a",
            ".navitem",
            ".kpi",
            ".verdict",
            ".drawer",
            "content-visibility:auto",
        ] {
            assert!(
                page().contains(want),
                "assembled console CSS missing {want}"
            );
        }
    }

    #[test]
    fn console_css_is_non_empty_and_drops_no_partial() {
        assert!(stylesheet().len() >= 2000, "{} bytes", stylesheet().len());
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/console/styles");
        let mut on_disk: Vec<String> = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.ends_with(".css"))
            .collect();
        on_disk.sort();
        let embedded: Vec<&str> = PARTIALS.iter().map(|(name, _)| *name).collect();
        assert_eq!(on_disk, embedded, "every partial is embedded, in order");
        let mut position = 0;
        for (name, css) in PARTIALS {
            let found = stylesheet()[position..]
                .find(css)
                .unwrap_or_else(|| panic!("{name} missing or out of order"));
            position += found + css.len();
        }
    }

    #[test]
    fn console_has_one_inline_script() {
        let script = inline_script().expect("console inline script not found");
        assert!(!script.trim().is_empty());
        assert_eq!(MARKUP.matches("<script>").count(), 1);
    }
}
