//! Named window controls share pointer, keyboard and AccessKit activation paths.
use crate::icons::{self, Icon};
use meari::domain::{Attribute, Node, NodeKind, Span, Value};
use meari_native::infrastructure::document::InteractiveDocument;

/// Intent for the current owned window, never target-page authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowCommand {
    /// Minimize the host.
    Minimize,
    /// Toggle maximize/restore.
    ToggleMaximize,
    /// Request coordinated CEF and host close.
    Close,
}

const BUTTONS: [(&str, &str, Icon, WindowCommand); 3] = [
    (
        "window-minimize",
        "Minimize window",
        Icon::Minimize,
        WindowCommand::Minimize,
    ),
    (
        "window-maximize",
        "Maximize or restore window",
        Icon::Maximize,
        WindowCommand::ToggleMaximize,
    ),
    (
        "window-close",
        "Close window",
        Icon::Close,
        WindowCommand::Close,
    ),
];

/// Render the local title-strip controls. Standalone management preview disables them.
pub fn controls(enabled: bool) -> Node {
    fn attr(name: &str, value: &str) -> Attribute {
        Attribute {
            name: name.into(),
            value: Value::String(value.into()),
        }
    }
    Node {
        span: Span::default(),
        kind: NodeKind::Element {
            tag: "div".into(),
            attributes: vec![attr("className", "window-controls")],
            children: BUTTONS
                .iter()
                .map(|(id, label, icon, _)| {
                    let mut attributes = vec![
                        attr("id", id),
                        attr("aria-label", label),
                        attr("title", label),
                        attr("className", "window-button"),
                    ];
                    if !enabled {
                        attributes.push(attr("disabled", "true"));
                    }
                    Node {
                        span: Span::default(),
                        kind: NodeKind::Element {
                            tag: "button".into(),
                            attributes,
                            children: vec![icons::node(*icon)],
                        },
                    }
                })
                .collect(),
        },
    }
}

/// Read new local button requests once; unknown paths cannot become native commands.
pub fn commands_since(doc: &InteractiveDocument, after: usize) -> (usize, Vec<WindowCommand>) {
    fn paths(node: &Node, path: String, output: &mut Vec<(String, WindowCommand)>) {
        if let NodeKind::Element {
            attributes,
            children,
            ..
        } = &node.kind
        {
            for (id, _, _, command) in BUTTONS {
                if attributes
                    .iter()
                    .any(|a| a.name == "id" && a.value == Value::String(id.into()))
                {
                    output.push((path.clone(), command));
                }
            }
            for (index, child) in children.iter().enumerate() {
                paths(child, format!("{path}/{index}"), output);
            }
        }
    }
    let mut paths_found = Vec::new();
    paths(&doc.source.root, "0".into(), &mut paths_found);
    let commands = doc
        .activation_requests
        .iter()
        .skip(after)
        .take(16)
        .filter_map(|path| {
            paths_found
                .iter()
                .find(|(known, _)| known == path)
                .map(|(_, command)| *command)
        })
        .collect();
    (doc.activation_requests.len(), commands)
}

/// Eight app-owned hit strips stay outside the actual CEF child.
pub fn resize_regions() -> Node {
    Node {
        span: Span::default(),
        kind: NodeKind::Element {
            tag: "div".into(),
            attributes: vec![],
            children: ["n", "s", "e", "w", "nw", "ne", "sw", "se"]
                .iter()
                .map(|edge| Node {
                    span: Span::default(),
                    kind: NodeKind::Element {
                        tag: "div".into(),
                        attributes: vec![
                            Attribute {
                                name: "className".into(),
                                value: Value::String(format!("resize-hit resize-{edge}")),
                            },
                            Attribute {
                                name: "aria-hidden".into(),
                                value: Value::String("true".into()),
                            },
                        ],
                        children: vec![],
                    },
                })
                .collect(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blitz_dom::Document;
    use meari_native::{domain::Viewport, infrastructure::accessibility};

    #[test]
    fn native_window_buttons_route_ax_once_and_ignore_unknown_paths() {
        let mut doc = InteractiveDocument::new(
            crate::ui::component_with_window_controls(true),
            crate::ui::management_styles(),
            Viewport {
                width: 1100,
                height: 720,
                dark: true,
            },
        )
        .unwrap();
        let mut cursor = 0;
        for (id, _, _, expected) in BUTTONS {
            let target = doc.renderer.document.get_element_by_id(id).unwrap();
            accessibility::act(
                &mut doc,
                &accesskit::ActionRequest {
                    action: accesskit::Action::Click,
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: accesskit::NodeId(target.as_u64()),
                    data: None,
                },
            )
            .unwrap();
            let (next, commands) = commands_since(&doc, cursor);
            assert_eq!(commands, vec![expected]);
            cursor = next;
            assert!(commands_since(&doc, cursor).1.is_empty());
        }
        doc.activation_requests
            .push("target-page/window-close".into());
        assert!(commands_since(&doc, cursor).1.is_empty());
        assert!(doc.file_requests.is_empty());
        assert!(doc.callback_scope().is_none());
    }

    #[test]
    fn keyboard_close_uses_same_named_button_path() {
        use blitz_traits::events::{BlitzKeyEvent, KeyState, UiEvent};
        use keyboard_types::{Code, Key, Location, Modifiers, NamedKey};
        let mut doc = InteractiveDocument::new(
            crate::ui::component_with_window_controls(true),
            crate::ui::management_styles(),
            Viewport {
                width: 1100,
                height: 720,
                dark: true,
            },
        )
        .unwrap();
        let close = doc
            .renderer
            .document
            .get_element_by_id("window-close")
            .unwrap();
        doc.renderer.document.set_focus_to(close);
        doc.handle_ui_event(UiEvent::KeyDown(BlitzKeyEvent {
            key: Key::Named(NamedKey::Enter),
            code: Code::Enter,
            modifiers: Modifiers::empty(),
            location: Location::Standard,
            is_auto_repeating: false,
            is_composing: false,
            state: KeyState::Pressed,
            text: None,
        }));
        assert_eq!(commands_since(&doc, 0).1, vec![WindowCommand::Close]);
        assert!(doc.last_error.is_none());
    }
}
