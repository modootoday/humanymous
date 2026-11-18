//! Operator console kernels ported from the legacy Gate admin and ops views
//! (internal/gate/admin.go, internal/gate/erasure.go, cmd/server/ops.go at
//! baseline 981d25c): audit search with cursor pagination, the cancellable
//! erasure hold window, the `/api/counters` rollup and the `/api/mlcorrect`
//! operator snapshot (cmd/server/mlcorrect_ops.go).
//!
//! These are pure, owner-free decisions. Apart from the legacy operator-bearer
//! predicate (`mlcorrect::operator_authorized`, a constant-time comparison
//! against an owner-supplied token), authentication, roles, HTTP routes,
//! dual-control approval and audit emission stay with the runtime and audit
//! owners; nothing here mints an erasure permit or appends an audit record.
pub mod audit_search;
pub mod counters;
pub mod erasure_hold;
pub mod mlcorrect;
