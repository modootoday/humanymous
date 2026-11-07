//! Static invariants of the reviewed theme, not browser or AT qualification.
use scraper::{ElementRef, Html, Selector};

fn select(value: &str) -> Selector {
    Selector::parse(value).expect("constant accessibility selector")
}

fn explicitly_hidden(element: ElementRef<'_>) -> bool {
    std::iter::once(element)
        .chain(element.ancestors().filter_map(ElementRef::wrap))
        .any(|element| {
            element.value().attr("hidden").is_some()
                || element.value().attr("aria-hidden") == Some("true")
        })
}

pub(crate) fn check(html: &Html) -> Vec<String> {
    let mut errors = Vec::new();
    let titles: Vec<_> = html.select(&select("head title")).collect();
    if titles.len() != 1 || titles[0].text().all(|text| text.trim().is_empty()) {
        errors.push("expected one nonempty document title".into());
    }
    let usable_skip = html.select(&select("a.mt-skip")).any(|link| {
        if explicitly_hidden(link) || link.text().all(|text| text.trim().is_empty()) {
            return false;
        }
        let Some(id) = link
            .value()
            .attr("href")
            .and_then(|href| href.strip_prefix('#'))
        else {
            return false;
        };
        let Some(target) = html
            .select(&select("[id]"))
            .find(|element| element.value().attr("id") == Some(id))
        else {
            return false;
        };
        !id.is_empty()
            && !explicitly_hidden(target)
            && target.value().attr("tabindex") == Some("-1")
            && std::iter::once(target)
                .chain(target.ancestors().filter_map(ElementRef::wrap))
                .any(|element| element.value().name() == "main")
    });
    if !usable_skip {
        errors.push("missing static theme skip link to a focus target in main".into());
    }
    errors
}
