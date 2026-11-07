//! Closed, non-sensitive failure classes for issuance primitives.

/// A watermark operation failed without producing successful attribution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// Empty or oversized secret material.
    InvalidKey,
    /// New issuance has an invalid ID or ambiguous/oversized identifiers.
    InvalidPayload,
    /// A recovered carrier has the wrong length.
    InvalidCarrier,
    /// Configuration is zero, oversized or inconsistent.
    InvalidConfiguration,
    /// The trusted clock moved behind an already observed instant.
    ClockRegression,
    /// The bounded ledger cannot admit another live entry.
    Capacity,
    /// The non-repeating legacy ID space is exhausted.
    IdExhausted,
    /// Synchronization failed; no poisoned state is used for attribution.
    Poisoned,
    /// The reservation expired or was already removed.
    ReservationExpired,
    /// Issuance metadata does not belong to the reserved ID.
    ReservationMismatch,
    /// MIME/channel/pseudonym metadata exceeds the accepted bounds.
    InvalidMetadata,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidKey => "invalid watermark key",
            Self::InvalidPayload => "invalid watermark payload",
            Self::InvalidCarrier => "invalid watermark carrier",
            Self::InvalidConfiguration => "invalid watermark configuration",
            Self::ClockRegression => "watermark clock regression",
            Self::Capacity => "watermark ledger capacity reached",
            Self::IdExhausted => "watermark identifier space exhausted",
            Self::Poisoned => "watermark ledger unavailable",
            Self::ReservationExpired => "watermark reservation expired",
            Self::ReservationMismatch => "watermark reservation mismatch",
            Self::InvalidMetadata => "invalid watermark metadata",
        })
    }
}

impl std::error::Error for Error {}
