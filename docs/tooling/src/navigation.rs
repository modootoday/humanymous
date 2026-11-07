//! Validate the deliberately small authored menu grammar, not arbitrary YAML.
use crate::files;
use regex::Regex;
use std::{collections::BTreeSet, path::Path, sync::LazyLock};

static LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\{title: [A-Za-z0-9][A-Za-z0-9 /?&()_-]*, url: (/[^\s,{}\[\]#"']*)\}$"#)
        .expect("constant menu link grammar")
});
static TITLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9][A-Za-z0-9 /?&()_-]*$").expect("constant menu title grammar")
});

pub(crate) fn check(root: &Path, urls: &BTreeSet<String>, errors: &mut Vec<String>) {
    let path = root.join("_data/nav/main.yml");
    let text = match files::read(&path) {
        Ok(text) => text,
        Err(_) => {
            errors.push("navigation: missing or unreadable _data/nav/main.yml".into());
            return;
        }
    };
    let mut group: Option<usize> = None;
    let mut waiting_items = false;
    let mut links = 0;
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let failure = || format!("navigation:{}: invalid authored menu structure", index + 1);
        let link = if let Some(value) = line.strip_prefix("- ") {
            if waiting_items || group == Some(0) {
                errors.push(failure());
            }
            group = None;
            waiting_items = false;
            if let Some(title) = value.strip_prefix("title: ") {
                if !TITLE.is_match(title) {
                    errors.push(failure());
                }
                group = Some(0);
                waiting_items = true;
                continue;
            }
            value
        } else if line == "  items:" && waiting_items {
            waiting_items = false;
            continue;
        } else if let Some(value) = line.strip_prefix("    - ") {
            if waiting_items || group.is_none() {
                errors.push(failure());
            }
            if let Some(count) = &mut group {
                *count += 1;
            }
            value
        } else {
            errors.push(failure());
            continue;
        };
        match LINK.captures(link) {
            Some(capture) => {
                links += 1;
                if !urls.contains(&capture[1]) {
                    errors.push(format!(
                        "navigation:{}: unknown page {}",
                        index + 1,
                        &capture[1]
                    ));
                }
            }
            None => errors.push(failure()),
        }
    }
    if waiting_items || group == Some(0) || links == 0 {
        errors.push("navigation: missing items or empty menu/group".into());
    }
}
