//! Static qualification document generated from Rust Meari IR, never a host UI.
use meari::domain::{Attribute, Component, Node, NodeKind, Span, Value};

fn node(tag: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Node {
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

pub fn markup() -> Result<String, String> {
    let component = Component {
        hook_order: vec![],
        states: vec![],
        client: false,
        name: "SyntheticCefInputFixture".into(),
        props: vec![],
        root: node(
            "div",
            &[],
            vec![
                node("h2", &[], vec![text("Synthetic CEF input fixture")]),
                node(
                    "Label",
                    &[("htmlFor", "cef-synthetic-input")],
                    vec![text("CEF synthetic text")],
                ),
                node(
                    "Input",
                    &[
                        ("id", "cef-synthetic-input"),
                        ("aria-label", "CEF synthetic text"),
                        ("defaultValue", "synthetic"),
                    ],
                    vec![],
                ),
                node(
                    "p",
                    &[],
                    vec![text(
                        "Local generated test data. No scripts, network resources or native bridge.",
                    )],
                ),
            ],
        ),
    };
    let session = meari_native::domain::widgets::Session::from_tree(&component.root)?;
    let html = meari_native::application::markup::build(&component, &session)?.html;
    if html.len() > 16384 || html.to_lowercase().contains("<script") {
        return Err("fixture markup outside bounds".into());
    }
    Ok(html)
}
pub fn uri() -> Result<String, String> {
    let encoded = cef::CefString::from(&cef::base64_encode(Some(markup()?.as_bytes()))).to_string();
    Ok(format!("data:text/html;charset=utf-8;base64,{encoded}"))
}

pub fn management_document()
-> Result<meari_native::infrastructure::document::InteractiveDocument, String> {
    fn label(node: &mut Node) {
        fn rename(value: &mut Value) {
            if let Value::String(text) = value {
                *text = match text.as_str() {
                    "Profile: synthetic-ephemeral-01 | Target tabs: 0" => "Fresh synthetic profile | CEF qualification child: 1 requested",
                    "Navigation unavailable: no sandbox-qualified CEF adapter attached." => "External browsing disabled. Local generated fixture only; browser NOT READY.",
                    "CEF attachment / sandbox / focus / IME / AX: unqualified" => "Native CEF child pilot | sandbox / keyboard / IME / AX qualification pending",
                    _ => return,
                }.into();
            }
        }
        match &mut node.kind {
            NodeKind::Text(value) => rename(value),
            NodeKind::Element {
                attributes,
                children,
                ..
            } => {
                for attribute in attributes {
                    rename(&mut attribute.value);
                }
                for child in children {
                    label(child);
                }
            }
        }
    }
    let mut component = humanymous_browser::ui::component();
    label(&mut component.root);
    meari_native::infrastructure::document::InteractiveDocument::new(
        component,
        humanymous_browser::ui::MANAGEMENT_STYLES,
        meari_native::domain::Viewport {
            width: 1100,
            height: 1000,
            dark: true,
        },
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn static_rust_ir_fixture_has_named_native_input_and_no_script_or_subresources() {
        let markup = super::markup().unwrap();
        assert!(markup.contains("CEF synthetic text"));
        assert!(markup.contains("cef-synthetic-input"));
        for forbidden in ["<script", "src=", "href=", "onkeydown=", "onclick="] {
            assert!(!markup.to_lowercase().contains(forbidden));
        }
    }
}
