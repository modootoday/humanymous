//! Stable native tab controls; hidden identities are never reused for new children.
use crate::{
    tabs::{Command, Id, Tabs},
    ui::{element, navigation_button, text},
    ui_navigation,
};
use meari::domain::Node;
use meari_native::infrastructure::document::InteractiveDocument;

/// Source-bound display projection of one actual native tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    /// Session-local identity, never reused.
    pub id: Id,
    /// Fixed local route title or creation notice.
    pub title: &'static str,
    /// True only for the active child.
    pub selected: bool,
}
/// Stable slots avoid rebuilding the shell or losing native input state on switch.
pub fn strip(enabled: bool) -> Node {
    let children = (0..Tabs::CREATIONS)
        .map(|id| {
            let select = format!("tab-select-{id}");
            let label = format!("tab-label-{id}");
            let close = format!("tab-close-{id}");
            let container = format!("tab-slot-{id}");
            element(
                "div",
                &[
                    ("id", &container),
                    (
                        "style",
                        if id == 0 {
                            "display:flex;flex:0 0 auto"
                        } else {
                            "display:none"
                        },
                    ),
                ],
                vec![
                    element(
                        "button",
                        &[
                            ("id", &select),
                            ("role", "tab"),
                            ("aria-label", "New tab"),
                            ("aria-selected", if id == 0 { "true" } else { "false" }),
                            ("className", "page-tab"),
                        ],
                        vec![element(
                            "span",
                            &[("id", &label)],
                            vec![text(if enabled { "New tab" } else { "No target" })],
                        )],
                    ),
                    navigation_button(&close, "Close tab", crate::icons::Icon::Close, enabled),
                ],
            )
        })
        .collect();
    element(
        "div",
        &[
            ("role", "tablist"),
            ("aria-label", "Browser tabs"),
            ("style", "display:flex;overflow:auto;flex:1;min-width:0"),
        ],
        children,
    )
}
/// Only exact rendered shell bridge paths can produce tab commands.
pub fn commands(doc: &InteractiveDocument) -> Vec<(String, Command)> {
    let mut commands = Vec::new();
    let mut add = |id: &str, command| {
        if let Some(node) = doc
            .renderer
            .document
            .get_element_by_id(id)
            .and_then(|id| doc.renderer.document.get_node(id))
            && let Some(path) = node.attr("data-bridge-path".into())
        {
            commands.push((path.to_owned(), command));
        }
    };
    add("tab-new", Command::New);
    for id in 0..Tabs::CREATIONS {
        add(&format!("tab-select-{id}"), Command::Select(id));
        add(&format!("tab-close-{id}"), Command::CloseId(id));
    }
    commands
}
/// Apply selected/name/hidden state without inventing a browser page or history.
pub fn project(doc: &mut InteractiveDocument, views: &[View]) -> Result<(), String> {
    for id in 0..Tabs::CREATIONS {
        let view = views.iter().find(|view| view.id == id);
        ui_navigation::attribute(
            doc,
            &format!("tab-slot-{id}"),
            "style",
            Some(if view.is_some() {
                "display:flex;flex:0 0 auto"
            } else {
                "display:none"
            }),
        );
        let select = format!("tab-select-{id}");
        ui_navigation::attribute(
            doc,
            &select,
            "aria-selected",
            Some(if view.is_some_and(|v| v.selected) {
                "true"
            } else {
                "false"
            }),
        );
        ui_navigation::attribute(
            doc,
            &select,
            "tabindex",
            Some(if view.is_some_and(|v| v.selected) {
                "0"
            } else {
                "-1"
            }),
        );
        if let Some(view) = view {
            ui_navigation::text(doc, &format!("tab-label-{id}"), view.title);
            ui_navigation::attribute(doc, &select, "aria-label", Some(view.title));
            ui_navigation::attribute(
                doc,
                &format!("tab-close-{id}"),
                "aria-label",
                Some(&format!("Close {}", view.title)),
            );
        }
    }
    crate::scene_projection::commit(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_tab_names_and_close_actions_reach_native_document() {
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
        project(
            &mut doc,
            &[
                View {
                    id: 0,
                    title: "Version",
                    selected: false,
                },
                View {
                    id: 1,
                    title: "New tab",
                    selected: true,
                },
            ],
        )
        .unwrap();
        for (id, selected) in [(0, "false"), (1, "true")] {
            let node = doc
                .renderer
                .document
                .get_element_by_id(&format!("tab-select-{id}"))
                .unwrap();
            assert_eq!(
                doc.renderer
                    .document
                    .get_node(node)
                    .unwrap()
                    .attr("aria-selected".into()),
                Some(selected)
            );
        }
        let hidden = doc
            .renderer
            .document
            .get_element_by_id("tab-slot-2")
            .unwrap();
        assert_eq!(
            doc.renderer
                .document
                .get_node(hidden)
                .unwrap()
                .attr("style".into()),
            Some("display:none")
        );
        for (id, expected) in [
            ("tab-new", Command::New),
            ("tab-select-0", Command::Select(0)),
            ("tab-close-1", Command::CloseId(1)),
        ] {
            let cursor = doc.activation_requests.len();
            let node = doc.renderer.document.get_element_by_id(id).unwrap();
            meari_native::infrastructure::accessibility::act(
                &mut doc,
                &accesskit::ActionRequest {
                    action: accesskit::Action::Click,
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: accesskit::NodeId(node.as_u64()),
                    data: None,
                },
            )
            .unwrap();
            assert_eq!(
                ui_navigation::intents_since(&doc, cursor).1,
                vec![ui_navigation::Intent::Tab(expected)]
            );
        }
    }
}
