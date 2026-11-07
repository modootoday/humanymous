//! Pure bounded tab identity and cleanup accounting; native children live elsewhere.

/// Never-reused session-local tab identity.
pub type Id = usize;
/// Window-level commands accepted only from the current owned native surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Navigate the active child to an already classified local route.
    Open(crate::internal_urls::Route),
    /// Open the owner newtab in the active child.
    Home,
    /// Stop the active child and invalidate pending work.
    Stop,
    /// Create another real browser child.
    New,
    /// Close the current child through its lifecycle owner.
    Close,
    /// Close a particular visible tab.
    CloseId(Id),
    /// Select a particular visible tab.
    Select(Id),
    /// Cycle through live tabs; true means reverse.
    Cycle(bool),
    /// Focus the native address input.
    Address,
    /// Traverse engine history backwards.
    Back,
    /// Traverse engine history forwards.
    Forward,
    /// Reload the active engine page.
    Reload,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Open,
    Closing,
    Closed,
}
/// Tab membership never implies that CEF has finished creating a child.
pub struct Tabs {
    phases: Vec<Phase>,
    active: Option<Id>,
}
impl Default for Tabs {
    fn default() -> Self {
        Self {
            phases: vec![Phase::Open],
            active: Some(0),
        }
    }
}
impl Tabs {
    /// Maximum native children including closing children that have not acknowledged.
    pub const CAPACITY: usize = 8;
    /// Bounded lifetime IDs; closed identities never select replacement children.
    pub const CREATIONS: usize = 64;
    /// Current selected open tab, or none while the final tab closes.
    pub fn active(&self) -> Option<Id> {
        self.active
    }
    /// Open tabs in stable display order.
    pub fn open_ids(&self) -> Vec<Id> {
        self.phases
            .iter()
            .enumerate()
            .filter_map(|(i, p)| (*p == Phase::Open).then_some(i))
            .collect()
    }
    /// Admit a new identity only while the session has an open tab and capacity.
    pub fn create(&mut self) -> Result<Id, &'static str> {
        if self.active.is_none()
            || self.phases.len() >= Self::CREATIONS
            || self.phases.iter().filter(|p| **p != Phase::Closed).count() >= Self::CAPACITY
        {
            return Err("tab capacity unavailable");
        }
        let id = self.phases.len();
        self.phases.push(Phase::Open);
        self.active = Some(id);
        Ok(id)
    }
    /// Reject stale and closing identities.
    pub fn select(&mut self, id: Id) -> bool {
        if self.phases.get(id) != Some(&Phase::Open) {
            return false;
        }
        self.active = Some(id);
        true
    }
    /// Cycle without changing membership or cleanup accounting.
    pub fn cycle(&mut self, reverse: bool) {
        let ids = self.open_ids();
        if let Some(index) = ids.iter().position(|id| Some(*id) == self.active) {
            self.active = Some(
                ids[if reverse {
                    (index + ids.len() - 1) % ids.len()
                } else {
                    (index + 1) % ids.len()
                }],
            );
        }
    }
    /// Begin close; capacity remains reserved until the native acknowledgement.
    pub fn close(&mut self, id: Id) -> bool {
        if self.phases.get(id) != Some(&Phase::Open) {
            return false;
        }
        self.phases[id] = Phase::Closing;
        if self.active == Some(id) {
            let ids = self.open_ids();
            self.active = ids
                .iter()
                .copied()
                .find(|other| *other > id)
                .or_else(|| ids.last().copied());
        }
        true
    }
    /// Only the adapter's actual before-close observation releases a reservation.
    pub fn acknowledge_closed(&mut self, id: Id) -> bool {
        if self.phases.get(id) != Some(&Phase::Closing) {
            return false;
        }
        self.phases[id] = Phase::Closed;
        true
    }
}
/// Windows logical accelerator projection; does not intercept IME control keys.
pub fn shortcut(
    vk: u32,
    control: bool,
    alt: bool,
    shift: bool,
    meta: bool,
    repeat: bool,
) -> Option<Command> {
    if meta || repeat {
        return None;
    }
    match (vk, control, alt, shift) {
        (0x54, true, false, false) => Some(Command::New),
        (0x57, true, false, false) => Some(Command::Close),
        (0x09, true, false, reverse) => Some(Command::Cycle(reverse)),
        (0x4c, true, false, false) => Some(Command::Address),
        (0x52, true, false, false) | (0x74, false, false, false) => Some(Command::Reload),
        (0x25, false, true, false) => Some(Command::Back),
        (0x27, false, true, false) => Some(Command::Forward),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn close_capacity_waits_for_real_ack_and_ids_never_reuse() {
        let mut tabs = Tabs::default();
        for _ in 1..Tabs::CAPACITY {
            tabs.create().unwrap();
        }
        assert!(tabs.close(1));
        assert!(tabs.create().is_err());
        assert!(!tabs.select(1));
        assert!(tabs.acknowledge_closed(1));
        assert_eq!(tabs.create(), Ok(8));
        assert!(!tabs.select(1));
    }
    #[test]
    fn selected_neighbor_cycle_and_last_tab_close_are_deterministic() {
        let mut tabs = Tabs::default();
        tabs.create().unwrap();
        tabs.create().unwrap();
        tabs.cycle(false);
        assert_eq!(tabs.active(), Some(0));
        tabs.cycle(true);
        assert_eq!(tabs.active(), Some(2));
        assert!(tabs.close(2));
        assert_eq!(tabs.active(), Some(1));
        assert!(tabs.close(0));
        assert_eq!(tabs.active(), Some(1));
        assert!(tabs.close(1));
        assert_eq!(tabs.active(), None);
        assert!(tabs.create().is_err());
    }
    #[test]
    fn standard_shortcuts_leave_ime_escape_and_repeats_unclaimed() {
        for (vk, command) in [
            (0x57, Command::Close),
            (0x4c, Command::Address),
            (0x52, Command::Reload),
        ] {
            assert_eq!(
                shortcut(vk, true, false, false, false, false),
                Some(command)
            );
        }
        assert_eq!(
            shortcut(0x74, false, false, false, false, false),
            Some(Command::Reload)
        );
        assert_eq!(
            shortcut(0x54, true, false, false, false, false),
            Some(Command::New)
        );
        assert_eq!(
            shortcut(0x09, true, false, true, false, false),
            Some(Command::Cycle(true))
        );
        assert_eq!(
            shortcut(0x25, false, true, false, false, false),
            Some(Command::Back)
        );
        for vk in [0x0d, 0x1b, 0x15, 0x19] {
            assert_eq!(shortcut(vk, false, false, false, false, false), None);
        }
        assert_eq!(shortcut(0x54, true, false, false, false, true), None);
        assert_eq!(shortcut(0x54, true, true, false, false, false), None);
    }
    #[test]
    fn lifetime_budget_cannot_recycle_old_close_or_selection_authority() {
        let mut tabs = Tabs::default();
        for id in 1..Tabs::CREATIONS {
            assert_eq!(tabs.create(), Ok(id));
            assert!(tabs.close(id - 1));
            assert!(tabs.acknowledge_closed(id - 1));
        }
        assert!(tabs.create().is_err());
        assert!(!tabs.close(0));
        assert!(!tabs.select(0));
        assert_eq!(tabs.active(), Some(Tabs::CREATIONS - 1));
    }
}
