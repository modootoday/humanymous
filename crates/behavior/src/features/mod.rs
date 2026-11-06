//! Frozen ordered feature schema and extraction.
mod extraction;
mod schema;

pub use extraction::{extract, has_key, has_pointer};
pub use schema::{FEATURE_DIM, FEATURE_NAMES, FEATURE_SCHEMA_VERSION, FeatureVector, schema_hash};
