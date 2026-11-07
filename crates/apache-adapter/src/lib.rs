//! Safe Apache child/request ownership. Native module ABI is not yet implemented.
mod lifecycle;
pub use lifecycle::{Cancellation, Child, Error, Mpm, Request};
