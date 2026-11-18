//! TTL-expiring merged session report store (legacy `internal/collector`
//! session.go and merge.go, baseline 981d25c; SoT-00 §5, plan/02 §2).
//!
//! It correlates a client report (L1-L4) with the first server network
//! observation (L5) into one [`SessionReport`] and keeps per-session RIT, PoW
//! and label state. Unlike the capability-issuing [`super::Sessions`] epoch,
//! entries are keyed by caller-supplied IDs exactly as in the Go store, so this
//! type must only receive IDs the server already authenticated. It is a
//! component port: the gateway request path does not consult it, and the SoT
//! keeps "no bootstrap token grace" for the Runtime application profile.
//! Callers serialize access (for example behind a `Mutex`) instead of the Go
//! internal `sync.RWMutex`.
use humanymous_core::signals::observation::{Signal, Source};
use humanymous_core::signals::report::{ClientReport, NetworkReport, SessionReport, Timestamp};
use std::collections::HashMap;
use std::time::Duration;

/// Accumulating record for one session plus its counters (Go `sessionState`).
#[derive(Debug, Clone, Default)]
struct SessionState {
    report: SessionReport,
    /// Last RIT counter a seed was issued for.
    rit_counter: u64,
    /// Whether the first network observation has been pinned.
    network_pinned: bool,
    /// SoT-13: the session solved a proof-of-work challenge.
    pow_solved: bool,
    /// When the current PoW challenge was first issued; zero time when none.
    pow_issued: Timestamp,
    /// Difficulty the current challenge was issued at; 0 when none.
    pow_difficulty: i32,
    /// SoT-13/15: the session ever produced a super-human valid solve.
    pow_too_fast: bool,
    /// SoT-07: the one-shot tokenless RIT grace has been consumed.
    rit_bootstrap: bool,
    updated: Timestamp,
}

impl SessionState {
    /// Go `&sessionState{}` with only the session ID set (zero timestamp).
    fn bare(id: &str) -> Self {
        let mut state = Self::default();
        state.report.session_id = id.to_owned();
        state
    }
}

/// In-memory, TTL-expiring merged session store (Go `collector.Store`).
#[derive(Debug, Clone, Default)]
pub struct CollectorStore {
    entries: HashMap<String, SessionState>,
    ttl: Duration,
}

