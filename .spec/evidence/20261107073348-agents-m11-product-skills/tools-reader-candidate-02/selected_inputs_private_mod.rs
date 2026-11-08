//! Root's private maintenance context: import only consumed names.
pub(crate) mod files;
mod model;
mod reader;
mod request;
pub(crate) use request::{capture_request_file, capture_source_file};

pub use files::PreparationReceipt;
pub use model::{FrozenSelection, Input, Limits, Root, SelectionError};
pub use reader::{Clock, ContentRead, ReadBudget, ReadStatus, SystemClock};
pub use request::{MAX_REQUEST_BYTES, RequestError, observe_request_file};

#[cfg(test)]
mod tests;
