//! Pinned native document oracles; these are not OS display or latency measurements.
use super::*;
use meari::domain::host::{HostCapabilities, HostEnvironment, HostTarget, Support, VisualViewport};
use meari_native::application::Renderer;
use meari_native::domain::Viewport;

fn environment(width: u32, dpi: u32) -> HostEnvironment {
    HostEnvironment {
        target: HostTarget::Native,
        layout: Viewport {
            width,
            height: 720,
            dark: true,
        },
        visual: VisualViewport {
            width: f64::from(width),
            height: 720.0,
            offset_x: 0.0,
            offset_y: 0.0,
            scale: 1.0,
        },
        device_scale: f64::from(dpi) / 96.0,
        layout_zoom: Some(1.0),
        safe_area: None,
        capabilities: HostCapabilities {
            client_resize: Support::Supported,
            preview_resize: Support::Unsupported,
            reduced_motion: Support::Unknown,
            contrast: Support::Unknown,
            pointer_hover: Support::Unknown,
            accessibility: Support::Unknown,
        },
    }
}
fn handle() -> NativeDocumentHandle {
    NativeDocumentHandle::new(
        InteractiveDocument::new(
            crate::ui::component_with_window_controls(true),
            crate::ui::management_styles(),
            Viewport {
                width: 1100,
                height: 720,
                dark: true,
            },
        )
        .unwrap(),
        1,
    )
    .unwrap()
}
fn views(count: usize) -> Vec<View> {
    (0..count)
        .map(|id| View {
            id,
            title: "New tab",
            selected: id == 0,
        })
        .collect()
}
fn resize(handle: &NativeDocumentHandle, width: u32, dpi: u32) {
    let stamp = handle.stamp().unwrap();
    handle
        .try_reproject(&stamp, |doc| {
            doc.commit_host_environment(environment(width, dpi))
        })
        .unwrap();
    assert!(
        handle
            .try_reproject(&stamp, |_| Ok::<_, String>(()))
            .is_err(),
        "viewport work must invalidate stale host updates"
    );
}
fn click(doc: &mut InteractiveDocument, id: &str) -> Result<(), String> {
    let node = doc.renderer.document.get_element_by_id(id).unwrap();
    meari_native::infrastructure::accessibility::act(
        doc,
        &accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: accesskit::NodeId(node.as_u64()),
            data: None,
        },
    )
}

#[test]
fn viewport_only_work_keeps_prepared_source_current_and_preserves_dirty_focus() {
    for count in [1, 2, 8] {
        for dpi in [96, 144, 192] {
            let handle = handle();
            let views = views(count);
            let mut projection = Projection::default();
            assert!(projection.update(&handle, &views).unwrap());
            let stamp = handle.stamp().unwrap();
            let (address, _) = handle
                .try_reproject(&stamp, |doc| {
                    let path = doc
                        .elements()
                        .iter()
                        .find(|e| e.id == "address")
                        .unwrap()
                        .path
                        .clone();
                    doc.set_input_value(&path, "unsubmitted local draft")?;
                    doc.focus_path(&path)?;
                    Ok::<_, String>(doc.renderer.document.get_element_by_id("address").unwrap())
                })
                .unwrap();
            let prepared = handle
                .try_with(|doc| doc.prepare_source_replacement(doc.source.clone()))
                .unwrap()
                .unwrap();
            for width in [1100, 1008, 900, 640, 914, 1100] {
                resize(&handle, width, dpi);
                assert!(!projection.update(&handle, &views).unwrap());
                assert_eq!(
                    projection.commits(),
                    1,
                    "viewport-only work recompiled tab source"
                );
                handle
                    .try_with(|doc| {
                        assert_eq!(
                            doc.renderer.document.get_element_by_id("address"),
                            Some(address)
                        );
                        assert_eq!(
                            ui_navigation::address(doc).as_deref(),
                            Some("unsubmitted local draft")
                        );
                        assert!(ui_navigation::address_focused(doc));
                        let new = doc
                            .renderer
                            .document
                            .get_node(doc.renderer.document.get_element_by_id("tab-new").unwrap())
                            .unwrap();
                        let caption = doc
                            .renderer
                            .document
                            .get_node(
                                doc.renderer
                                    .document
                                    .get_element_by_id("window-minimize")
                                    .unwrap(),
                            )
                            .unwrap();
                        assert!(
                            new.absolute_position(0.0, 0.0).x + new.final_layout().size.width
                                <= caption.absolute_position(0.0, 0.0).x
                        );
                    })
                    .unwrap();
            }
            // A hidden source replacement would invalidate this captured preparation.
            let stamp = handle.stamp().unwrap();
            handle
                .try_reproject(&stamp, |doc| doc.commit_source_replacement(prepared))
                .unwrap();
            // A genuine external source change invalidates the projection cache.
            assert!(projection.update(&handle, &views).unwrap());
        }
    }
}

