//! Bounded observations of a caller-frozen selection, never a security snapshot.
mod files;
mod model;
mod reader;
mod request;

pub use files::observe;
pub use files::{FileObservation, PreparationReceipt, Stability};
pub use model::{FrozenSelection, Input, Limits, Root, SelectionError};
pub use reader::{Clock, ContentRead, ReadBudget, ReadStatus, SystemClock};
pub use request::{
    CliReceipt, MAX_REQUEST_BYTES, RequestError, decode_request, observe_request_file,
};

#[cfg(test)]
mod tests;
