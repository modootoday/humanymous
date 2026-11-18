//! Authenticated trace (legacy `Ledger.Trace`): recover, look up, verify HMAC.
//!
//! Outcomes stay distinct inside the trusted operator boundary. A forged
//! carrier carries the referenced record only as a diagnostic; it is never a
//! session attribution. Unknown and expired IDs are not bot evidence. No outcome
//! changes a score, verdict or ban; operator authorization and audit belong to
//! the runtime.

use std::time::Duration;

use super::{Ledger, Lookup, Record};
use crate::watermark::{
    Error,
    codec::Channel,
    frame::Limits,
    payload::{WatermarkKey, split_legacy_carrier},
    recovery,
};

/// The record referenced by a recovered carrier and how it was recovered.
#[derive(Clone)]
pub struct Attribution {
    /// Ledger ID read from the carrier.
    pub tag_id: u32,
    /// Committed issuance record (privileged).
    pub record: Record,
    /// Channels that agreed on the carrier; the first is the legacy channel.
    pub channels: Vec<Channel>,
    /// Whether the legacy carrier parse ignored suffix bytes.
    pub ignored_suffix: bool,
}

/// Trace outcome for a carrier that was recovered and parsed.
pub enum Trace {
    /// The carrier HMAC matches the live record: the issuance is identified.
    Verified(Attribution),
    /// A live record exists but the HMAC does not match (wrong key or forgery).
    /// Diagnostic only; never serialize this as a session attribution.
    Forged(Attribution),
    /// No live committed record has this ID (including pending reservations).
    Unknown {
        /// ID read from the carrier.
        tag_id: u32,
    },
    /// The record existed but is beyond retention and was removed.
    Expired {
        /// ID read from the carrier.
        tag_id: u32,
    },
}

impl Ledger {
    /// Recover a carrier from leaked bytes, resolve its record and verify it.
    ///
    /// Errors are returned for no frame, ambiguous carriers, carriers shorter
    /// than fourteen bytes, resource limits, clock regression and poisoning.
    pub fn trace(
        &self,
        key: &WatermarkKey,
        declared_mime: &str,
        leaked: &[u8],
        limits: Limits,
        now: Duration,
    ) -> Result<Trace, Error> {
        let recovered = recovery::recover(declared_mime, leaked, limits)?;
        let carrier = split_legacy_carrier(&recovered.carrier)?;
        let record = match self.lookup(carrier.tag_id, now)? {
            Lookup::Live(record) => record,
            Lookup::Expired => {
                return Ok(Trace::Expired {
                    tag_id: carrier.tag_id,
                });
            }
            Lookup::Unknown => {
                return Ok(Trace::Unknown {
                    tag_id: carrier.tag_id,
                });
            }
        };
        let verified = key.verify_legacy(record.payload(), carrier.tag)?;
        let attribution = Attribution {
            tag_id: carrier.tag_id,
            record,
            channels: recovered.channels,
            ignored_suffix: carrier.ignored_suffix,
        };
        Ok(if verified {
            Trace::Verified(attribution)
        } else {
            Trace::Forged(attribution)
        })
    }
}
