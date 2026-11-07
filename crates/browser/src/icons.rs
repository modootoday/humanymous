//! Small Rust-authored SVG icon library. No font glyphs, scripts or remote assets.
use meari::domain::{Attribute, Node, NodeKind, Span, Value};

/// Fixed browser and window symbols; never accepts untrusted SVG markup.
#[derive(Clone, Copy)]
pub enum Icon {
    /// Navigate back.
    Back,
    /// Navigate forward.
    Forward,
    /// Reload current page.
    Reload,
    /// Add a tab.
    Plus,
    /// Minimize the window.
    Minimize,
    /// Toggle maximization.
    Maximize,
    /// Close the window.
    Close,
}

/// Build decorative vector geometry; the containing button supplies its AX name.
pub fn node(icon: Icon) -> Node {
    fn element(tag: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Node {
        Node {
            span: Span::default(),
            kind: NodeKind::Element {
                tag: tag.into(),
                attributes: attrs
                    .iter()
                    .map(|(key, value)| Attribute {
                        name: (*key).into(),
                        value: Value::String((*value).into()),
                    })
                    .collect(),
                children,
            },
        }
    }
    let path = match icon {
        Icon::Back => "M19 12H5M11 6l-6 6 6 6",
        Icon::Forward => "M5 12h14M13 6l6 6-6 6",
        Icon::Reload => "M19 8a8 8 0 1 0 1 7M19 3v5h-5",
        Icon::Plus => "M12 5v14M5 12h14",
        Icon::Minimize => "M5 12h14",
        Icon::Maximize => "M5 5h14v14H5Z",
        Icon::Close => "M6 6l12 12M18 6 6 18",
    };
    element(
        "svg",
        &[
            ("viewBox", "0 0 24 24"),
            ("width", "18"),
            ("height", "18"),
            ("fill", "none"),
            ("stroke", "#d8e7fb"),
            (
                "style",
                "stroke-width:1.8;stroke-linecap:round;stroke-linejoin:round",
            ),
            ("aria-hidden", "true"),
        ],
        vec![element("path", &[("d", path)], vec![])],
    )
}
