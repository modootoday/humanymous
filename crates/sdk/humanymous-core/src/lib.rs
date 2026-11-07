//! Shared security contracts and pure engines, organized by bounded context.
#![forbid(unsafe_code)]

pub mod behavior;
pub mod challenge;
pub mod integrity;
pub mod model;
pub mod scoring;
pub mod signals;
pub mod watermark;
