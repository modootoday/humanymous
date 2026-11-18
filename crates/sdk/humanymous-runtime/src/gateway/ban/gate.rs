//! Front-of-path ban gate kernel (SoT-27 sections 1-3, HR-21).
//!
//! Port of legacy internal/gate gates.go `banGate` and gate.go `enforceBan` and
//! `banReason` at baseline 981d25c, as a pure decision over a [`BanLedger`].
//! The gate meters the `ip:` key and, when the request has a header
//! fingerprint, the `fp:` key, so an IP-rotating flood is still banned by its
//! fingerprint. Records are returned in emission order; the caller appends
//! each one durably before it acts, so a block is never served unaudited.
//!
//! Nothing here is wired into `Application`: the owner decision of 2026-11-16
//! leaves serving composition undecided. Deliberate differences, pinned by
//! tests:
//! - A ledger refusal (capacity, rate history) is returned as an error; the
//!   caller must refuse the request. Go's store could not fail.
//! - Records use the Runtime verdict vocabulary `DENY`, as
//!   `smuggle::spoof_record` does, and carry no `key_id` (Go wrote `deny` and
//!   `k1`). A dual-control ban renders as `manual`, its operator origin.
use super::{Entry, Key, Outcome, Refusal, Source};
use crate::audit::record::{Actor, Record};
use crate::gateway::seams::BanLedger;
use crate::network::history::rate::Level;
use std::net::IpAddr;
use std::time::Duration;

/// HTTP status of a blocked request.
pub const BLOCK_STATUS: u16 = 403;
/// Static body of a blocked request; no ban detail is reflected.
pub const BLOCK_BODY: &str = "Source temporarily blocked (rate limit / abuse protection).";
/// Incident pointer header; the detail lives only in the audit chain.
pub const INCIDENT_HEADER: (&str, &str) = ("X-Hmn-Incident", "see-audit-log");

/// Settings-driven module mode of `gate.ban` (SoT-39).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Gate skipped: nothing is checked, counted or recorded.
    Off,
    /// Audit only; records carry mode `monitor`.
    Monitor,
    /// Audit only; records carry mode `shadow`.
    Shadow,
    /// Enforce bans and apply automatic bans.
    Enforce,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::Shadow => "shadow",
            Self::Enforce => "enforce",
            Self::Off | Self::Monitor => "monitor",
        }
    }
}

/// Request context copied into records; no raw address or header bytes.
#[derive(Debug, Clone, Copy)]
pub struct Subject<'a> {
    /// Node or tenant identifier.
    pub tenant: &'a str,
    /// Purpose-scoped subject pseudonym; empty without a session.
    pub pseudonym: &'a str,
    /// Request route class (`html`, `api`, `static`, `upgrade`).
    pub route_class: &'a str,
}

/// An enforced block. The caller answers [`BLOCK_STATUS`] with [`headers`]
/// and [`BLOCK_BODY`] and never contacts the scorer or the origin.
///
/// [`headers`]: Block::headers
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The ban in force.
    pub entry: Entry,
    /// Whole seconds until a temporary ban ends; `None` when permanent or due.
    pub retry_after: Option<u64>,
    /// A fresh auto-ban reached the legacy permanent tier; propose dual control.
    pub dual_control_required: bool,
}

impl Block {
    /// Response metadata: the incident pointer and, for temporary bans, Retry-After.
    pub fn headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![(INCIDENT_HEADER.0, INCIDENT_HEADER.1.to_owned())];
        if let Some(seconds) = self.retry_after {
            headers.push(("Retry-After", seconds.to_string()));
        }
        headers
    }
}

/// Gate result: records to append in order, and the block if enforced.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Gate {
    /// Audit records in emission order.
    pub records: Vec<Record>,
    /// `Some` when the request must be dropped at the edge.
    pub block: Option<Block>,
}

/// Keys metered for one request: `ip:` always, `fp:` with a header fingerprint
/// (`token::bind_key`, which is `None` without a User-Agent).
pub fn keys(ip: IpAddr, fingerprint: Option<&str>) -> Result<Vec<Key>, Refusal> {
    let mut keys = vec![Key::parse(&format!("ip:{ip}"))?];
    if let Some(fingerprint) = fingerprint {
        keys.push(Key::parse(&format!("fp:{fingerprint}"))?);
    }
    Ok(keys)
}

