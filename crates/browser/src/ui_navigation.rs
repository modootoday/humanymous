//! Native chrome intents and receipt projection, separate from CEF page authority.
use meari::domain::{Node, NodeKind, Value};
use meari_native::infrastructure::document::InteractiveDocument;

/// User request; the native adapter must classify it before CEF task creation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
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
    let intents = doc
        .activation_requests
        .iter()
        .skip(after)
        .take(16)
        .filter_map(|request| {
            if Some(request) == go.as_ref() {
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
fn text(doc: &mut InteractiveDocument, id: &str, value: &str) {
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
    text(doc, "page-tab-label", route.title());
    text(doc, "navigation-status", "LOCAL ONLY");
    Ok(())
}
/// Present a bounded application notice; the caller supplies a fixed safe message.
pub fn notice(doc: &mut InteractiveDocument, message: &str) {
    if message.len() <= 160 {
        text(doc, "navigation-status", message);
    }
}

/// Pure composition guard; retained here as a compatible public re-export.
pub use crate::input::ImeGuard;

#[cfg(test)]
mod tests {
    use super::*;
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