impl CollectorStore {
    /// Empty store with the given TTL; no background GC, call [`Self::gc`].
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
        }
    }

    /// Create the session if needed (report ID and timestamp set to `now`)
    /// and refresh its activity time.
    pub fn ensure(&mut self, id: &str, now: Timestamp) {
        self.ensure_entry(id, now).updated = now;
    }

    /// Snapshot of a session report, if the session exists.
    pub fn get(&self, id: &str) -> Option<SessionReport> {
        self.entries.get(id).map(|state| state.report.clone())
    }

    /// Last issued RIT counter for a session (0 if unknown).
    pub fn rit_counter(&self, id: &str) -> u64 {
        self.entries.get(id).map_or(0, |state| state.rit_counter)
    }

    /// Increment and return the session RIT counter, creating a bare session
    /// (zero report timestamp, as in Go) when unknown. Wraps like Go `uint64`.
    pub fn advance_rit(&mut self, id: &str, now: Timestamp) -> u64 {
        let state = self.bare_entry(id);
        state.rit_counter = state.rit_counter.wrapping_add(1);
        state.updated = now;
        state.rit_counter
    }

    /// Consume the session's one-shot tokenless RIT grace: true the first time
    /// only, so a bot cannot camp in the grace state to suppress `l5.rit.absent`.
    pub fn claim_rit_bootstrap(&mut self, id: &str, now: Timestamp) -> bool {
        let state = self.bare_entry(id);
        state.updated = now;
        !std::mem::replace(&mut state.rit_bootstrap, true)
    }

    /// Mark an existing session as PoW-solved and reset the issue clock and
    /// difficulty for a fresh challenge cycle; unknown sessions are ignored.
    /// The too-fast observation is deliberately left untouched.
    pub fn set_pow_solved(&mut self, id: &str, now: Timestamp) {
        if let Some(state) = self.entries.get_mut(id) {
            state.pow_solved = true;
            state.pow_issued = Timestamp::default();
            state.pow_difficulty = 0;
            state.updated = now;
        }
    }

    /// First-write-wins record of when a PoW challenge was issued and at which
    /// difficulty. Re-issuing must neither reset the clock nor change the
    /// verification difficulty. Unknown sessions are ignored; the activity
    /// time is not refreshed (Go parity).
    pub fn set_pow_issued(&mut self, id: &str, now: Timestamp, difficulty: i32) {
        if let Some(state) = self.entries.get_mut(id)
            && state.pow_issued == Timestamp::default()
        {
            state.pow_issued = now;
            state.pow_difficulty = difficulty;
        }
    }

    /// Difficulty the current challenge was issued at (0 if none).
    pub fn pow_difficulty(&self, id: &str) -> i32 {
        self.entries.get(id).map_or(0, |state| state.pow_difficulty)
    }

    /// When the current challenge was issued (zero time if none).
    pub fn pow_issued_at(&self, id: &str) -> Timestamp {
        self.entries
            .get(id)
            .map_or_else(Timestamp::default, |state| state.pow_issued)
    }

    /// Permanently record a super-human PoW solve (`l7.pow.too_fast`); never
    /// cleared on solve. Unknown sessions are ignored.
    pub fn set_pow_too_fast(&mut self, id: &str) {
        if let Some(state) = self.entries.get_mut(id) {
            state.pow_too_fast = true;
        }
    }

    /// Whether the session ever produced a too-fast PoW solve.
    pub fn pow_too_fast(&self, id: &str) -> bool {
        self.entries.get(id).is_some_and(|state| state.pow_too_fast)
    }

    /// Whether the session solved a PoW challenge.
    pub fn pow_solved(&self, id: &str) -> bool {
        self.entries.get(id).is_some_and(|state| state.pow_solved)
    }

    /// Up to `limit` report snapshots in unspecified order for the admin
    /// console. As in Go, the bound is checked after each append, so a
    /// non-empty store returns at least one report even when `limit` is 0.
    pub fn list(&self, limit: usize) -> Vec<SessionReport> {
        let mut out = Vec::with_capacity(self.entries.len());
        for state in self.entries.values() {
            out.push(state.report.clone());
            if out.len() >= limit {
                break;
            }
        }
        out
    }

    /// Remove sessions idle for longer than the TTL.
    pub fn gc(&mut self, now: Timestamp) {
        let ttl = self.ttl;
        self.entries
            .retain(|_, state| (now.0 - state.updated.0) <= ttl);
    }

    /// Replace the client portion after dropping any signal that claims server
    /// provenance (`server` source or a reserved L5/L6/L7/cross-check ID).
    pub fn merge_client(&mut self, id: &str, mut client: ClientReport, now: Timestamp) {
        client.signals = client.signals.map(sanitize_client_signals);
        let state = self.ensure_entry(id, now);
        state.report.client = client;
        state.updated = now;
    }

    /// Pin the first network observation as the session TLS/H2 basis. Later
    /// observations keep the pin and append only request-scoped hop residuals.
    pub fn merge_network(&mut self, id: &str, network: NetworkReport, now: Timestamp) {
        let state = self.ensure_entry(id, now);
        if !state.network_pinned {
            state.report.network = network;
            state.network_pinned = true;
        } else {
            let residuals = network
                .signals
                .into_iter()
                .flatten()
                .filter(|signal| request_scoped_network(&signal.id));
            append(&mut state.report.network.signals, residuals);
        }
        state.updated = now;
    }

    /// Append extra L5 signals (for example RIT/resource) to the pinned network
    /// report without replacing it.
    pub fn append_network_signals(&mut self, id: &str, extra: Vec<Signal>, now: Timestamp) {
        let state = self.ensure_entry(id, now);
        append(&mut state.report.network.signals, extra);
        state.updated = now;
    }

    /// Record ground truth (`human` or `bot:<tool>`) for e2e evaluation.
    pub fn set_label(&mut self, id: &str, label: &str, now: Timestamp) {
        let state = self.ensure_entry(id, now);
        label.clone_into(&mut state.report.label);
        state.updated = now;
    }

    /// Persist the final scored report after the engine ran.
    pub fn store_scored(&mut self, id: &str, report: SessionReport, now: Timestamp) {
        let state = self.ensure_entry(id, now);
        state.report = report;
        state.updated = now;
    }

    /// Go `ensureLocked`: create with ID and timestamp, without touching
    /// the activity time of an existing entry.
    fn ensure_entry(&mut self, id: &str, now: Timestamp) -> &mut SessionState {
        self.entries.entry(id.to_owned()).or_insert_with(|| {
            let mut state = SessionState::bare(id);
            state.report.timestamp = now;
            state
        })
    }

    fn bare_entry(&mut self, id: &str) -> &mut SessionState {
        self.entries
            .entry(id.to_owned())
            .or_insert_with(|| SessionState::bare(id))
    }
}

/// Go `append` on a nil-able slice: an empty extension leaves `None` alone.
fn append(signals: &mut Option<Vec<Signal>>, extra: impl IntoIterator<Item = Signal>) {
    let mut extra = extra.into_iter().peekable();
    if extra.peek().is_some() {
        signals.get_or_insert_with(Vec::new).extend(extra);
    }
}

/// Drop forged server-provenance claims from a client report.
fn sanitize_client_signals(signals: Vec<Signal>) -> Vec<Signal> {
    signals
        .into_iter()
        .filter(|signal| signal.collected != Source::Server && !reserved_server_id(&signal.id))
        .collect()
}

/// Server-minted layers a client must never author.
fn reserved_server_id(id: &str) -> bool {
    ["l5.", "l6.", "l7.", "x."]
        .iter()
        .any(|prefix| id.starts_with(prefix))
}

/// IDs that re-emit on every collect after the network pin: hop headers and
/// IP intelligence depend on the current request path, not the first TLS.
fn request_scoped_network(id: &str) -> bool {
    matches!(
        id,
        "l5.header.proxy_hop"
            | "l5.header.client_ip_spoof"
            | "l5.header.xff_multi_hop"
            | "l5.header.forwarded_private"
            | "l5.proxy.tor_circuit"
            | "l5.proxy.anon_chain"
            | "l5.ip.proxy_vpn_tor"
            | "l5.ip.tor_exit"
            | "l5.ip.datacenter_asn"
    )
}

#[cfg(test)]
mod tests;
