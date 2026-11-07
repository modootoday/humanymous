//! Read-only validation of authored Markdown and rendered Pages artifacts.
mod build;
mod files;
mod output;
pub mod snapshot;
mod source;
pub mod vectors;

use serde::Serialize;
use std::path::Path;

/// A bounded validation result; successful documents are not runtime acceptance.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Number of authored reader pages examined.
    pub pages: usize,
    /// Every observed validation failure, without writing replacement content.
    pub errors: Vec<String>,
}

/// Check authored reader sources without changing files.
pub fn check_source(root: &Path) -> Result<Report, String> {
    let (pages, errors) = source::load(root)?;
    Ok(Report {
        pages: pages.len(),
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
