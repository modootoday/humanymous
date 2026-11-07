//! Bounded native qualification receipts, distinct from product readiness.
use crate::native;
#[derive(Default, serde::Serialize)]
pub(super) struct Report {
    pub(super) tabs: Vec<TabReceipt>,
    pub(super) tabs_created: usize,
    pub(super) tabs_before_close: usize,
    pub(super) tabs_incomplete: usize,
    pub(super) host_pid: u32,
    pub(super) navigation_schema: u32,
    pub(super) renderer_file_probe_schema: u32,
    pub(super) navigation_receipts: Vec<crate::navigation::Receipt>,
    pub(super) navigation_receipts_dropped: u64,
    pub(super) internal_probe_requested: bool,
    pub(super) internal_probe_finished: bool,
    pub(super) internal_probe_observed: usize,
    pub(super) internal_probe_passed: bool,
    pub(super) decorations_removed_readback: bool,
    pub(super) window_observations: Vec<crate::window_frame::Observation>,
    pub(super) window_commands: Vec<String>,
    pub(super) cancellation_requested: bool,
    pub(super) late_created_after_cancel: bool,
    pub(super) cef_child_created: bool,
    pub(super) actual_parent_matched: bool,
    pub(super) resize_readbacks: usize,
    pub(super) focus_observed: bool,
    pub(super) before_close: bool,
    pub(super) host_closed: bool,
    pub(super) process_tokens: Vec<native::ProcessToken>,
    pub(super) process_roles: Vec<crate::child::ProcessRole>,
    pub(super) process_info_status: &'static str,
    pub(super) renderer_restricted_token_observed: bool,
    pub(super) sandbox_qualified: bool,
    pub(super) ime_candidate_placement: &'static str,
    pub(super) accessibility_qualified: bool,
    pub(super) target_navigation_enabled: bool,
    pub(super) cef_key_events: usize,
    pub(super) cef_editable_key_events: usize,
    pub(super) cef_focus_events: usize,
    pub(super) parent_key_events: usize,
    pub(super) renderer_file_probes: Vec<crate::child::FileProbe>,
    pub(super) input_snapshots: Vec<native::InputSnapshot>,
    pub(super) ime_setup: Option<String>,
    pub(super) ime_candidate_request: Option<String>,
    pub(super) measured_hanja: Vec<String>,
    pub(super) hanja_key_events: usize,
    pub(super) owned_surface_transitions: Vec<Vec<crate::measured_input::Surface>>,
    pub(super) page_allocations: Vec<([i32; 4], u32)>,
}

#[derive(serde::Serialize)]
pub(super) struct TabReceipt {
    tab_id: usize,
    browser_id: Option<i32>,
    creation_requested: bool,
    created: bool,
    before_close: bool,
    committed_route: Option<&'static str>,
    navigation_receipts: Vec<crate::navigation::Receipt>,
    navigation_receipts_dropped: u64,
}

impl Report {
    pub(super) fn observe_tabs(&mut self, tabs: &[crate::child::Shared]) -> std::io::Result<()> {
        for tab in tabs {
            let tab = tab
                .lock()
                .map_err(|_| std::io::Error::other("tab state poisoned"))?;
            self.tabs.push(TabReceipt {
                tab_id: tab.tab_id,
                browser_id: tab.browser_id,
                creation_requested: tab.creation_requested,
                created: tab.created,
                before_close: tab.closed,
                committed_route: tab.navigation.committed().map(|(_, route)| route.url()),
                navigation_receipts: tab.navigation.receipts.clone(),
                navigation_receipts_dropped: tab.navigation.receipts_dropped,
            });
            if tab.creation_requested || tab.created {
                self.tabs_created += 1;
                if tab.closed {
                    self.tabs_before_close += 1;
                } else {
                    self.tabs_incomplete += 1;
                }
            }
        }
        Ok(())
    }
    pub(super) fn observe_navigation(&mut self, state: &crate::navigation::State) {
        if let Some(probe) = &state.probe {
            self.internal_probe_finished = probe.finished();
            self.internal_probe_observed = probe.observed();
            self.internal_probe_passed = probe.passed() && state.receipts_dropped == 0;
        }
    }
}

impl Report {
    /// Project one consistent post-loop state snapshot; no CEF or OS calls here.
    pub(super) fn observe_state(&mut self, state: &crate::child::State, host_closed: bool) {
        self.navigation_receipts = state.navigation.receipts.clone();
        self.navigation_receipts_dropped = state.navigation.receipts_dropped;
        self.observe_navigation(&state.navigation);
        self.cef_key_events = state.key_events;
        self.cef_editable_key_events = state.editable_key_events;
        self.cef_focus_events = state.focus_events;
        self.renderer_file_probes = state.file_probes.clone();
        self.ime_setup = state.ime_setup.clone();
        self.ime_candidate_request = state.ime_candidate_request.clone();
        self.measured_hanja = state.measured_hanja.clone();
        self.hanja_key_events = state.hanja_key_events;
        if self.ime_candidate_request.is_some() {
            self.ime_candidate_placement = "request_attempted_unqualified";
        }
        if !self.measured_hanja.is_empty() {
            self.ime_candidate_placement =
                "measured_key_attempted_external_visual_verification_required";
        }
        self.cef_child_created = state.created;
        self.before_close = state.closed;
        self.host_closed = host_closed;
        self.late_created_after_cancel = state.late_created_after_cancel;
        self.process_roles = state.process_roles.clone();
        self.process_info_status = state.process_info_status;
        let renderers: Vec<_> = self
            .process_tokens
            .iter()
            .filter(|p| p.role.as_deref() == Some("renderer"))
            .collect();
        self.renderer_restricted_token_observed = !renderers.is_empty()
            && renderers
                .iter()
                .all(|t| t.restricted && t.integrity_rid == 0);
    }
}
