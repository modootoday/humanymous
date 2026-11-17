//! B23 fleet coordinator transport: the shared-state store several gate nodes
//! consult for bans, verdicts and rate limits. A coordinator outage degrades to
//! the caller's local view; it never claims fleet completeness.
pub mod ledger;
pub mod redis;
