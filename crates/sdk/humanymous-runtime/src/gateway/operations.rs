//! Operator console kernels ported from the legacy Gate admin and ops views
//! (internal/gate/admin.go, internal/gate/erasure.go, cmd/server/ops.go at
//! baseline 981d25c): audit search with cursor pagination, the cancellable
//! erasure hold window, and the `/api/counters` rollup.
//!
//! These are pure, owner-free decisions. Authentication, roles, HTTP routes,
//! dual-control approval and audit emission stay with the runtime and audit
//! owners; nothing here mints an erasure permit or appends an audit record.
pub mod audit_search;
pub mod counters;
pub mod erasure_hold;
