//! Fixed monotonic resource window, not a client identity or detection signal.
use std::time::Duration;
pub(super) struct Window {
    start: Duration,
    last: Duration,
    count: u32,
    limit: u32,
    poisoned: bool,
}
impl Window {
    pub(super) fn new(limit: u32) -> Self {
        Self {
            start: Duration::ZERO,
            last: Duration::ZERO,
            count: 0,
            limit,
            poisoned: false,
        }
    }
    pub(super) fn admit(&mut self, elapsed: Duration) -> bool {
        if self.poisoned || elapsed < self.last {
            self.poisoned = true;
            return false;
        }
        self.last = elapsed;
        if elapsed - self.start >= Duration::from_secs(1) {
            self.start = elapsed;
            self.count = 0;
        }
        if self.count >= self.limit {
            return false;
        }
        self.count += 1;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quota_is_bounded_and_reopens_only_at_monotonic_window_boundary() {
        let mut window = Window::new(2);
        assert!(window.admit(Duration::ZERO));
        assert!(window.admit(Duration::from_millis(5)));
        assert!(!window.admit(Duration::from_millis(999)));
        assert!(window.admit(Duration::from_secs(1)));
        assert!(window.admit(Duration::from_millis(1001)));
        assert!(!window.admit(Duration::from_millis(1999)));
        assert!(window.admit(Duration::from_secs(2)));
    }
    #[test]
    fn rollback_cannot_restore_an_exhausted_or_poisoned_window() {
        let mut window = Window::new(1);
        assert!(window.admit(Duration::from_millis(500)));
        assert!(!window.admit(Duration::from_millis(499)));
        assert!(!window.admit(Duration::from_secs(2)));
        assert!(!Window::new(0).admit(Duration::ZERO));
    }
}
