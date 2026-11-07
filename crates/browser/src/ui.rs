//! Meari typed native management UI; no target-content surface is simulated.
use meari::domain::{Attribute, Component, Node, NodeKind, Span, Value};
use meari_native::{domain::Viewport, infrastructure::document::InteractiveDocument};

pub(crate) fn text(value: &str) -> Node {
    Node {
        kind: NodeKind::Text(Value::String(value.into())),
        span: Span::default(),
    }
}

pub(crate) fn element(tag: &str, attributes: &[(&str, &str)], children: Vec<Node>) -> Node {
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
    if attributes.iter().any(|attribute| {
        attribute.name == "id" && attribute.value == Value::String("browser-shell".into())
    }) {
        attributes.push(Attribute {
            name: "data-browser-ready".into(),
            value: Value::State("browserReady".into()),
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

pub(crate) fn navigation_button(
    id: &str,
    label: &str,
    icon: crate::icons::Icon,
    enabled: bool,
) -> Node {
    let mut node = element(
        "button",
        &[
            ("id", id),
            ("aria-label", label),
            ("title", label),
            ("className", "nav-button"),
        ],
        vec![crate::icons::node(icon)],
    );
    if !enabled && let NodeKind::Element { attributes, .. } = &mut node.kind {
        attributes.push(Attribute {
            name: "disabled".into(),
            value: Value::Bool(true),
        });
    }
    node
}

/// Browser-first native shell. Actual page pixels are owned by the CEF child.
pub fn component() -> Component {
    component_with_window_controls(false)
}

pub(crate) fn native_states() -> Vec<meari::domain::state_program::StateBinding> {
    vec![meari::domain::state_program::StateBinding {
        name: "browserReady".into(),
        setter: "setBrowserReady".into(),
        initial: meari::domain::component_state::StateValue::Bool(false),
        span: Span::default(),
    }]
}

/// Native pilot variant with owned window commands enabled.
pub fn component_with_window_controls(enabled: bool) -> Component {
    fn inspector(prefix: &str, dock: &str) -> Node {
        let tabs = [
            (
                "diagnostics",
                "Engine",
                "Local engine observations",
                "Observer sources are not connected. Missing events are unknown, not zero.",
            ),
            (
                "scenario",
                "Page exposure",
                "Website-visible evidence",
                "No page-exposure experiment has run. Engine-private facts are not website detection signals.",
            ),
            (
                "audit",
                "Server",
                "Optional server evidence",
                "Not integrated. No verified server verdict, receipt or origin effect is available.",
            ),
        ];
        let triggers = tabs
            .iter()
            .map(|(value, label, _, _)| {
                let id = format!("{prefix}{value}-tab");
                element(
                    "TabsTrigger",
                    &[("id", &id), ("value", value)],
                    vec![text(label)],
                )
            })
            .collect();
        let mut children = vec![element(
            "TabsList",
            &[
                ("aria-label", "Laboratory panels"),
                ("className", "analysis-tabs"),
            ],
            triggers,
        )];
        for (value, _, title, description) in tabs {
            children.push(element(
                "TabsContent",
                &[("value", value)],
                vec![
                    element("h2", &[], vec![text(title)]),
                    element("p", &[], vec![text(description)]),
                    element(
                        "p",
                        &[("className", "muted")],
                        vec![text(
                            "Required sandbox: partial evidence. Full qualification pending.",
                        )],
                    ),
                ],
            ));
        }
        element(
            "TabsContent",
            &[
                ("value", dock),
                ("className", &format!("inspector inspector-{dock}")),
            ],
            vec![
                element(
                    "div",
                    &[("className", "inspector-title")],
                    vec![text("Research inspector")],
                ),
                element(
                    "p",
                    &[("className", "muted")],
                    vec![text("Local fixture · bounded native pilot")],
                ),
                element("Tabs", &[("defaultValue", "diagnostics")], children),
            ],
        )
    }
    Component {
        hook_order: vec![],
        states: native_states(),
        client: false,
        name: "LaboratoryBrowser".into(),
        props: vec![],
        root: element(
            "Tabs",
            &[
                ("id", "browser-shell"),
                ("className", "browser-shell"),
                ("defaultValue", "hidden"),
            ],
            vec![
                element(
                    "div",
                    &[("className", "tab-strip")],
                    vec![
                        element("span", &[("className", "brand")], vec![text("h")]),
                        element(
                            "div",
                            &[("className", "tab-workspace")],
                            vec![
                                crate::tabs_ui::strip(enabled),
                                navigation_button(
                                    "tab-new",
                                    "New tab",
                                    crate::icons::Icon::Plus,
                                    enabled,
                                ),
                            ],
                        ),
                        element(
                            "span",
                            &[("className", "tab-caption")],
                            vec![text("humanymous research browser")],
                        ),
                        crate::window_chrome::controls(enabled),
                    ],
                ),
                element(
                    "div",
                    &[("className", "toolbar")],
                    vec![
                        navigation_button("nav-back", "Back", crate::icons::Icon::Back, false),
                        navigation_button(
                            "nav-forward",
                            "Forward",
                            crate::icons::Icon::Forward,
                            false,
                        ),
                        navigation_button(
                            "nav-home",
                            "New tab page",
                            crate::icons::Icon::Home,
                            enabled,
                        ),
                        navigation_button(
                            "nav-reload",
                            "Reload local page",
                            crate::icons::Icon::Reload,
                            enabled,
                        ),
                        navigation_button(
                            "nav-stop",
                            "Stop loading",
                            crate::icons::Icon::Close,
                            false,
                        ),
                        element(
                            "Input",
                            &[
                                ("id", "address"),
                                (
                                    "defaultValue",
                                    if enabled {
                                        crate::internal_urls::NEW_TAB_URL
                                    } else {
                                        "about:blank"
                                    },
                                ),
                                (
                                    "aria-label",
                                    if enabled {
                                        "Address — local pages only"
                                    } else {
                                        "Address — editing only; navigation locked"
                                    },
                                ),
                                ("className", "address"),
                            ],
                            vec![],
                        ),
                        navigation_button(
                            "nav-go",
                            "Open address",
                            crate::icons::Icon::Forward,
                            enabled,
                        ),
                        element(
                            "span",
                            &[
                                ("role", "status"),
                                ("id", "navigation-status"),
                                ("className", "mode-badge"),
                            ],
                            vec![text(if enabled { "LOCAL ONLY" } else { "NOT READY" })],
                        ),
                        element(
                            "TabsList",
                            &[
                                ("aria-label", "Analysis panel position"),
                                ("className", "dock-controls"),
                            ],
                            vec![
                                element(
                                    "TabsTrigger",
                                    &[
                                        ("id", "dock-hidden"),
                                        ("value", "hidden"),
                                        ("aria-label", "Hide analysis panel"),
                                    ],
                                    vec![text("Page")],
                                ),
                                element(
                                    "TabsTrigger",
                                    &[
                                        ("id", "dock-right"),
                                        ("value", "right"),
                                        ("aria-label", "Dock analysis panel right"),
                                    ],
                                    vec![text("Right")],
                                ),
                                element(
                                    "TabsTrigger",
                                    &[
                                        ("id", "dock-bottom"),
                                        ("value", "bottom"),
                                        ("aria-label", "Dock analysis panel bottom"),
                                    ],
                                    vec![text("Bottom")],
                                ),
                            ],
                        ),
                    ],
                ),
                element(
                    "div",
                    &[
                        ("className", "content-reservation"),
                        ("aria-label", "Native Chromium content surface"),
                    ],
                    vec![element(
                        "p",
                        &[("className", "muted")],
                        vec![text(
                            "Native CEF surface is not attached in the standalone controls preview.",
                        )],
                    )],
                ),
                inspector("", "right"),
                inspector("bottom-", "bottom"),
                crate::window_chrome::resize_regions(),
            ],
        ),
    }
}

/// Read only the shell's root widget; inspector sections cannot alter allocation.
pub fn inspector_dock(doc: &InteractiveDocument) -> crate::viewport::InspectorDock {
    use crate::viewport::InspectorDock;
    match doc
        .session
        .widgets
        .get("0")
        .map(|widget| widget.value.as_str())
    {
        Some("right") => InspectorDock::Right,
        Some("bottom") => InspectorDock::Bottom,
        _ => InspectorDock::Hidden,
    }
}

/// CSS metrics and native allocation share the same Rust constants.
pub fn management_styles() -> &'static str {
    static STYLES: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    STYLES.get_or_init(|| {
        use crate::viewport::*;
        r#"
body{margin:0;background:#101620;color:#e8edf7;font-family:system-ui;font-size:14px}
*{box-sizing:border-box} button,input{font:inherit} [hidden]{display:none!important}
.browser-shell{display:block;gap:0;width:100vw;height:100vh;position:relative;overflow:hidden}
.tab-strip{height:__TABS__px;display:flex;align-items:center;gap:6px;padding:0 8px;background:#0d121b}
.brand{font-size:20px;font-weight:700;color:#6cddca;width:20px;text-align:center}
.tab-workspace{display:flex;align-items:center;gap:6px;flex:0 1 auto;min-width:0}
.tab-list{display:flex;align-items:center;gap:2px;overflow:auto;flex:0 1 auto;min-width:0}
.tab-slot{position:relative;align-items:center;flex:0 0 232px;width:232px;height:32px}
.page-tab{display:flex;align-items:center;gap:8px;height:32px;width:100%;min-width:0;text-align:left;padding:0 38px 0 12px;border:0;border-radius:9px 9px 0 0;background:#202b3d;color:#f1f5fd;overflow:hidden;white-space:nowrap;text-overflow:ellipsis}
.page-tab span{overflow:hidden;white-space:nowrap;text-overflow:ellipsis}
.tab-icon{width:16px;min-width:16px;height:16px}.tab-icon svg{width:16px;height:16px}
.page-tab.compact{justify-content:center;padding:0}
.page-tab.compact span:not(.tab-icon){display:none}
.page-tab.compact[aria-selected="true"] .tab-icon{display:none}
.page-tab[aria-selected="false"]{background:#111b29;color:#9daec6}
.page-tab[aria-selected="false"]:hover{background:#263246;color:#f1f5fd}
.tab-caption{flex:1;min-width:0;color:#8d9ab0;font-size:12px;text-align:right;overflow:hidden;white-space:nowrap;text-overflow:ellipsis}
.window-controls{height:34px;display:flex;flex-shrink:0;gap:0;margin-right:-8px}
.window-button{width:46px;height:34px;display:flex;align-items:center;justify-content:center;padding:0;border:0;background:transparent;color:#d8e7fb}
.window-button:hover{background:#344259} #window-close:hover{background:#b6293b}
.nav-button{display:flex;align-items:center;justify-content:center} svg{pointer-events:none;flex-shrink:0}
.toolbar{height:__BAR__px;display:flex;align-items:center;gap:6px;padding:6px 10px;background:#202b3d;border-bottom:1px solid #344259}
.nav-button{width:30px;height:30px;padding:0;border:0;border-radius:5px;background:transparent;color:#8592a8;font-size:18px}
.nav-button.reload{width:30px}
.nav-button.tab-close{position:absolute;right:4px;top:2px;width:28px;height:28px;border-radius:14px;color:#c6d0df}
.tab-close svg{width:14px;height:14px}
.tab-close:hover,#tab-new:hover{background:#3a485e;color:#ffffff}
#tab-new{flex-shrink:0;border-radius:15px;color:#d9e2ef}
.page-tab:focus-visible,.tab-close:focus-visible,#tab-new:focus-visible{outline:2px solid #9bbcff;outline-offset:-2px}
.address{flex:1;min-width:120px;height:34px;padding:5px 12px;border:1px solid #43536e;border-radius:18px;background:#101925;color:#edf3ff}
.mode-badge{max-width:210px;overflow:hidden;text-overflow:ellipsis;font-size:10px;color:#9fbbc7;white-space:nowrap;padding:0 6px}
.dock-controls{display:flex;gap:2px;height:32px;padding:2px;background:#101925;border-radius:6px}
.dock-controls button{height:28px;padding:3px 8px;border:0;border-radius:4px;background:#202f45;color:#d8e7fb;font-size:12px}
.resize-hit{position:absolute;z-index:20}
.resize-n{top:0;left:__EDGE__px;right:__EDGE__px;height:__EDGE__px;cursor:ns-resize}
.resize-s{bottom:0;left:__EDGE__px;right:__EDGE__px;height:__EDGE__px;cursor:ns-resize}
.resize-w{left:0;top:__EDGE__px;bottom:__EDGE__px;width:__EDGE__px;cursor:ew-resize}
.resize-e{right:0;top:__EDGE__px;bottom:__EDGE__px;width:__EDGE__px;cursor:ew-resize}
.resize-nw{left:0;top:0;width:__EDGE__px;height:__EDGE__px;cursor:nwse-resize}
.resize-ne{right:0;top:0;width:__EDGE__px;height:__EDGE__px;cursor:nesw-resize}
.resize-sw{left:0;bottom:0;width:__EDGE__px;height:__EDGE__px;cursor:nesw-resize}
.resize-se{right:0;bottom:0;width:__EDGE__px;height:__EDGE__px;cursor:nwse-resize}
button:not(:disabled){cursor:pointer} button:disabled{cursor:not-allowed} input{cursor:text}
.content-reservation{position:absolute;left:0;right:0;top:__TOP__px;bottom:0;background:#ffffff;padding:20px;color:#58687f}
.inspector{position:absolute;background:#131d2c;border:1px solid #344259;padding:16px;overflow:auto;z-index:2}
.inspector-right{top:__TOP__px;right:__EDGE__px;bottom:__EDGE__px;width:32vw;max-width:__RIGHT__px}
.inspector-bottom{left:__EDGE__px;right:__EDGE__px;bottom:__EDGE__px;height:40vh;max-height:__BOTTOM__px}
.inspector-title{font-size:15px;font-weight:600;color:#e9f3ff}
.inspector h2{font-size:14px;margin-top:20px}.inspector p{line-height:1.5;font-size:13px}
.muted{color:#92a4bc;font-size:12px}.analysis-tabs{display:flex;gap:3px;margin-top:14px}
.analysis-tabs button{padding:7px 8px;background:#24354e;color:#e7f0ff;border:0;border-radius:4px;font-size:12px}
"#.replace("__TABS__", &TAB_STRIP_DIP.to_string())
            .replace("__BAR__", &TOOLBAR_DIP.to_string())
            .replace("__TOP__", &CHROME_TOP_DIP.to_string())
            .replace("__RIGHT__", &RIGHT_MAX_DIP.to_string())
            .replace("__BOTTOM__", &BOTTOM_MAX_DIP.to_string())
            .replace("__EDGE__", &crate::frame_geometry::RESIZE_DIP.to_string())
    })
}
/// Build the actual native document, with no parser or foreign application source.
pub fn document(width: u32, height: u32) -> Result<InteractiveDocument, String> {
    InteractiveDocument::new(
        component(),
        management_styles(),
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
        let dock = doc
            .renderer
            .document
            .get_element_by_id("dock-right")
            .unwrap();
        accessibility::act(
            &mut doc,
            &accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: accesskit::NodeId(dock.as_u64()),
                data: None,
            },
        )
        .unwrap();
        assert_eq!(inspector_dock(&doc), crate::viewport::InspectorDock::Right);
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
    fn dock_actions_change_allocation_state_without_navigation_or_authority() {
        use crate::viewport::InspectorDock;
        let mut doc = document(1280, 800).unwrap();
        assert_eq!(inspector_dock(&doc), InspectorDock::Hidden);
        for (id, expected) in [
            ("dock-right", InspectorDock::Right),
            ("dock-bottom", InspectorDock::Bottom),
            ("dock-hidden", InspectorDock::Hidden),
        ] {
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
            assert_eq!(inspector_dock(&doc), expected);
            assert!(doc.activation_requests.is_empty());
            assert!(doc.file_requests.is_empty());
            assert!(doc.callback_scope().is_none());
        }
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
