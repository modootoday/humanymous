//! Shared native-shell allocation; page rendering belongs to CEF, never Meari.
/// Default-scale target tab-strip height in device-independent pixels.
pub const TAB_STRIP_DIP: f64 = 34.0;
/// Default-scale navigation/address toolbar height.
pub const TOOLBAR_DIP: f64 = 50.0;
/// Total default-scale header reservation shared with native controls.
pub const CHROME_TOP_DIP: f64 = TAB_STRIP_DIP + TOOLBAR_DIP;
/// Maximum width of a right-docked inspector.
pub const RIGHT_MAX_DIP: f64 = 340.0;
/// Maximum height of a bottom-docked inspector.
pub const BOTTOM_MAX_DIP: f64 = 260.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Native analysis-panel allocation, independent of the page's engine state.
pub enum InspectorDock {
    #[default]
    /// Page occupies the full area below the header.
    Hidden,
    /// Reserve an inspector region at the right edge.
    Right,
    /// Reserve an inspector region along the bottom edge.
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Physical-pixel child allocation inside its native parent's client rectangle.
pub struct ContentBounds {
    /// Left edge relative to the parent client area.
    pub x: i32,
    /// Top edge below the native browser controls.
    pub y: i32,
    /// Positive page width after inspector reservation.
    pub width: i32,
    /// Positive page height after chrome and inspector reservations.
    pub height: i32,
}

/// Allocate physical pixels from the actual client rectangle and DPI.
pub fn content_bounds(
    width: i32,
    height: i32,
    dpi: u32,
    dock: InspectorDock,
) -> Result<ContentBounds, &'static str> {
    if !(1..=16384).contains(&width) || !(1..=16384).contains(&height) || !(48..=768).contains(&dpi)
    {
        return Err("native viewport dimensions/DPI outside bounds");
    }
    let scale = f64::from(dpi) / 96.0;
    let top = (CHROME_TOP_DIP * scale).round() as i32;
    let right = if dock == InspectorDock::Right {
        (f64::from(width) * 0.32).min(RIGHT_MAX_DIP * scale).round() as i32
    } else {
        0
    };
    let bottom = if dock == InspectorDock::Bottom {
        (f64::from(height) * 0.40)
            .min(BOTTOM_MAX_DIP * scale)
            .round() as i32
    } else {
        0
    };
    let edge = (crate::frame_geometry::RESIZE_DIP * scale).round() as i32;
    let bounds = ContentBounds {
        x: edge,
        y: top,
        width: width - right - edge * 2,
        height: height - top - bottom - edge,
    };
    if bounds.width < 1 || bounds.height < 1 {
        return Err("no positive CEF page allocation");
    }
    Ok(bounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collapsed_content_meets_default_browser_first_height_budget() {
        let bounds = content_bounds(1280, 800, 96, InspectorDock::Hidden).unwrap();
        assert_eq!(
            bounds,
            ContentBounds {
                x: 5,
                y: 84,
                width: 1270,
                height: 711
            }
        );
        assert!(f64::from(bounds.height) / 800.0 >= 0.885);
    }
    #[test]
    fn docking_reallocates_the_same_positive_page_at_multiple_dpi() {
        for dpi in [96, 120, 144, 192] {
            let width = 1280 * dpi as i32 / 96;
            let height = 800 * dpi as i32 / 96;
            let full = content_bounds(width, height, dpi, InspectorDock::Hidden).unwrap();
            let right = content_bounds(width, height, dpi, InspectorDock::Right).unwrap();
            let bottom = content_bounds(width, height, dpi, InspectorDock::Bottom).unwrap();
            assert_eq!(right.height, full.height);
            assert_eq!(bottom.width, full.width);
            assert!(right.width > 0 && right.width < full.width);
            assert!(bottom.height > 0 && bottom.height < full.height);
            assert_eq!(
                content_bounds(width, height, dpi, InspectorDock::Hidden).unwrap(),
                full
            );
        }
    }
    #[test]
    fn invalid_or_empty_geometry_is_rejected() {
        for (width, height, dpi) in [
            (0, 800, 96),
            (1280, 1, 96),
            (1280, 800, 0),
            (20000, 800, 96),
        ] {
            assert!(content_bounds(width, height, dpi, InspectorDock::Hidden).is_err());
        }
    }
}
