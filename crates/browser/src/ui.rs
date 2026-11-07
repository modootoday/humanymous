//! Meari typed native management UI; no target-content surface is simulated.
use meari::domain::{Attribute, Component, Node, NodeKind, Span, Value};
use meari_native::{domain::Viewport, infrastructure::document::InteractiveDocument};

fn text(value: &str) -> Node {
    Node {
        kind: NodeKind::Text(Value::String(value.into())),
        span: Span::default(),
    }
}

fn element(tag: &str, attributes: &[(&str, &str)], children: Vec<Node>) -> Node {
    let mut attributes: Vec<_> = attributes
        .iter()
        .map(|(name, value)| Attribute {
            name: (*name).into(),
            value: Value::String((*value).into()),
        })
        .collect();
    // Explicit names compensate for the pinned renderer's incomplete static-text
    // name propagation into Windows UIA. This grants no native engine capability.
    if matches!(tag, "h1" | "h2" | "p" | "Label") {
        let name = children
            .iter()
            .filter_map(|node| match &node.kind {
                NodeKind::Text(Value::String(value)) => Some(value.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        attributes.push(Attribute {
            name: "aria-label".into(),
            value: Value::String(name),
        });
    }
    Node {
        kind: NodeKind::Element {
            tag: tag.into(),
            attributes,
            children,
        },
        span: Span::default(),
    }
}

/// Synthetic management controls. Address edits remain local native input state.
pub fn component() -> Component {
    Component {
        hook_order: vec![],
        states: vec![],
        client: false,
        name: "LaboratoryBrowser".into(),
        props: vec![],
        root: element(
            "div",
            &[("className", "shell")],
            vec![
                element("h1", &[], vec![text("humanymous browser laboratory")]),
                element(
                    "p",
                    &[],
                    vec![text("Synthetic profile only | CEF: NOT READY")],
                ),
                element(
                    "Label",
                    &[("htmlFor", "address")],
                    vec![text("Owned fixture address (editing only)")],
                ),
                element(
                    "Input",
                    &[
                        ("id", "address"),
                        ("defaultValue", "https://fixture.invalid/"),
                        ("aria-label", "Owned fixture address"),
                    ],
                    vec![],
                ),
                element(
                    "p",
                    &[("role", "status")],
                    vec![text(
                        "Navigation unavailable: no sandbox-qualified CEF adapter attached.",
                    )],
                ),
                element(
                    "Tabs",
                    &[("defaultValue", "diagnostics")],
                    vec![
                        element(
                            "TabsList",
                            &[("aria-label", "Laboratory panels")],
                            vec![
                                element(
                                    "TabsTrigger",
                                    &[("id", "diagnostics-tab"), ("value", "diagnostics")],
                                    vec![text("Diagnostics")],
                                ),
                                element(
                                    "TabsTrigger",
                                    &[("id", "scenario-tab"), ("value", "scenario")],
                                    vec![text("Scenario")],
                                ),
                                element(
                                    "TabsTrigger",
                                    &[("id", "audit-tab"), ("value", "audit")],
                                    vec![text("Audit")],
                                ),
                            ],
                        ),
                        element(
                            "TabsContent",
                            &[("value", "diagnostics")],
                            vec![
                                element("h2", &[], vec![text("Engine and profile")]),
                                element(
                                    "p",
                                    &[],
                                    vec![text("Profile: synthetic-ephemeral-01 | Target tabs: 0")],
                                ),
                                element(
                                    "p",
                                    &[],
                                    vec![text(
                                        "CEF attachment / sandbox / focus / IME / AX: unqualified",
                                    )],
                                ),
                                element(
                                    "p",
                                    &[],
                                    vec![text(
                                        "Filesystem, capture, popup, administration, bearer and native bridge: DENY",
                                    )],
                                ),
                            ],
                        ),
                        element(
                            "TabsContent",
                            &[("value", "scenario")],
                            vec![
                                element("h2", &[], vec![text("Owned scenario")]),
                                element(
                                    "p",
                                    &[],
                                    vec![text(
                                        "Not connected. No requests sent. No detector outcome measured.",
                                    )],
                                ),
                            ],
                        ),
                        element(
                            "TabsContent",
                            &[("value", "audit")],
                            vec![
                                element("h2", &[], vec![text("Server audit")]),
                                element(
                                    "p",
                                    &[],
                                    vec![text(
                                        "Not connected. No server verdict or origin effect receipt.",
                                    )],
                                ),
                            ],
                        ),
                    ],
                ),
                element(
                    "p",
                    &[],
                    vec![text(
                        "Keyboard: Tab to address or panels; arrow keys switch panels. Close uses the native window control.",
                    )],
                ),
            ],
        ),
    }
}

/// Build the actual native document, with no parser or foreign application source.
pub fn document(width: u32, height: u32) -> Result<InteractiveDocument, String> {
    InteractiveDocument::new(
        component(),
        "body{background:#101826;color:#edf3ff;font-family:system-ui;font-size:16px} .shell{padding:28px;max-width:1000px} h1{font-size:26px} input{padding:10px;border:1px solid #7890aa;border-radius:6px;background:#18263a;color:#edf3ff} button{padding:12px;background:#203652;color:#edf3ff} [hidden]{display:none!important}",
        Viewport {
            width,
            height,
            dark: true,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use blitz_dom::Document;
    use blitz_traits::events::{BlitzKeyEvent, KeyState, UiEvent};
    use keyboard_types::{Code, Key, Location, Modifiers, NamedKey};
    use meari_native::infrastructure::accessibility;

    #[test]
    fn keyboard_roving_tabs_and_accessibility_actions_reach_real_document() {
        let mut doc = document(1100, 720).unwrap();
        let diagnostics = doc
            .renderer
            .document
            .get_element_by_id("diagnostics-tab")
            .unwrap();
        doc.renderer.document.set_focus_to(diagnostics);
        doc.handle_ui_event(UiEvent::KeyDown(BlitzKeyEvent {
            key: Key::Named(NamedKey::ArrowRight),
            code: Code::ArrowRight,
            modifiers: Modifiers::empty(),
            location: Location::Standard,
            is_auto_repeating: false,
            is_composing: false,
            state: KeyState::Pressed,
            text: None,
        }));
        assert!(doc.last_error.is_none(), "{:?}", doc.last_error);
        let scenario = doc
            .renderer
            .document
            .get_element_by_id("scenario-tab")
            .unwrap();
        assert_eq!(doc.renderer.document.get_focussed_node_id(), Some(scenario));
        let audit = doc
            .renderer
            .document
            .get_element_by_id("audit-tab")
            .unwrap();
        accessibility::act(
            &mut doc,
            &accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: accesskit::NodeId(audit.as_u64()),
                data: None,
            },
        )
        .unwrap();
        assert!(
            doc.session
                .widgets
                .values()
                .any(|widget| widget.value == "audit")
        );
    }

    #[test]
    fn accessible_address_edit_has_no_navigation_or_authority_side_effect() {
        let mut doc = document(1100, 720).unwrap();
        let address = doc.renderer.document.get_element_by_id("address").unwrap();
        accessibility::act(
            &mut doc,
            &accesskit::ActionRequest {
                action: accesskit::Action::SetValue,
                target_tree: accesskit::TreeId::ROOT,
                target_node: accesskit::NodeId(address.as_u64()),
                data: Some(accesskit::ActionData::Value(
                    "https://fixture.invalid/synthetic".into(),
                )),
            },
        )
        .unwrap();
        assert!(
            doc.input_values
                .values()
                .any(|value| value == "https://fixture.invalid/synthetic")
        );
        assert!(doc.activation_requests.is_empty());
        assert!(doc.file_requests.is_empty());
        assert!(doc.last_error.is_none());
        assert!(doc.callback_scope().is_none());
    }

    #[test]
    fn native_document_accepts_bounded_window_sizes_without_target_surface() {
        for (width, height) in [(640, 480), (1100, 720), (1920, 1080)] {
            let doc = document(width, height).unwrap();
            assert_eq!(doc.viewport.width, width);
            assert!(doc.renderer.document.get_element_by_id("address").is_some());
            assert!(doc.file_requests.is_empty());
        }
    }

    #[test]
    fn ime_commit_stays_in_native_address_without_triggering_target_navigation() {
        use blitz_traits::events::BlitzImeEvent;
        let mut doc = document(1100, 720).unwrap();
        let address = doc.renderer.document.get_element_by_id("address").unwrap();
        doc.renderer.document.set_focus_to(address);
        doc.handle_ui_event(UiEvent::Ime(BlitzImeEvent::Enabled));
        doc.handle_ui_event(UiEvent::Ime(BlitzImeEvent::Preedit(
            "한".into(),
            Some((0, 3)),
        )));
        doc.handle_ui_event(UiEvent::Ime(BlitzImeEvent::Preedit(String::new(), None)));
        doc.handle_ui_event(UiEvent::Ime(BlitzImeEvent::Commit("한글".into())));
        assert!(doc.last_error.is_none(), "{:?}", doc.last_error);
        assert!(
            doc.input_values
                .values()
                .any(|value| value.contains("한글"))
        );
        assert!(doc.activation_requests.is_empty());
    }
}
