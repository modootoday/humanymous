//! Native chrome intents and receipt projection, separate from CEF page authority.
use meari::domain::{Node, NodeKind, Value};
use meari_native::infrastructure::document::InteractiveDocument;

/// User request; the native adapter must classify it before CEF task creation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Window-level tab or address command, guarded by the native host.
    Tab(crate::tabs::Command),
    /// Traverse the actual engine history to its previous entry.
    Back,
    /// Traverse the actual engine history to its next entry.
    Forward,
    /// Cancel loading and pending verification.
    Stop,
    /// Open the explicitly edited address.
    Open(String),
    /// Return the current real tab to the owner landing page.
    Home,
    /// Reload the current committed typed route.
    Reload,
}

fn path(doc: &InteractiveDocument, id: &str) -> Option<String> {
    let node = doc.renderer.document.get_element_by_id(id)?;
    doc.renderer
        .document
        .get_node(node)?
        .attr("data-bridge-path".into())
        .map(str::to_owned)
}
/// Current edited address; no synthetic navigation side effect.
pub fn address(doc: &InteractiveDocument) -> Option<String> {
    let path = path(doc, "address")?;
    if let Some(value) = doc.input_values.get(&path) {
        return Some(value.clone());
    }
    let id = doc.renderer.document.get_element_by_id("address")?;
    doc.renderer
        .document
        .get_node(id)?
        .attr("value".into())
        .map(str::to_owned)
}
/// Whether the actual native address input owns focus.
pub fn address_focused(doc: &InteractiveDocument) -> bool {
    doc.renderer
        .document
        .get_element_by_id("address")
        .is_some_and(|id| doc.renderer.document.get_focussed_node_id() == Some(id))
}
/// Read only exact local control requests. No target page path can issue them.
pub fn intents_since(doc: &InteractiveDocument, after: usize) -> (usize, Vec<Intent>) {
    let go = path(doc, "nav-go");
    let home = path(doc, "nav-home");
    let reload = path(doc, "nav-reload");
    let back = path(doc, "nav-back");
    let forward = path(doc, "nav-forward");
    let stop = path(doc, "nav-stop");
    let tabs = crate::tabs_ui::commands(doc);
    let intents = doc
        .activation_requests
        .iter()
        .skip(after)
        .take(16)
        .filter_map(|request| {
            if let Some((_, command)) = tabs.iter().find(|(path, _)| path == request) {
                Some(Intent::Tab(*command))
            } else if Some(request) == back.as_ref() {
                Some(Intent::Back)
            } else if Some(request) == forward.as_ref() {
                Some(Intent::Forward)
            } else if Some(request) == stop.as_ref() {
                Some(Intent::Stop)
            } else if Some(request) == go.as_ref() {
                address(doc).map(Intent::Open)
            } else if Some(request) == home.as_ref() {
                Some(Intent::Home)
            } else if Some(request) == reload.as_ref() {
                Some(Intent::Reload)
            } else {
                None
            }
        })
        .collect();
    (doc.activation_requests.len(), intents)
}
pub(crate) fn text(doc: &mut InteractiveDocument, id: &str, value: &str) {
    fn source(node: &mut Node, id: &str, value: &str) {
        if let NodeKind::Element {
            attributes,
            children,
            ..
        } = &mut node.kind
        {
            if attributes
                .iter()
                .any(|a| a.name == "id" && a.value == Value::String(id.into()))
            {
                if let Some(Node {
                    kind: NodeKind::Text(current),
                    ..
                }) = children.first_mut()
                {
                    *current = Value::String(value.into());
                }
            } else {
                for child in children {
                    source(child, id, value);
                }
            }
        }
    }
    source(&mut doc.source.root, id, value);
    if let Some(node) = doc.renderer.document.get_element_by_id(id)
        && let Some(child) = doc
            .renderer
            .document
            .get_node(node)
            .and_then(|n| n.children.first().copied())
    {
        doc.renderer.document.mutate().set_node_text(child, value);
    }
}
/// Apply an actual main-frame commit receipt, not a merely accepted request.
pub fn committed(
    doc: &mut InteractiveDocument,
    route: crate::internal_urls::Route,
) -> Result<(), String> {
    let address = path(doc, "address").ok_or("native address missing")?;
    doc.set_input_value(&address, route.url())?;
    Ok(())
}
/// Present a bounded application notice; the caller supplies a fixed safe message.
pub fn notice(doc: &mut InteractiveDocument, message: &str) {
    if message.len() <= 160 {
        text(doc, "navigation-status", message);
    }
}

