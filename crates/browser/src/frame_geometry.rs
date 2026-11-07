//! Pure borderless-window resize geometry shared by native input and page allocation.
/// App-owned perimeter width in device-independent pixels.
pub const RESIZE_DIP: f64 = 3.0;
/// Resize directions. Opposite sides remain fixed when minimum size is reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    /// Left.
    West,
    /// Right.
    East,
    /// Top.
    North,
    /// Bottom.
    South,
    /// Top left.
    NorthWest,
    /// Top right.
    NorthEast,
    /// Bottom left.
    SouthWest,
    /// Bottom right.
    SouthEast,
}
/// Return an edge only inside a valid window's DPI-scaled hit strip.
pub fn hit(width: i32, height: i32, dpi: u32, x: f64, y: f64) -> Option<Edge> {
    if width < 1
        || height < 1
        || !(48..=768).contains(&dpi)
        || !x.is_finite()
        || !y.is_finite()
        || x < 0.0
        || y < 0.0
        || x >= f64::from(width)
        || y >= f64::from(height)
    {
        return None;
    }
    let edge = (RESIZE_DIP * f64::from(dpi) / 96.0).round();
    let west = x < edge;
    let east = x >= f64::from(width) - edge;
    let north = y < edge;
    let south = y >= f64::from(height) - edge;
    match (west, east, north, south) {
        (true, _, true, _) => Some(Edge::NorthWest),
        (_, true, true, _) => Some(Edge::NorthEast),
        (true, _, _, true) => Some(Edge::SouthWest),
        (_, true, _, true) => Some(Edge::SouthEast),
        (true, _, _, _) => Some(Edge::West),
        (_, true, _, _) => Some(Edge::East),
        (_, _, true, _) => Some(Edge::North),
        (_, _, _, true) => Some(Edge::South),
        _ => None,
    }
}
/// Apply a physical delta to a known valid original rectangle, with DPI-scaled640x480 DIP minimum and bounded physical size.
pub fn resize(
    rect: [i32; 4],
    edge: Edge,
    dx: i32,
    dy: i32,
    dpi: u32,
) -> Result<[i32; 4], &'static str> {
    if !(48..=768).contains(&dpi) {
        return Err("resize DPI unavailable or outside bounds");
    }
    let min_width = (640.0 * f64::from(dpi) / 96.0).ceil() as i32;
    let min_height = (480.0 * f64::from(dpi) / 96.0).ceil() as i32;
    let [mut left, mut top, mut right, mut bottom] = rect;
    if matches!(edge, Edge::West | Edge::NorthWest | Edge::SouthWest) {
        left = left
            .saturating_add(dx)
            .clamp(right.saturating_sub(16384), right.saturating_sub(min_width));
    }
    if matches!(edge, Edge::East | Edge::NorthEast | Edge::SouthEast) {
        right = right
            .saturating_add(dx)
            .clamp(left.saturating_add(min_width), left.saturating_add(16384));
    }
    if matches!(edge, Edge::North | Edge::NorthWest | Edge::NorthEast) {
        top = top.saturating_add(dy).clamp(
            bottom.saturating_sub(16384),
            bottom.saturating_sub(min_height),
        );
    }
    if matches!(edge, Edge::South | Edge::SouthWest | Edge::SouthEast) {
        bottom = bottom
            .saturating_add(dy)
            .clamp(top.saturating_add(min_height), top.saturating_add(16384));
    }
    Ok([left, top, right, bottom])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eight_edges_scale_and_never_capture_the_interior() {
        for dpi in [96, 144, 192] {
            for (x, y, expected) in [
                (0., 0., Edge::NorthWest),
                (999., 0., Edge::NorthEast),
                (0., 799., Edge::SouthWest),
                (999., 799., Edge::SouthEast),
                (0., 400., Edge::West),
                (999., 400., Edge::East),
                (500., 0., Edge::North),
                (500., 799., Edge::South),
            ] {
                assert_eq!(hit(1000, 800, dpi, x, y), Some(expected));
            }
            assert_eq!(hit(1000, 800, dpi, 500., 400.), None);
        }
        assert_eq!(hit(1000, 800, 0, 0., 0.), None);
        assert_eq!(hit(1000, 800, 96, f64::NAN, 0.), None);
    }
    #[test]
    fn opposite_edges_stay_fixed_and_extreme_deltas_clamp() {
        let rect = [100, 100, 1100, 900];
        assert_eq!(
            resize(rect, Edge::SouthEast, 100, 50, 96).unwrap(),
            [100, 100, 1200, 950]
        );
        assert_eq!(
            resize(rect, Edge::NorthWest, 10000, 10000, 96).unwrap(),
            [460, 420, 1100, 900]
        );
        assert_eq!(
            resize(rect, Edge::SouthEast, i32::MAX, i32::MAX, 96).unwrap(),
            [100, 100, 16484, 16484]
        );
        assert_eq!(
            resize(rect, Edge::West, -50, 0, 96).unwrap(),
            [50, 100, 1100, 900]
        );
    }
    #[test]
    fn thin_resize_band_stops_before_the_page_at_each_dpi() {
        for dpi in [96, 120, 144, 192] {
            let edge = (RESIZE_DIP * f64::from(dpi) / 96.0).round();
            assert_eq!(hit(1280, 800, dpi, edge - 0.5, 400.0), Some(Edge::West));
            assert_eq!(hit(1280, 800, dpi, edge, 400.0), None);
            assert_eq!(hit(1280, 800, dpi, 600.0, 800.0 - edge), Some(Edge::South));
            assert_eq!(hit(1280, 800, dpi, 600.0, 800.0 - edge - 0.5), None);
        }
    }
    #[test]
    fn dpi_scaled_minimum_keeps_every_dock_drawable() {
        for dpi in [48, 96, 120, 144, 192, 384, 768] {
            let rect = resize(
                [0, 0, 16000, 16000],
                Edge::SouthEast,
                i32::MIN,
                i32::MIN,
                dpi,
            )
            .unwrap();
            for dock in [
                crate::viewport::InspectorDock::Hidden,
                crate::viewport::InspectorDock::Right,
                crate::viewport::InspectorDock::Bottom,
            ] {
                assert!(crate::viewport::content_bounds(rect[2], rect[3], dpi, dock).is_ok());
            }
        }
        assert!(resize([0, 0, 1000, 800], Edge::East, 0, 0, 0).is_err());
    }
}
