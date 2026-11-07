use crate::files;
use percent_encoding::percent_decode_str;
use pulldown_cmark::{Event, Options, Parser, Tag};
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub(crate) struct Page {
    pub(crate) path: PathBuf,
    pub(crate) url: String,
}

static PRIVATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r"SoT-\d+|sot:root/|\.spec(?:\.local)?/|\.agents/|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|github_pat_[A-Za-z0-9_]+|ghp_[A-Za-z0-9]{30,}"
).expect("constant private-pattern regex")
});

pub(crate) fn private(text: &str) -> bool {
    PRIVATE.is_match(text)
}

pub(crate) fn load(root: &Path) -> Result<(Vec<Page>, Vec<String>), String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut pages = Vec::new();
    let mut errors = Vec::new();
    let mut urls = BTreeSet::new();
    for path in files::walk(&root, true)? {
        if path.extension().and_then(|s| s.to_str()) != Some("md")
            || matches!(
                path.file_name().and_then(|s| s.to_str()),
                Some("AGENTS.md" | "INTEGRATION.md")
            )
        {
            continue;
        }
        let rel = files::relative(&root, &path);
        let text = files::read(&path)?.replace("\r\n", "\n");
        let Some((front, body)) = text
            .strip_prefix("---\n")
            .and_then(|s| s.split_once("\n---\n"))
        else {
            errors.push(format!("{rel}: missing explicit front matter"));
            continue;
        };
        let mut fields = BTreeMap::new();
        for line in front.lines() {
            if let Some((key, value)) = line.split_once(':')
                && fields
                    .insert(
                        key.trim(),
                        value.trim().trim_matches('"').trim_matches('\''),
                    )
                    .is_some()
            {
                errors.push(format!("{rel}: duplicate front matter key {key}"));
            }
        }
        for key in ["layout", "title", "permalink", "reviewed"] {
            if fields.get(key).is_none_or(|s| s.is_empty()) {
                errors.push(format!("{rel}: missing {key}"));
            }
        }
        if fields.get("layout") != Some(&"doc") {
            errors.push(format!("{rel}: reader layout must be doc"));
        }
        let url = fields.get("permalink").copied().unwrap_or_default();
        if !url.starts_with('/')
            || url.starts_with("//")
            || url.contains(['?', '#', '\\'])
            || url.split('/').any(|p| matches!(p, "." | ".."))
            || !urls.insert(url.to_owned())
        {
            errors.push(format!("{rel}: invalid or duplicate permalink {url}"));
        }
        if !body.contains("**Diátaxis quadrant:**") || !body.contains("**Audience:**") {
            errors.push(format!("{rel}: missing purpose/audience"));
        }
        if private(&text) {
            errors.push(format!("{rel}: private identifier or key pattern"));
        }
        if rel.starts_with("standards/") && fields.get("draft_status") != Some(&"project-draft") {
            errors.push(format!("{rel}: missing project-draft status"));
        }
        if rel.starts_with("standards/draft-v0/") && fields.get("draft_version") != Some(&"v0") {
            errors.push(format!("{rel}: missing v0 draft version"));
        }
        for event in Parser::new_ext(body, Options::all()) {
            if let Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) = event {
                let target = dest_url.as_ref();
                if target.starts_with("//") || url::Url::parse(target).is_ok() {
                    continue;
                }
                let local = target.split(['#', '?']).next().unwrap_or_default();
                if local.is_empty() {
                    continue;
                }
                let decoded = percent_decode_str(local)
                    .decode_utf8()
                    .map_err(|e| e.to_string())?;
                let linked = if local.starts_with('/') {
                    root.join(decoded.trim_start_matches('/'))
                } else {
                    path.parent().expect("reader parent").join(decoded.as_ref())
                };
                if !files::contained(&root, &linked) {
                    errors.push(format!("{rel}: missing or escaping source link {target}"));
                }
            }
        }
        pages.push(Page {
            path,
            url: url.to_owned(),
        });
    }
    if pages.is_empty() {
        errors.push("no authored reader pages".into());
    }
    crate::navigation::check(&root, &urls, &mut errors);
    Ok((pages, errors))
}
