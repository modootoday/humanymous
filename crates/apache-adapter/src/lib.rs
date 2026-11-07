//! Safe Apache child/request ownership. Native module ABI is not yet implemented.
mod lifecycle;
pub use lifecycle::{Child, Error, Mpm, Request};
