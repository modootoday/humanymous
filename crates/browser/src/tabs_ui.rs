//! Stable native tab controls; hidden identities are never reused for new children.
use crate::{
    tabs::{Command, Id, Tabs},
    ui::{element, navigation_button, text},
    ui_navigation,
};
use meari::domain::{Node, NodeKind, Value};
use meari_native::infrastructure::{
    document::InteractiveDocument,
    native_host::{HostStamp, NativeDocumentHandle},
};

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
/// Acknowledged model projection; viewport geometry belongs to retained CSS layout.
#[derive(Default)]
pub struct Projection {
    views: Vec<View>,
    commits: u64,
    source: Option<HostStamp>,
}
impl Projection {
    /// Commit actual model changes through the checked source owner only once.
    pub fn update(
        &mut self,
        document: &NativeDocumentHandle,
        views: &[View],
    ) -> Result<bool, String> {
        let stamp = document.stamp().map_err(|e| e.to_string())?;
        let same_source = self.source.as_ref().is_some_and(|old| {
            old.host_id == stamp.host_id
                && old.owner == stamp.owner
                && old.definition_revision == stamp.definition_revision
        });
        if self.views == views && same_source {
            return Ok(false);
        }
        let next = self
            .commits
            .checked_add(1)
            .ok_or("tab projection counter exhausted")?;
        document
            .try_reproject(&stamp, |doc| project(doc, views))
            .map_err(|e| e.to_string())?;
        let committed = document.stamp().map_err(|e| e.to_string())?;
        self.views = views.to_vec();
        self.commits = next;
        self.source = Some(committed);
        Ok(true)
    }
    /// Successful model-source commits; viewport-only layout is not counted.
    pub fn commits(&self) -> u64 {
        self.commits
    }
}

#[cfg(test)]
#[path = "tabs_ui_resize_tests.rs"]
mod resize_tests;
pub(crate) fn responsive_styles() -> String {
    use std::fmt::Write;
    let mut css = String::new();
    for count in 1..=Tabs::CAPACITY {
        let reserve = 228 + 2 * (count - 1);
        let close = reserve + 96 * count - 1;
        let title = reserve + 84 * count - 1;
        write!(css, "@media(max-width:{close}px){{.tabs-count-{count} .tab-slot:not(.selected) .tab-close:not(:focus){{display:none}}}}").expect("String formatting");
        write!(css, "@media(max-width:{title}px){{.tabs-count-{count} .page-tab{{justify-content:center;padding:0}}.tabs-count-{count} .tab-title{{display:none}}.tabs-count-{count} .selected .tab-icon,.tabs-count-{count} .tab-slot:focus-within .tab-icon{{display:none}}}}").expect("String formatting");
    }
    css
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
                        "className",
                        if id == 0 {
                            "tab-slot selected"
                        } else {
                            "tab-slot"
                        },
                    ),
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
                                &[("id", &label), ("className", "tab-title")],
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
            ("id", "browser-tabs"),
            ("aria-label", "Browser tabs"),
            ("className", "tab-list tabs-count-1"),
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
    if views.len() > Tabs::CAPACITY
        || views.iter().any(|v| v.id >= Tabs::CREATIONS)
        || views.iter().filter(|v| v.selected).count() != usize::from(!views.is_empty())
        || views
            .iter()
            .enumerate()
            .any(|(i, v)| views[..i].iter().any(|old| old.id == v.id))
    {
        return Err("invalid bounded tab projection".into());
    }
    ui_navigation::attribute(
        doc,
        "browser-tabs",
        "className",
        Some(&format!("tab-list tabs-count-{}", views.len())),
    );
    for id in 0..Tabs::CREATIONS {
        let view = views.iter().find(|view| view.id == id);
        ui_navigation::attribute(
            doc,
            &format!("tab-slot-{id}"),
            "className",
            Some(if view.is_some_and(|v| v.selected) {
                "tab-slot selected"
            } else {
                "tab-slot"
            }),
        );
        ui_navigation::attribute(
            doc,
            &format!("tab-slot-{id}"),
            "style",
            if view.is_some() {
                None
            } else {
                Some("display:none")
            },
        );
        let select = format!("tab-select-{id}");
        ui_navigation::attribute(
            doc,
            &format!("tab-close-{id}"),
            "style",
            if view.is_some() {
                None
            } else {
                Some("display:none")
            },
        );
        ui_navigation::attribute(doc, &select, "className", Some("page-tab"));
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
