//! Pure native-address composition policy; no timing or motor-richness predicate.
/// Composition/candidate-confirm Enter barrier for the focused native address.
#[derive(Default)]
pub struct ImeGuard {
    composing: bool,
    confirmation: bool,
}
impl ImeGuard {
    /// Remember active composition, including an empty preedit before confirmation.
    pub fn preedit(&mut self, active: bool) {
        self.composing = active;
        self.confirmation |= active;
    }
    /// Commit may precede its confirming Enter key event.
    pub fn commit(&mut self) {
        self.composing = false;
        self.confirmation = true;
    }
    /// Cancel a pending composition on IME disable or native focus loss.
    pub fn cancel(&mut self) {
        self.confirmation |= self.composing;
        self.composing = false;
    }
    /// Repeats and foreign focus never consume the pending confirmation barrier.
    pub fn enter(&mut self, focused: bool, repeat: bool) -> bool {
        if !focused || repeat {
            return false;
        }
        if self.composing || self.confirmation {
            self.confirmation = false;
            return false;
        }
        true
    }
    /// A later ordinary key is separate from the previous commit confirmation.
    pub fn ordinary_key(&mut self) {
        self.confirmation = false;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commit_preedit_order_never_turns_confirmation_into_navigation() {
        let mut guard = ImeGuard::default();
        guard.preedit(true);
        guard.preedit(false);
        guard.commit();
        assert!(!guard.enter(true, false));
        assert!(guard.enter(true, false));
        guard.preedit(true);
        guard.commit();
        guard.preedit(false);
        assert!(!guard.enter(true, false));
        assert!(guard.enter(true, false));
    }
    #[test]
    fn disabled_or_lost_focus_cancels_composition_without_a_permanent_lock() {
        let mut guard = ImeGuard::default();
        guard.preedit(true);
        guard.cancel();
        assert!(!guard.enter(true, false));
        assert!(guard.enter(true, false));
        guard.cancel();
        assert!(guard.enter(true, false));
    }
    #[test]
    fn repeats_or_foreign_focus_do_not_consume_confirmation() {
        let mut guard = ImeGuard::default();
        guard.commit();
        assert!(!guard.enter(false, false));
        assert!(!guard.enter(true, true));
        assert!(!guard.enter(true, false));
        assert!(guard.enter(true, false));
    }
    #[test]
    fn ordinary_editing_does_not_bypass_active_composition() {
        let mut guard = ImeGuard::default();
        guard.preedit(true);
        guard.ordinary_key();
        assert!(!guard.enter(true, false));
        guard.cancel();
        guard.ordinary_key();
        assert!(guard.enter(true, false));
    }
}
