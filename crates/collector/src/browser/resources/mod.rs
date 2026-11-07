//! Shared browser callback and timer ownership; no executable source strings.
mod timer;
pub(crate) use timer::{Settlement, Timer, settle};
