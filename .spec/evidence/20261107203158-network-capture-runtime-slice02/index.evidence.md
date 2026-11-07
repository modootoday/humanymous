---
id: evidence:root/20261107203158
title: RUNTIME-SLICE-02 Network Capture authority disposition
status: active
created: 2026-11-07T20:31:58Z
updated: 2026-11-07T20:31:58Z
files:
  - {path: disposition.json, sha256: 924121136309b6601c820b4436fdfe4de1fba370000e800f3a18a82067750bdc, bytes: 10810, store: repo}
rel:
  depends_on: [sot:root/structural-development, plan:root/20261107142752, sot:root/network-capture]
---

# Concrete owner disposition: source read-only

Root's accepted map places capture and network beside gateway, authority, audit
and native_security in crates/sdk/humanymous-runtime/src. Gateway retains the old
runtime application child; it must not become the new crate root accidentally.
This review inspected Runtime plan checkpointba588e22 and current source167093fd
(inventory HEAD2bf6cc68). Attached52file hashes bind the current source. Owned
source is clean; no handle/target lease. No move/import/manifest/compiler action.

## Five crate-visibility declarations: no sibling exception required

| Existing declaration | Required destination boundary | Reason |
|---|---|---|
| network/src/tls/mod.rs::is_grease | pub(in crate::network), or equivalent network ancestor scope | Called by network TLS/report transforms; Capture uses public parsers, not this helper |
| network/src/history/mod.rs::OrderedTime | pub(in crate::network) | Shared only by network history/correlation/traffic |
| OrderedTime::new | pub(in crate::network) | Same network-local construction |
| OrderedTime::check | pub(in crate::network) | Same network-local pre-mutation clock check |
| OrderedTime::commit | pub(in crate::network) | Same network-local clock commit |

There are zero former pub(crate) declarations in Capture. Keep all existing
pub(super) declarations at their original relative module level: TLS RecordBoundary,
H2 decode, network config fields, population adapter and traffic record helpers.
Flattening them into a context root would widen who can bypass invariants.
No Gateway/Authority/Audit/Native sibling needs to see OrderedTime or is_grease.

## Concrete path and authority hazards

1. Capture depends on Network, never the reverse. Capture's ClientHello parser/
   type, H2 Fingerprint/Setting and frozen FrameMonitor references become
   crate::network::{tls,http2} imports. Do not add a network-to-capture dependency
   or merge passive H2 state into serving flow control.
2. Every old crate::http2 reference in capture/tls/transport.rs belongs to
   crate::capture::http2 (Capture, Request, StreamMetadata). The rustdoc link in
   capture/http2/mod.rs belongs to crate::capture::tls::Connection. Network's
   crate::{history,correlation,http,http2,tls,tcp,residual} references belong to
   crate::network. Do not resolve either to gateway::tls or a root alias.
3. PendingHandshake.{tls,ingress}, Connection.{tls,ingress,hello,h2,failed},
   Ingress and Cancellation.{stream,cancelled} remain private to Capture's
   transport module. Gateway must call existing public methods; it must not
   acquire field access, raw socket extraction, a new success constructor or a
   resettable deadline. Existing rustls success and cancellation checks mint the
   owner, not a sibling context or supplied observation.
4. Registry/Detector/Limiter/Log mutable state and time fields remain private to
   their implementing modules. Constructors still validate finite limits/layout;
   siblings cannot initialize Config/Limits fields or clear histories directly.
   All errors precede domain mutation. Existing Outcome/Measurement/Record data
   remains deliberately constructible, unqualified observation data, NEVER a
   receipt just because sibling code now shares a package.
5. Preserve one Network ordered-time helper and generic MAD. Runtime gateway owns
   its separate private composite-progress/coverage adapter, not a second MAD or
   temporal store. GC/reset successful calls still commit clock even with zero
   records; empty key operations do not. Required loss remains latched by Gateway.
6. TLS/H2 names exist independently in capture, network and gateway. Preserve
   their contextual nesting and borrowed public return types. No forwarding-only
   crate, compatibility alias at root or fake fleet/backend constructor.

## Decisions Root must settle before the actual move

The hosted_transport integration test is currently an EXTERNAL crate consumer
using humanymous_capture::tls. After relocation its approved test target needs
the intended humanymous_runtime::capture public path, or a deliberate test-target
relocation. Do not expose owner fields or unqualified success constructors to
make a private-context fixture compile. Root owns that public API/target choice.
Keep the five required names/ignored execution policy and hosted-pilot rcgen
feature binding; implicit inclusion in a default suite is not an actual pilot.

Pure temporal modules require no visibility exception beyond their public API.
If Core or Platform requires a Network-private helper after integration, that is
a new dependency direction/authority exception requiring review, not permission
to retain package-wide pub(crate). Platform's lower-cluster integration precedes
this slice; Root owns all manifests/features/lock and cross-context move mapping.

## Exact regression anchors, not new execution

Retain all72network unit identities, including:

- history::anomaly::tests::constructor_rejects_vec_layout_overflow_without_allocating_samples
- history::rate::tests::constructor_rejects_vec_layout_overflow_without_allocating_hits
- history::anomaly::tests::regression_is_rejected_before_observe_gc_or_reset_mutates
- history::rate::tests::failure_preserves_hits_policy_cursor_and_clock
- correlation::registry::boundary_tests::full_arrival_ring_appends_before_prune_and_keeps_cutoff_equality
- traffic::tests::two_ja4_resumption_values_are_tolerated_but_three_rotate
- traffic::tests::oracle_backfill_is_address_only_and_snapshot_is_copied

Retain the five actual Capture identities:

- actual_tls_h1_never_claims_h2
- actual_h2_continuation_and_stream_binding
- actual_h2_wrong_stream_invalidates_all_capture
- actual_invalid_tls_never_creates_an_owner
- actual_supervisor_cancellation_revokes_the_connection_epoch

After coherent Root integration: fmt/check/strict Clippy and scoped units under
the new context paths; external hosted fixture build and explicitly allocated
actual socket pilots; Gateway/Authority consumer compilation; privacy negatives
that sibling modules cannot construct Capture owners or Network validated config
fields. Source/target/features must be bound. The old72L1/5actual receipts are
historical, not new package proof. Real fleet, Runtime serving, verified epoch
association and Docker product remain separate open gates. No tests ran here.
