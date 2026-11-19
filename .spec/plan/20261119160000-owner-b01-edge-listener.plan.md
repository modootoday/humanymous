---
id: plan:root/20261119160000
title: Owner B01 edge listener
status: active
created: 2026-11-19T16:00:00Z
updated: 2026-11-19T16:00:00Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/configuration.rs, crates/sdk/humanymous-runtime/src/gateway/configuration/exposure.rs, crates/sdk/humanymous-runtime/src/gateway/configuration/admin.rs, crates/sdk/humanymous-runtime/src/gateway/service.rs, crates/sdk/humanymous-runtime/src/gateway/service/admin.rs, crates/sdk/humanymous-runtime/src/gateway/service/lifecycle.rs, crates/sdk/humanymous-runtime/src/gateway/service/readiness.rs, crates/sdk/humanymous-runtime/src/gateway/application/tls.rs, crates/sdk/humanymous-runtime/src/gateway/admin_plane.rs, crates/sdk/humanymous-runtime/src/gateway/core_server/serve.rs, crates/sdk/humanymous-runtime/src/gateway/core_server/role.rs, crates/sdk/humanymous-runtime/src/gateway/main.rs, crates/sdk/humanymous-runtime/tests/gateway/exposure.rs]
  depends_on: [sot:root/rust-migration, sot:root/runtime-application]
---

# Owner B01 edge listener

## Goal

Carry out owner decision B01 #1 and the #153 readiness finding (lane G):
an explicit, fail-closed public TLS edge, a separate loopback admin TCP
listener composing `admin_plane` into `gateway::service`, a Core binary
role serving `core_server` with `PassMount`, and TLS readiness that reports
what is served. Base is `origin/codex/rust` ba926ddc; Go oracle 981d25c.

Detection freeze: no score, weight, threshold, hard-rule predicate, verdict
band or signal ID changes. Trust-header and spoof handling are unchanged. A11y
gating is untouched. The human-gated `gateway/control/` and `audit/` trees are
not edited.

## Steps

- [x] Exposure predicates: `gateway::configuration::exposure` owns
  `listen_permitted`, `peer_admitted`, `bound_permitted` and
  `admin_listen_permitted`. A non-loopback listen needs `public_listen` plus a
  provisioned or ACME `tls` source; defaults stay loopback; no public
  plaintext; the origin stays loopback; PROXY v2 is refused with a public
  listener. `Application::serve_tls_until` and `core_server::serve` admit a
  non-loopback peer only on a public listener; `serve_until` stays loopback.
- [x] Admin listener: the `admin` configuration object (loopback listen
  distinct from edge and origin, bearer operator table with Go weak-secret
  refusals, explicit ban store bounds, optional `settings_dir`) builds
  `gateway::service::admin::AdminService`, which shares its ban store and
  epoch with the served path, sweeps the settings overlay every minute and
  joins the service lifecycle (any listener failure stops the rest).
- [x] Core role: `humanymous-runtime core` reads a private `CoreRole`
  document, reuses the B02 certificate sources and ACME maintenance, serves
  `core_server` with `PassMount` over HTTP/1.1 and writes one readiness line.
- [x] Readiness (#153): `service::readiness::tls_listener` reports
  `original_h2` and `alpn` from the served ALPN offer; the capability gate is
  unchanged.
- [x] Tests on real sockets (non-loopback peers over the host interface
  address): library tests and the real-binary `gateway-exposure` target;
  qualification contract inventory updated.
- [x] SoT section "Public edge, admin listener and Core role" in
  `sot:root/runtime-application`; FM11 rows `cmd/gate/tls.go` and
  `cmd/server/acme.go` move to `ported_unit` with `go_test_map`; the other
  four rows stay `partial` with exact gaps.

## Remaining (not in this lane)

- Lane H: operator provisioning beyond the static bearer table (#2), gate
  default modes (#3), verdict-token minting (#4), HTML injection (#7).
- Non-loopback origin upstream and origin auth (B14); PROXY v2 trusted
  ranges for a public listener (B26); admin mTLS/off-host admin (not
  planned: the admin listener is loopback only).
- Core HTTP/2 and keep-alive; ML, Redis, playground and Core logging.
- Runtime proof of public :443 ACME issuance through the gateway ingress.