/// Compact ban descriptor for audit records (legacy `banReason`).
pub fn reason(entry: &Entry) -> String {
    let source = match entry.source {
        Source::Auto => "auto",
        Source::Manual | Source::DualControl => "manual",
    };
    let term = if entry.permanent() {
        "permanent".to_owned()
    } else {
        format!("strike {}", entry.strike)
    };
    format!("{source} ban ({term}): {}", entry.reason)
}

fn record(event: &str, actor: &str, subject: &Subject, action: &str, mode: Mode) -> Record {
    Record {
        event_type: event.into(),
        actor: Actor {
            kind: actor.into(),
            id_pseudonym: if actor == "subject" {
                subject.pseudonym.into()
            } else {
                String::new()
            },
        },
        tenant_id: subject.tenant.into(),
        route_class: subject.route_class.into(),
        enforcement_action: action.into(),
        enforcement_mode: mode.name().into(),
        ..Default::default()
    }
}

fn ban_record(
    event: &str,
    actor: &str,
    subject: &Subject,
    action: &str,
    mode: Mode,
    fail_reason: String,
) -> Record {
    Record {
        verdict: "DENY".into(),
        triggered_rules: Some(vec!["HR-21".into()]),
        fail_reason,
        ..record(event, actor, subject, action, mode)
    }
}

fn enforce(
    gate: &mut Gate,
    subject: &Subject,
    entry: Entry,
    dual_control_required: bool,
    now: Duration,
) {
    gate.records.push(ban_record(
        "ban.enforced",
        "subject",
        subject,
        "block",
        Mode::Enforce,
        reason(&entry),
    ));
    let retry_after = (!entry.permanent())
        .then(|| entry.remaining(now).as_secs())
        .filter(|seconds| *seconds > 0);
    gate.block = Some(Block {
        entry,
        retry_after,
        dual_control_required,
    });
}

/// Run the ban gate for one request in legacy order: every key is checked for
/// an active ban first, then every key is metered. Monitor and shadow modes
/// record what enforcement would do and never create or enforce a ban.
pub fn evaluate<B: BanLedger + ?Sized>(
    ledger: &mut B,
    keys: &[Key],
    mode: Mode,
    subject: &Subject,
    now: Duration,
) -> Result<Gate, Refusal> {
    let mut gate = Gate::default();
    if mode == Mode::Off {
        return Ok(gate);
    }
    for key in keys {
        if let Some(entry) = ledger.check(key, now) {
            if mode != Mode::Enforce {
                gate.records.push(ban_record(
                    "ban.enforced",
                    "subject",
                    subject,
                    "would_block",
                    mode,
                    format!("{} (audit only)", reason(&entry)),
                ));
                continue;
            }
            enforce(&mut gate, subject, entry, false, now);
            return Ok(gate);
        }
    }
    for key in keys {
        if mode != Mode::Enforce {
            let (event, action) = match ledger.observe_rate(key, now)? {
                Level::Normal => continue,
                Level::Soft => ("ratelimit.soft_exceeded", "would_rate_limit"),
                Level::Hard => ("ratelimit.hard_exceeded", "would_auto_ban"),
            };
            gate.records
                .push(record(event, "subject", subject, action, mode));
            continue;
        }
        let (entry, dual_control_required) = match ledger.observe(key, now)? {
            Outcome::Clear(Level::Soft) => {
                gate.records.push(record(
                    "ratelimit.soft_exceeded",
                    "subject",
                    subject,
                    "rate_limit",
                    mode,
                ));
                continue;
            }
            Outcome::Clear(_) => continue,
            Outcome::Banned(entry) => (entry, false),
            Outcome::AutoBanned {
                entry,
                dual_control_required,
            } => (entry, dual_control_required),
        };
        gate.records.push(ban_record(
            "ban.applied",
            "system",
            subject,
            "block",
            mode,
            reason(&entry),
        ));
        enforce(&mut gate, subject, entry, dual_control_required, now);
        return Ok(gate);
    }
    Ok(gate)
}

#[cfg(test)]
mod tests;
