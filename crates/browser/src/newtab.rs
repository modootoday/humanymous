//! Script-free owner page built from typed Meari IR, serialized by html5ever.
use html5ever::{
    LocalName, Namespace, QualName,
    serialize::{Serialize, SerializeOpts, Serializer, TraversalScope},
};
use meari::application::dom::DomNode;
use meari::domain::{Attribute, Component, Node, NodeKind, Span, Value};
use std::io;

fn element(tag: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Node {
    Node {
        span: Span::default(),
        kind: NodeKind::Element {
            tag: tag.into(),
            attributes: attrs
                .iter()
                .map(|(name, value)| Attribute {
                    name: (*name).into(),
                    value: Value::String((*value).into()),
                })
                .collect(),
            children,
        },
    }
}
fn text(value: &str) -> Node {
    Node {
        span: Span::default(),
        kind: NodeKind::Text(Value::String(value.into())),
    }
}

/// The complete landing-page IR. No network assets, executable source or recents.
pub fn component() -> Component {
    let cards = crate::internal_urls::InternalPage::ALL.into_iter().map(|page| {
        let description = match page {
            crate::internal_urls::InternalPage::Version => "Engine and build information",
            crate::internal_urls::InternalPage::Gpu => "Graphics and rendering diagnostics",
            crate::internal_urls::InternalPage::Credits => "Open-source acknowledgements",
            crate::internal_urls::InternalPage::License => "Chromium and CEF license",
            crate::internal_urls::InternalPage::Sandbox => "Process sandbox diagnostics",
        };
        element("a", &[("href",page.action()), ("style","display:block;flex:1 1 170px;max-width:270px;padding:22px;border:1px solid #2b3d52;border-radius:16px;background:#141f2e;color:#e5edf7;text-decoration:none;box-sizing:border-box")], vec![
            element("strong", &[("style","display:block;font-size:16px;margin-bottom:9px")], vec![text(page.label())]),
            element("span", &[("style","font-size:13px;line-height:1.6;color:#9fb1c5")], vec![text(description)]),
        ])
    }).collect();
    Component {
        name: "LocalNewTab".into(),
        props: vec![],
        states: vec![],
        hook_order: vec![],
        client: false,
        root: element(
            "main",
            &[(
                "style",
                "min-height:100vh;box-sizing:border-box;padding:clamp(24px,4vw,52px);font-family:system-ui,sans-serif;background:radial-gradient(ellipse at 20% 0%,#1b3545 0%,#0d1520 52%);color:#e8eff8",
            )],
            vec![
                element(
                    "header",
                    &[(
                        "style",
                        "max-width:1040px;margin:auto;display:flex;align-items:center;gap:12px",
                    )],
                    vec![
                        element(
                            "span",
                            &[
                                ("aria-hidden", "true"),
                                (
                                    "style",
                                    "display:grid;place-items:center;width:36px;height:36px;border-radius:12px;background:#70dbc2;color:#102b2b;font-size:27px;font-weight:750",
                                ),
                            ],
                            vec![text("h")],
                        ),
                        element(
                            "strong",
                            &[("style", "font-size:15px;letter-spacing:.03em")],
                            vec![text("humanymous")],
                        ),
                        element(
                            "span",
                            &[(
                                "style",
                                "margin-left:auto;color:#9dc4c5;font-size:12px;border:1px solid #3b5b62;border-radius:20px;padding:7px 12px",
                            )],
                            vec![text("LOCAL WORKSPACE")],
                        ),
                    ],
                ),
                element(
                    "section",
                    &[(
                        "style",
                        "max-width:900px;margin:clamp(32px,6vh,72px) auto 20px",
                    )],
                    vec![
                        element(
                            "p",
                            &[(
                                "style",
                                "font-size:12px;letter-spacing:.16em;color:#8fbaaF;text-transform:uppercase",
                            )],
                            vec![text("A closer look at the browser")],
                        ),
                        element(
                            "h1",
                            &[(
                                "style",
                                "font-size:clamp(34px,5vw,56px);line-height:1.1;letter-spacing:-.04em;margin:16px 0",
                            )],
                            vec![text("Start with a clear view.")],
                        ),
                        element(
                            "p",
                            &[(
                                "style",
                                "font-size:16px;line-height:1.7;color:#a6b7cb;max-width:650px",
                            )],
                            vec![text(
                                "Open a local diagnostic or use the address bar. Your engine, its observations, and what a website can see stay distinct.",
                            )],
                        ),
                        element(
                            "form",
                            &[
                                ("action", "/open/"),
                                ("method", "get"),
                                (
                                    "style",
                                    "display:flex;flex-wrap:wrap;gap:10px;margin:30px 0 12px;padding:8px;background:#111d2b;border:1px solid #3f586e;border-radius:16px",
                                ),
                            ],
                            vec![
                                element(
                                    "input",
                                    &[
                                        ("name", "address"),
                                        ("type", "text"),
                                        ("placeholder", "chrome://version"),
                                        ("aria-label", "Open a local diagnostic address"),
                                        ("autocomplete", "off"),
                                        ("maxlength", "4096"),
                                        ("required", "true"),
                                        (
                                            "style",
                                            "flex:1 1 220px;min-width:0;border:0;outline-offset:4px;background:transparent;color:#eef4fb;font:inherit;padding:12px 14px",
                                        ),
                                    ],
                                    vec![],
                                ),
                                element(
                                    "button",
                                    &[
                                        ("type", "submit"),
                                        (
                                            "style",
                                            "border:0;border-radius:10px;padding:12px 24px;background:#79dcc5;color:#102f2b;font:inherit;font-weight:650;cursor:pointer",
                                        ),
                                    ],
                                    vec![text("Open")],
                                ),
                            ],
                        ),
                        element(
                            "p",
                            &[("style", "font-size:12px;color:#8399ae;margin-bottom:34px")],
                            vec![text(
                                "Local diagnostics only · External browsing is locked · No history or tracking",
                            )],
                        ),
                        element(
                            "nav",
                            &[
                                ("aria-label", "Local diagnostics"),
                                ("style", "display:flex;flex-wrap:wrap;gap:12px"),
                            ],
                            cards,
                        ),
                        element(
                            "footer",
                            &[(
                                "style",
                                "margin-top:28px;padding-top:20px;border-top:1px solid #263747;font-size:12px;line-height:1.7;color:#8298ad",
                            )],
                            vec![text(
                                "Sandbox required. Full qualification and external browsing remain pending. Diagnostic pages may display local paths or device details.",
                            )],
                        ),
                    ],
                ),
            ],
        ),
    }
}

fn name(tag: &str) -> QualName {
    QualName::new(
        None,
        Namespace::from("http://www.w3.org/1999/xhtml"),
        LocalName::from(tag),
    )
}
fn visit<S: Serializer>(node: &DomNode, serializer: &mut S) -> io::Result<()> {
    match node {
        DomNode::Text(text) => serializer.write_text(text),
        DomNode::Element {
            tag,
            attributes,
            children,
        } => {
            let attrs: Vec<_> = attributes
                .iter()
                .map(|(key, value)| {
                    (
                        QualName::new(None, Namespace::from(""), LocalName::from(key.as_str())),
                        value.as_str(),
                    )
                })
                .collect();
            serializer.start_elem(name(tag), attrs.iter().map(|(key, value)| (key, *value)))?;
            for child in children {
                visit(child, serializer)?;
            }
            serializer.end_elem(name(tag))
        }
    }
}
struct Document(DomNode);
impl Serialize for Document {
    fn serialize<S: Serializer>(
        &self,
        serializer: &mut S,
        _scope: TraversalScope,
    ) -> io::Result<()> {
        serializer.write_doctype("html")?;
        serializer.start_elem(name("html"), std::iter::empty())?;
        serializer.start_elem(name("head"), std::iter::empty())?;
        let charset = QualName::new(None, Namespace::from(""), LocalName::from("charset"));
        serializer.start_elem(name("meta"), [(&charset, "utf-8")].into_iter())?;
        serializer.end_elem(name("meta"))?;
        serializer.start_elem(name("title"), std::iter::empty())?;
        serializer.write_text("New tab · humanymous")?;
        serializer.end_elem(name("title"))?;
        serializer.end_elem(name("head"))?;
        let style = QualName::new(None, Namespace::from(""), LocalName::from("style"));
        serializer.start_elem(name("body"), [(&style, "margin:0")].into_iter())?;
        visit(&self.0, serializer)?;
        serializer.end_elem(name("body"))?;
        serializer.end_elem(name("html"))
    }
}
/// Bounded UTF-8 document. Escaping/HTML syntax belong to upstream serializers.
pub fn markup() -> Result<String, String> {
    let tree = meari::application::dom::prepare(&component())?;
    let mut bytes = Vec::new();
    html5ever::serialize(
        &mut bytes,
        &Document(tree),
        SerializeOpts {
            scripting_enabled: false,
            traversal_scope: TraversalScope::IncludeNode,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("newtab document budget exceeded".into());
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn landing_page_has_real_form_and_relative_links_without_script_or_remote_assets() {
        let markup = super::markup().unwrap();
        assert!(markup.contains("Start with a clear view."));
        assert!(markup.contains("action=\"/open/\""));
        assert!(markup.contains("href=\"/open/version\""));
        assert!(markup.contains("name=\"address\""));
        for denied in [
            "<script",
            "onclick=",
            "onkeydown=",
            "src=",
            "https://",
            "http://",
            "iframe",
            "chrome://quit",
            "chrome://chrome-urls",
        ] {
            assert!(!markup.contains(denied), "{denied}");
        }
        assert_eq!(markup, super::markup().unwrap());
    }
}