/// Project engine availability to real disabled attributes and accessibility state.
pub fn controls(doc: &mut InteractiveDocument, state: crate::history::Controls) {
    fn source(node: &mut Node, id: &str, enabled: bool) {
        if let NodeKind::Element {
            attributes,
            children,
            ..
        } = &mut node.kind
        {
            if attributes
                .iter()
                .any(|a| a.name == "id" && a.value == Value::String(id.into()))
            {
                attributes.retain(|a| a.name != "disabled" && a.name != "aria-disabled");
                attributes.push(meari::domain::Attribute {
                    name: "aria-disabled".into(),
                    value: Value::String((!enabled).to_string()),
                });
                if !enabled {
                    attributes.push(meari::domain::Attribute {
                        name: "disabled".into(),
                        value: Value::Bool(true),
                    });
                }
            } else {
                for child in children {
                    source(child, id, enabled);
                }
            }
        }
    }
    for (id, enabled) in [
        ("nav-back", state.back),
        ("nav-forward", state.forward),
        ("nav-stop", state.loading),
        ("nav-reload", !state.loading),
    ] {
        source(&mut doc.source.root, id, enabled);
        if let Some(node) = doc.renderer.document.get_element_by_id(id) {
            let mut mutation = doc.renderer.document.mutate();
            let name = html5ever::QualName::new(None, "".into(), "disabled".into());
            if enabled {
                mutation.clear_attribute(node, name);
            } else {
                mutation.set_attribute(node, name, "");
            }
            mutation.set_attribute(
                node,
                html5ever::QualName::new(None, "".into(), "aria-disabled".into()),
                if enabled { "false" } else { "true" },
            );
        }
    }
}

/// Mutate one fixed shell attribute in both source and rendered scene.
pub(crate) fn attribute(doc: &mut InteractiveDocument, id: &str, name: &str, value: Option<&str>) {
    fn source(node: &mut Node, id: &str, name: &str, value: Option<&str>) {
        if let NodeKind::Element {
            attributes,
            children,
            ..
        } = &mut node.kind
        {
            if attributes
                .iter()
                .any(|a| a.name == "id" && a.value == Value::String(id.into()))
            {
                attributes.retain(|a| a.name != name);
                if let Some(value) = value {
                    attributes.push(meari::domain::Attribute {
                        name: name.into(),
                        value: Value::String(value.into()),
                    });
                }
            } else {
                for child in children {
                    source(child, id, name, value);
                }
            }
        }
    }
    source(&mut doc.source.root, id, name, value);
    if let Some(node) = doc.renderer.document.get_element_by_id(id) {
        let mut mutation = doc.renderer.document.mutate();
        let name = html5ever::QualName::new(None, "".into(), name.into());
        if let Some(value) = value {
            mutation.set_attribute(node, name, value);
        } else {
            mutation.clear_attribute(node, name);
        }
    }
}

/// Pure composition guard; retained here as a compatible public re-export.
pub use crate::input::ImeGuard;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_controls_project_disabled_state_and_native_actions() {
        let mut doc = InteractiveDocument::new(
            crate::ui::component_with_window_controls(true),
            crate::ui::management_styles(),
            meari_native::domain::Viewport {
                width: 1100,
                height: 720,
                dark: true,
            },
        )
        .unwrap();
        controls(
            &mut doc,
            crate::history::Controls {
                back: true,
                forward: false,
                loading: false,
            },
        );
        for (id, disabled) in [
            ("nav-back", false),
            ("nav-forward", true),
            ("nav-stop", true),
            ("nav-reload", false),
        ] {
            let node = doc.renderer.document.get_element_by_id(id).unwrap();
            assert_eq!(
                doc.renderer
                    .document
                    .get_node(node)
                    .unwrap()
                    .attr("disabled".into())
                    .is_some(),
                disabled
            );
        }
        let back = doc.renderer.document.get_element_by_id("nav-back").unwrap();
        meari_native::infrastructure::accessibility::act(
            &mut doc,
            &accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: accesskit::NodeId(back.as_u64()),
                data: None,
            },
        )
        .unwrap();
        let (cursor, intents) = intents_since(&doc, 0);
        assert_eq!(intents, vec![Intent::Back]);
        controls(
            &mut doc,
            crate::history::Controls {
                loading: true,
                ..Default::default()
            },
        );
        let stop = doc.renderer.document.get_element_by_id("nav-stop").unwrap();
        meari_native::infrastructure::accessibility::act(
            &mut doc,
            &accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: accesskit::NodeId(stop.as_u64()),
                data: None,
            },
        )
        .unwrap();
        assert_eq!(intents_since(&doc, cursor).1, vec![Intent::Stop]);
    }
    #[test]
    fn go_and_commit_are_distinct_real_document_operations() {
        let mut doc = InteractiveDocument::new(
            crate::ui::component_with_window_controls(true),
            crate::ui::management_styles(),
            meari_native::domain::Viewport {
                width: 1100,
                height: 720,
                dark: true,
            },
        )
        .unwrap();
        let go = doc.renderer.document.get_element_by_id("nav-go").unwrap();
        meari_native::infrastructure::accessibility::act(
            &mut doc,
            &accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: accesskit::NodeId(go.as_u64()),
                data: None,
            },
        )
        .unwrap();
        let (cursor, intents) = intents_since(&doc, 0);
        assert_eq!(intents.len(), 1);
        assert!(matches!(intents[0], Intent::Open(_)));
        assert!(intents_since(&doc, cursor).1.is_empty());
        committed(
            &mut doc,
            crate::internal_urls::Route::Internal(crate::internal_urls::InternalPage::Gpu),
        )
        .unwrap();
        assert_eq!(address(&doc).as_deref(), Some("chrome://gpu/"));
        assert!(doc.file_requests.is_empty());
        assert!(doc.callback_scope().is_none());
    }
}
