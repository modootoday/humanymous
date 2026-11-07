use crate::{Report, files, source};
use percent_encoding::percent_decode_str;
use scraper::{Html, Selector};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use url::Url;

struct Document {
    ids: BTreeSet<String>,
    links: Vec<String>,
    canonical: Vec<String>,
    errors: Vec<String>,
    main: usize,
    h1: usize,
    english: bool,
}

fn select(value: &str) -> Selector {
    Selector::parse(value).expect("constant HTML selector")
}

fn parse(text: &str) -> Document {
    let html = Html::parse_document(text);
    let mut d = Document {
        ids: BTreeSet::new(),
        links: Vec::new(),
        canonical: Vec::new(),
        errors: Vec::new(),
        main: html.select(&select("main")).count(),
        h1: html.select(&select("h1")).count(),
        english: html
            .select(&select("html"))
            .any(|e| e.value().attr("lang") == Some("en")),
    };
    for e in html.select(&select("*")) {
        if let Some(id) = e.value().attr("id")
            && !d.ids.insert(id.to_owned())
        {
            d.errors.push(format!("duplicate id {id}"));
        }
        for attr in ["href", "src"] {
            if let Some(v) = e.value().attr(attr)
                && !v.is_empty()
            {
                d.links.push(v.to_owned());
            }
        }
        if e.value().name() == "img" && e.value().attr("alt").is_none() {
            d.errors.push("image without alt".into());
        }
        if e.value().name() == "link" && e.value().attr("rel") == Some("canonical") {
            d.canonical
                .push(e.value().attr("href").unwrap_or_default().to_owned());
        }
    }
    if source::private(text) {
        d.errors
            .push("private identifier or key pattern in HTML".into());
    }
    d
}

pub(crate) fn validate_baseurl(baseurl: &str) -> Result<(), String> {
    if !baseurl.is_empty()
        && (!baseurl.starts_with('/')
            || baseurl.ends_with('/')
            || baseurl.starts_with("//")
            || baseurl.contains(['?', '#', '\\'])
            || baseurl.split('/').any(|s| matches!(s, "." | "..")))
    {
        return Err(
            "baseurl must be empty or an absolute site prefix without trailing slash".into(),
        );
    }
    Ok(())
}

pub(crate) fn check(root: &Path, site: &Path, baseurl: &str) -> Result<Report, String> {
    validate_baseurl(baseurl)?;
    let (pages, mut errors) = source::load(root)?;
    let site = site.canonicalize().map_err(|e| e.to_string())?;
    let mut documents = BTreeMap::new();
    let mut output_files = BTreeSet::new();
    for path in files::walk(&site, false)? {
        let relative = files::relative(&site, &path);
        output_files.insert(relative.clone());
        if path.extension().and_then(|s| s.to_str()) == Some("map") {
            errors.push(format!("debug source map in public output: {relative}"));
        }
        if path.extension().and_then(|s| s.to_str()) == Some("html") {
            let url = if path.file_name().and_then(|s| s.to_str()) == Some("index.html") {
                format!(
                    "/{}",
                    relative.strip_suffix("index.html").expect("index suffix")
                )
            } else {
                format!("/{relative}")
            };
            documents.insert(url, parse(&files::read(&path)?));
        }
    }
    for page in &pages {
        let Some(d) = documents.get(&page.url) else {
            errors.push(format!(
                "{}: missing rendered page {}",
                page.path.display(),
                page.url
            ));
            continue;
        };
        if d.main != 1 || d.h1 != 1 || !d.english {
            errors.push(format!(
                "{}: expected one main/h1 and English language",
                page.url
            ));
        }
        let expected = format!("https://humanymous.net{baseurl}{}", page.url);
        if d.canonical != [expected.clone()] {
            errors.push(format!("{}: canonical mismatch", page.url));
        }
        errors.extend(d.errors.iter().map(|e| format!("{}: {e}", page.url)));
        let base = Url::parse(&expected).map_err(|e| e.to_string())?;
        for link in &d.links {
            if link.starts_with("//") || Url::parse(link).is_ok() {
                continue;
            }
            let target = base.join(link).map_err(|e| e.to_string())?;
            let decoded = percent_decode_str(target.path())
                .decode_utf8()
                .map_err(|e| e.to_string())?;
            if !baseurl.is_empty() && !decoded.starts_with(&format!("{baseurl}/")) {
                errors.push(format!("{}: link escapes baseurl {link}", page.url));
                continue;
            }
            let pathname = decoded.strip_prefix(baseurl).unwrap_or(&decoded);
            let invalid = pathname.contains(['\\', '\0'])
                || pathname.split('/').any(|part| matches!(part, "." | ".."));
            let key = if pathname.ends_with('/') {
                format!("{}index.html", pathname.trim_start_matches('/'))
            } else {
                pathname.trim_start_matches('/').to_owned()
            };
            if invalid || !output_files.contains(&key) {
                errors.push(format!(
                    "{}: missing or escaping built target {link}",
                    page.url
                ));
            }
            if let (Some(fragment), Some(other)) = (target.fragment(), documents.get(pathname)) {
                let fragment = percent_decode_str(fragment)
                    .decode_utf8()
                    .map_err(|e| e.to_string())?;
                if !fragment.is_empty() && !other.ids.contains(fragment.as_ref()) {
                    errors.push(format!("{}: missing anchor {link}", page.url));
                }
            }
        }
    }
    for private in [
        "AGENTS.md",
        "INTEGRATION.md",
        "Gemfile",
        "Gemfile.lock",
        "reviews.json",
        "scripts",
        "tooling",
        "assets/llms",
        "assets/og",
        "assets/screenshots",
        "images",
    ] {
        if site.join(private).exists() {
            errors.push(format!("non-public/retired output exists: {private}"));
        }
    }
    errors.sort();
    errors.dedup();
    Ok(Report {
        pages: pages.len(),
        errors,
    })
}
