//! B23 fleet coordinator transport: the shared-state store several gate nodes
//! consult for bans, verdicts, rate limits and correlation counts. A coordinator
//! outage degrades to the caller's local view; it never claims fleet completeness.
pub mod correlation;
pub mod ledger;
pub mod redis;