#[test]
fn responsive_close_actions_and_real_model_changes_keep_native_metadata_current() {
    let handle = handle();
    let mut views = views(8);
    let mut projection = Projection::default();
    projection.update(&handle, &views).unwrap();
    for (width, visible) in [(1100, true), (1008, false), (640, false), (1100, true)] {
        resize(&handle, width, 96);
        assert!(!projection.update(&handle, &views).unwrap());
        let stamp = handle.stamp().unwrap();
        handle
            .try_reproject(&stamp, |doc| {
                assert_eq!(
                    click(doc, "tab-close-1").is_ok(),
                    visible,
                    "CSS and AX action visibility disagree"
                );
                let cursor = doc.activation_requests.len();
                click(doc, "tab-select-1")?;
                assert_eq!(
                    ui_navigation::intents_since(doc, cursor).1,
                    vec![ui_navigation::Intent::Tab(Command::Select(1))]
                );
                Ok::<_, String>(())
            })
            .unwrap();
    }
    let captured = handle
        .try_with(|doc| doc.prepare_source_replacement(doc.source.clone()))
        .unwrap()
        .unwrap();
    views[0].selected = false;
    views[1].selected = true;
    views[1].title = "Version";
    assert!(projection.update(&handle, &views).unwrap());
    let stamp = handle.stamp().unwrap();
    assert!(
        handle
            .try_reproject(&stamp, |doc| doc.commit_source_replacement(captured))
            .is_err(),
        "real model changes must invalidate captured source work"
    );
    resize(&handle, 640, 96);
    let stamp = handle.stamp().unwrap();
    handle
        .try_reproject(&stamp, |doc| {
            assert!(click(doc, "tab-close-0").is_err());
            click(doc, "tab-close-1")?;
            Ok::<_, String>(())
        })
        .unwrap();
    views.pop();
    assert!(projection.update(&handle, &views).unwrap());
    assert_eq!(projection.commits(), 3);
    let invalid = self::views(Tabs::CREATIONS);
    assert!(projection.update(&handle, &invalid).is_err());
    assert_eq!(
        projection.commits(),
        3,
        "failed projection must not acknowledge its cache"
    );
}

#[test]
fn focused_inactive_close_stays_visible_until_focus_moves_to_another_control() {
    let handle = handle();
    let views = views(8);
    let mut projection = Projection::default();
    projection.update(&handle, &views).unwrap();
    let stamp = handle.stamp().unwrap();
    let (focused, _) = handle
        .try_reproject(&stamp, |doc| {
            let path = doc
                .elements()
                .iter()
                .find(|e| e.id == "tab-close-2")
                .unwrap()
                .path
                .clone();
            doc.focus_path(&path)?;
            Ok::<_, String>(
                doc.renderer
                    .document
                    .get_element_by_id("tab-close-2")
                    .unwrap(),
            )
        })
        .unwrap();
    resize(&handle, 640, 144);
    assert!(!projection.update(&handle, &views).unwrap());
    handle
        .try_with(|doc| {
            assert_eq!(doc.renderer.document.get_focussed_node_id(), Some(focused));
            let close = doc.renderer.document.get_node(focused).unwrap();
            assert!(
                close.final_layout().size.width >= 28.0,
                "focused close must not disappear"
            );
        })
        .unwrap();
    let stamp = handle.stamp().unwrap();
    handle
        .try_reproject(&stamp, |doc| {
            let path = doc
                .elements()
                .iter()
                .find(|e| e.id == "address")
                .unwrap()
                .path
                .clone();
            doc.focus_path(&path)
        })
        .unwrap();
    let stamp = handle.stamp().unwrap();
    handle
        .try_reproject(&stamp, |doc| {
            let viewport = doc.viewport.clone();
            assert_eq!(viewport.width, 640);
            // Resolve focus styling at unchanged geometry, as a document redraw does.
            doc.renderer.layout(&viewport, 0.0)?;
            assert!(ui_navigation::address_focused(doc));
            assert!(
                click(doc, "tab-close-2").is_err(),
                "unfocused narrow close must hide again"
            );
            Ok::<_, String>(())
        })
        .unwrap();
    assert!(!projection.update(&handle, &views).unwrap());
    assert_eq!(projection.commits(), 1);
}
