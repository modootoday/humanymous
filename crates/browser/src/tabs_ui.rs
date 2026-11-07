//! Stable native tab controls; hidden identities are never reused for new children.
use crate::{
    tabs::{Command, Id, Tabs},
    ui::{element, navigation_button, text},
    ui_navigation,
};
use meari::domain::{Node, NodeKind, Value};
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
fn widths(viewport: u32, views: &[View]) -> Vec<u32> {
    if views.is_empty() {
        return Vec::new();
    }
    // Reserve brand, new-tab, gaps and fixed caption controls before shrinking tabs.
    let available = viewport
        .saturating_sub(228)
        .saturating_sub(2 * (views.len() as u32 - 1));
    let equal = (available / views.len() as u32).min(232);
    if equal >= 52 {
        return vec![equal; views.len()];
    }
    let inactive = available.saturating_sub(52) / (views.len().saturating_sub(1).max(1) as u32);
    views
        .iter()
        .map(|v| {
            if v.selected {
                52
            } else {
                inactive.clamp(32, 52)
            }
        })
        .collect()
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
                    ("className", "tab-slot"),
                    (
                        "style",
                        if id == 0 {
                            "display:flex"
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
                        vec![
                            element(
                                "span",
                                &[("className", "tab-icon"), ("aria-hidden", "true")],
                                vec![crate::icons::node(crate::icons::Icon::Page)],
                            ),
                            element(
                                "span",
                                &[("id", &label)],
                                vec![text(if enabled { "New tab" } else { "No target" })],
                            ),
                        ],
                    ),
                    {
                        let mut button = navigation_button(
                            &close,
                            "Close tab",
                            crate::icons::Icon::Close,
                            enabled,
                        );
                        if let NodeKind::Element { attributes, .. } = &mut button.kind
                            && let Some(class) =
                                attributes.iter_mut().find(|a| a.name == "className")
                        {
                            class.value = Value::String("nav-button tab-close".into());
                        }
                        button
                    },
                ],
            )
        })
        .collect();
    element(
        "div",
        &[
            ("role", "tablist"),
            ("aria-label", "Browser tabs"),
            ("className", "tab-list"),
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
    let widths = widths(doc.viewport.width, views);
    for id in 0..Tabs::CREATIONS {
        let view = views.iter().find(|view| view.id == id);
        let width = views
            .iter()
            .position(|v| v.id == id)
            .map(|index| widths[index]);
        let style = width.map(|width| format!("display:flex;width:{width}px;flex:0 0 {width}px"));
        ui_navigation::attribute(
            doc,
            &format!("tab-slot-{id}"),
            "style",
            Some(style.as_deref().unwrap_or("display:none")),
        );
        let select = format!("tab-select-{id}");
        let show_close = view.is_some_and(|v| v.selected) || width.is_some_and(|w| w >= 96);
        ui_navigation::attribute(
            doc,
            &format!("tab-close-{id}"),
            "style",
            Some(if show_close {
                "display:flex"
            } else {
                "display:none"
            }),
        );
        ui_navigation::attribute(
            doc,
            &select,
            "className",
            Some(if width.is_some_and(|w| w < 84) {
                "page-tab compact"
            } else {
                "page-tab"
            }),
        );
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
    fn bounds(doc: &InteractiveDocument, id: &str) -> (f32, f32, f32, f32) {
        let id = doc.renderer.document.get_element_by_id(id).unwrap();
        let node = doc.renderer.document.get_node(id).unwrap();
        let position = node.absolute_position(0.0, 0.0);
        let size = node.final_layout().size;
        (position.x, position.y, size.width, size.height)
    }
    #[test]
    fn close_hit_area_is_inside_tab_and_new_tab_precedes_caption_controls() {
        for (width, count) in [(1100, 1), (1100, 2), (640, 8)] {
            let mut doc = InteractiveDocument::new(
                crate::ui::component_with_window_controls(true),
                crate::ui::management_styles(),
                meari_native::domain::Viewport {
                    width,
                    height: 720,
                    dark: true,
                },
            )
            .unwrap();
            let views: Vec<_> = (0..count)
                .map(|id| View {
                    id,
                    title: "New tab",
                    selected: id == 0,
                })
                .collect();
            project(&mut doc, &views).unwrap();
            let (tx, ty, tw, th) = bounds(&doc, "tab-select-0");
            let (cx, cy, cw, ch) = bounds(&doc, "tab-close-0");
            assert!(cw >= 28.0 && ch >= 28.0, "close hit target: {cw}x{ch}");
            assert!(cx >= tx && cy >= ty && cx + cw <= tx + tw && cy + ch <= ty + th);
            let (nx, _, nw, _) = bounds(&doc, "tab-new");
            let (wx, _, _, _) = bounds(&doc, "window-minimize");
            assert!(nx + nw <= wx && nx >= 0.0 && nw >= 28.0);
            if count <= 2 {
                let (last_x, _, last_w, _) = bounds(&doc, &format!("tab-select-{}", count - 1));
                assert!((0.0..=10.0).contains(&(nx - last_x - last_w)));
            }
            if count == 8 {
                assert!(tw < 100.0, "crowded tabs must shrink before scrolling");
                let (_, _, hidden_width, _) = bounds(&doc, "tab-close-1");
                assert_eq!(hidden_width, 0.0, "narrow inactive close must be hidden");
            }
        }
    }
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
