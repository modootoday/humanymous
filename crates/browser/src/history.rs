//! Bounded validation of actual engine history, without inventing visited pages.
use crate::internal_urls::Route;

/// Engine-observed control availability; unknown defaults to unavailable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Controls {
    /// CEF reports a previous entry; dispatch still validates its route.
    pub back: bool,
    /// CEF reports a next entry; dispatch still validates its route.
    pub forward: bool,
    /// CEF is loading or host verification is pending.
    pub loading: bool,
}

/// Exact ordered CEF entry snapshot. Malformed or oversized streams fail closed.
#[derive(Default)]
pub struct Entries {
    routes: Vec<Route>,
    current: Option<usize>,
    total: Option<usize>,
    invalid: bool,
}
impl Entries {
    /// Append the next engine callback, retaining typed routes only.
    pub fn push(&mut self, route: Option<Route>, current: bool, index: i32, total: i32) -> bool {
        let valid = (1..=128).contains(&total)
            && index >= 0
            && index as usize == self.routes.len()
            && self.total.is_none_or(|n| n == total as usize)
            && index < total
            && route.is_some()
            && (!current || self.current.is_none());
        if self.invalid || !valid {
            self.invalid = true;
            return false;
        }
        self.total = Some(total as usize);
        if current {
            self.current = Some(index as usize);
        }
        if let Some(route) = route {
            self.routes.push(route);
        }
        true
    }
    /// Adjacent target only after a complete stream matches the current host page.
    pub fn target(&self, forward: bool, committed: Route) -> Option<Route> {
        if self.invalid || self.total != Some(self.routes.len()) {
            return None;
        }
        let index = self.current?;
        if self.routes.get(index) != Some(&committed) {
            return None;
        }
        let target = if forward {
            index.checked_add(1)?
        } else {
            index.checked_sub(1)?
        };
        self.routes.get(target).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal_urls::InternalPage;
    #[test]
    fn actual_adjacent_entries_and_complete_current_identity_are_required() {
        let version = Route::Internal(InternalPage::Version);
        let mut entries = Entries::default();
        assert!(entries.push(Some(Route::NewTab), false, 0, 3));
        assert!(entries.push(Some(version), true, 1, 3));
        assert_eq!(entries.target(false, version), None);
        assert!(entries.push(Some(Route::NewTab), false, 2, 3));
        assert_eq!(entries.target(false, version), Some(Route::NewTab));
        assert_eq!(entries.target(true, version), Some(Route::NewTab));
        assert_eq!(entries.target(false, Route::NewTab), None);
    }
    #[test]
    fn malformed_foreign_duplicate_and_oversized_entries_remain_denied() {
        for (route, index, total) in [
            (None, 0, 1),
            (Some(Route::NewTab), 1, 2),
            (Some(Route::NewTab), 0, 129),
        ] {
            let mut entries = Entries::default();
            assert!(!entries.push(route, true, index, total));
            assert_eq!(entries.target(false, Route::NewTab), None);
        }
        let mut entries = Entries::default();
        assert!(entries.push(Some(Route::NewTab), true, 0, 2));
        assert!(!entries.push(Some(Route::NewTab), true, 1, 2));
        assert_eq!(entries.target(true, Route::NewTab), None);
    }
}
