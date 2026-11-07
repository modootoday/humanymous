//! Commit shell changes through Meari's retained source owner, including AX metadata.
use meari::domain::{Attribute, NodeKind, Value};
use meari_native::infrastructure::document::InteractiveDocument;

pub(crate) fn commit(doc: &mut InteractiveDocument) -> Result<(), String> {
    let mut source = doc.source.clone();
    source.states = crate::ui::native_states();
    if let NodeKind::Element { attributes, .. } = &mut source.root.kind {
        attributes.retain(|attribute| attribute.name != "data-browser-ready");
        attributes.push(Attribute {
            name: "data-browser-ready".into(),
            value: Value::State("browserReady".into()),
        });
    }
    let next = meari_native::domain::widgets::Session::from_tree(&source.root)?;
    let allowed: std::collections::BTreeSet<_> = doc
        .elements()
        .iter()
        .filter(|element| {
            matches!(
                element.id.as_str(),
                "nav-back" | "nav-forward" | "nav-reload" | "nav-stop"
            )
        })
        .map(|element| element.path.clone())
        .collect();
    let mut changed_paths: std::collections::BTreeSet<_> = doc
        .session
        .disabled_paths
        .symmetric_difference(&next.disabled_paths)
        .cloned()
        .collect();
    let markup = meari_native::application::markup::build(&doc.source, &next)?;
    let disabled = |element: &meari_native::application::markup::Element| {
        element.attributes.contains_key("disabled")
            || element
                .attributes
                .get("aria-disabled")
                .is_some_and(|value| value == "true")
    };
    for before in doc.elements() {
        let after = markup
            .elements
            .iter()
            .find(|after| after.path == before.path)
            .ok_or("shell action identity changed")?;
        if disabled(before) != disabled(after) {
            changed_paths.insert(before.path.clone());
        }
    }
    for changed in &changed_paths {
        if !allowed.contains(changed)
            || doc.session.widgets.values().any(|widget| {
                widget.root == *changed
                    || widget.trigger.as_ref() == Some(changed)
                    || widget.close_paths.contains(changed)
                    || widget.items.iter().any(|item| item.path == *changed)
            })
        {
            return Err("shell disabled migration exceeds standalone navigation controls".into());
        }
    }
    // The pinned reload contract requires explicit disabled-path migration. Only
    // these four standalone host buttons may change; composite widget state stays owned.
    let previous = std::mem::replace(&mut doc.session.disabled_paths, next.disabled_paths);
    let result = doc
        .prepare_source_replacement(source)
        .and_then(|prepared| doc.commit_source_replacement(prepared));
    if result.is_err() {
        doc.session.disabled_paths = previous;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unrelated_widget_disabled_changes_cannot_use_navigation_migration() {
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
        let before = doc.session.disabled_paths.clone();
        crate::ui_navigation::attribute(&mut doc, "dock-right", "disabled", Some(""));
        assert!(commit(&mut doc).is_err());
        assert_eq!(doc.session.disabled_paths, before);
    }
}
