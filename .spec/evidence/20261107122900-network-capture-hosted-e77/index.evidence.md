---
id: evidence:root/20261107122900
title: Genuine hosted five-case TLS and H2 capture qualification
status: active
created: 2026-11-07T12:29:00Z
updated: 2026-11-07T12:29:00Z
files:
  - {path: receipt.json, sha256: c3aacd9898056e758926c09895b3d6de044d90eea5ea462b98b9fa2d54271e2f, bytes: 4220, store: repo}
rel:
  depends_on: [sot:root/network-capture, plan:root/20261107072531]
---

# Actual hosted capture component

Root launched the sole genuine hosted run42372114913 at full source
e77b9792b5c31db25e008c0b4a6f42ae7ac969da. Preflight128402198072 and
component128402635135 both completed successfully. Ubuntu24.04, Linux x86_64,
runner image20261101.360.1, Rust1.99.1. The owner followed that run without
duplicate/local execution or spoofing hosted environment. Public run:
https://github.com/modootoday/humanymous/actions/runs/42372114913

The existing native component invokes the five ignored hosted fixtures under a
180s command budget, one test thread. It verifies exactly one successful result
for each required name and one summary:5passed,0failed,0ignored,0filtered.
Command exit0; source_changed=false. Exact names, command, toolchain, source
manifest/lock hashes, test stdout/stderr hashes and raw-log hashes are attached.
Raw test stdout remains private to the runner; its exact-name validation is
reported by the source-bound Rust controller, not independently reconstructed
from an uploaded raw test log.

| Case | Actual measured component scope |
|---|---|
| actual_tls_h1_never_claims_h2 | H1 ALPN separation and idle/FIN/close-notify/buffered/pending/partial-record refusal |
| actual_h2_continuation_and_stream_binding | Real TLS/H2 continuation, ordered fields and stream association |
| actual_h2_wrong_stream_invalidates_all_capture | Wrong continuation stream terminal refusal |
| actual_invalid_tls_never_creates_an_owner | Invalid handshake never yields qualified owner |
| actual_supervisor_cancellation_revokes_the_connection_epoch | Pre-dispatch, in-handshake, pending-drop and post-handshake same-owner revocation |

All fixture worker handles are joined on success; pending-drop checks peer EOF
despite a surviving supervisor handle. Process exit0 and completed job were
observed. This is not a separate global orphan census or runtime/container
cleanup attestation. No durable-audit/origin-effect witness is present in these
component tests.

Capture Rust/tests/manifest match edf700bc. The recursive internal source closure
differs at signals/src/wire/mod.rs, so this actual result binds e77's full source,
not edf's historical dependency snapshot. The separate27pure/Clippy/no-run proof
remains evidence:root/20261107120400. This run does not execute those27lib units.

full_product_qualified=false remains explicit. Runtime serving, complete H2 body/
flow control, original-hop deployment and Docker product validation remain open.
The prior813 Docker failure is not converted into success by these five cases.
