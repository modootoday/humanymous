//! Read-only validation of authored Markdown and rendered Pages artifacts.
mod accessibility;
pub mod browser;
mod build;
mod files;
mod inputs;
mod navigation;
mod output;
pub mod review;
pub mod snapshot;
mod source;
pub mod vectors;

use serde::Serialize;
use std::path::Path;

/// A bounded validation result; successful documents are not runtime acceptance.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Exact local build inputs, only when validated unchanged across a real build.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_input_sha256: Option<String>,
    /// Number of authored reader pages examined.
    pub pages: usize,
    /// All rendered HTML files checked, including generated-only public pages.
    /// Absent for source-only checks, which never inspect output artifacts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_html: Option<usize>,
    /// Every observed validation failure, without writing replacement content.
    pub errors: Vec<String>,
}

/// Check authored reader sources without changing files.
pub fn check_source(root: &Path) -> Result<Report, String> {
    let (pages, errors) = source::load(root)?;
    Ok(Report {
        build_input_sha256: None,
        pages: pages.len(),
        rendered_html: None,
        errors,
    })
}

/// Check rendered pages, links, canonical URLs and private/debug exclusions.
pub fn check_output(root: &Path, site: &Path, baseurl: &str) -> Result<Report, String> {
    output::check(root, site, baseurl)
}

/// Execute the pinned upstream build/checker and then validate its actual output.
pub fn validate(root: &Path, destination: &Path, baseurl: &str) -> Result<Report, String> {
    build::validate(root, destination, baseurl)
}

#[cfg(test)]
mod tests;
