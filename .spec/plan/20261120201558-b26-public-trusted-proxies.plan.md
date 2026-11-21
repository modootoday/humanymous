---
id: plan:root/20261120201558
title: B26 trusted balancers on the public listener
status: active
created: 2026-11-20T20:15:58Z
updated: 2026-11-21T00:40:00Z
rel:
  governs: [crates/sdk/humanymous-runtime/src/gateway/configuration.rs, crates/sdk/humanymous-runtime/src/gateway/configuration/exposure.rs, crates/sdk/humanymous-runtime/src/gateway/application/tls.rs, crates/sdk/humanymous-runtime/src/gateway/application/edge.rs, crates/sdk/humanymous-runtime/src/gateway/proxy_listener.rs, crates/sdk/humanymous-runtime/src/gateway/service/readiness.rs]
  depends_on: [sot:root/runtime-application, sot:root/ip-reputation, sot:root/network-capture, plan:root/20261119160000]
---

# B26 trusted balancers on the public listener

## Goal

Port Go `cmd/gate -trusted-proxies` (cmd/gate/main.go and
internal/gate/proxyproto.go at 981d25c, shipping-profile family B26) onto the
public TLS edge of owner decision B01 #1. The trust gate itself is already
ported and pinned against the Go golden
(`gateway::proxy_listener`, `network::proxy::{Allowlist, resolve, Peer}`,
`tests/fixtures/network/proxyproto-go-golden.json`); no composition root
uses it. This plan composes it and decides what the recovered address feeds.

Detection freeze: no score, weight, threshold, hard-rule predicate, verdict
band or signal ID changes. The recovered address only keys address-keyed
state (bans, rate history, the IP reputation lookup); it never becomes TLS
or original-client provenance (network-capture N01). HTTP forwarding headers
stay untrusted. A11y gating is untouched.

## Current state (base fb014c30)

- The loopback listener takes the client from a PROXY v2 header on every
  connection (`ip_reputation.client_address: proxy_v2`): the only peer is
  the trusted edge.
- The public listener takes no client address at all. `Configuration::
  validate` refuses `public_listen` with `ip_reputation` (a header from every
  peer would let any client forge its address), and `install_bans` refuses
  an enforcing ban gate without a client-address source, so a public edge can
  only run with `edge.ban: off` and without IP reputation.
- Go always keyed bans and rates by `RemoteAddr`: the socket peer, or the
  source a trusted balancer declared.

## Decisions

1. **Configuration.** New optional top-level `trusted_proxies`, an array of
   CIDR or bare-IP strings, parsed with `network::proxy::Allowlist` (Go
   `ParseCIDRs`: bare IPs as /32 or /128, a prefix shorter than /8 refused as
   too broad, at most 256 entries, empty refused). Allowed only with
   `public_listen: true`; on a loopback listener it is refused, because the
   loopback contract (every connection carries a header) is stricter and
   already configured through `ip_reputation`.
2. **Accept path.** The PROXY v2 header is read inside the per-connection
   receive task of `serve_public_tls`, before the TLS handshake, under the
   same `public_receive` permit and deadline (the earlier of Go's 5 s
   `READ_TIMEOUT` and the receive deadline). It is not read in the serialized
   accept loop, so a slow balancer holds one permit, not the listener. This
   differs from Go and from `ProxyListener::accept`, which read inside
   `Accept`; `gateway::proxy_listener::read_header` becomes the shared reader.
3. **Trust gate (Go parity, golden-pinned).**
   - A peer outside the allowlist: nothing is read; its bytes stay TLS.
   - A trusted peer with a valid PROXY header: the declared TCP source is the
     client.
   - A trusted peer with LOCAL, UNSPEC or a non-TCP family: Go kept the
     balancer address. Here the connection is served but **unattributed**
     (client `None`), as on the loopback listener: a balancer address is not
     a client, and keying bans on it would ban every client behind it.
   - A malformed, truncated or late header from a trusted peer closes that
     connection only.
4. **What the address feeds.** On the public listener the client is the
   socket peer for an untrusted peer (unspoofable TCP source, Go parity) and
   the declared source for a trusted balancer. It feeds exactly what the
   loopback edge's client feeds today: the edge ban gate and the
   `is_datacenter_ip` lookup. It does not feed scoring tells, the session
   binding or any TLS fact.
5. **Ban gate and IP reputation on a public listener.** With the public
   listener the transport always yields an attributable source, so
   `install_bans` accepts an enforcing ban gate there. `ip_reputation` with
   `public_listen` stays refused unless `trusted_proxies` is set: without a
   balancer allowlist every peer's address is its socket address, which is
   acceptable for bans but `ip_reputation.client_address: proxy_v2` would be
   a false statement. With `trusted_proxies`, `client_address` must be
   `proxy_v2` and means "PROXY v2 from the listed balancers only".
   Behaviour without `trusted_proxies` and without `ip_reputation` is
   unchanged except that the client is now the socket peer.
6. **Readiness.** The profile reports `trusted_proxies` as an entry count
   only (no networks), and `client_address` as `socket` or
   `socket_or_trusted_proxy_v2`.

## Critique reflected

- `edge_policy()` also gates other client-keyed paths
  (`application/gates.rs`). The implementation replaces the boolean
  "has an edge policy" with one client-address source per listener
  (loopback PROXY v2 edge, or public socket/trusted balancer), and every
  consumer is listed and tested; none silently starts or stops enforcing.
- Behind an L7 front (a CDN or TLS-terminating balancer) the socket peer is
  the front, so an enforcing ban gate would ban every client behind it. Go
  had the same exposure. The how-to says so, and readiness names the source
  so an operator can see it; the gateway cannot detect an L7 front.
- A trusted balancer can declare any source, including private or loopback
  addresses. Bans key on it as declared (Go parity); IP reputation already
  never classifies private, loopback, link-local or unspecified addresses.

## Open questions resolved by default

- Using the socket peer as client on a public listener without
  `trusted_proxies` (decision 4) is a behaviour change for existing public
  deployments: bans begin to key on it. Default: adopt it (Go parity, and a
  ban store is only present with the admin listener). If the owner rejects
  it, decision 4 narrows to trusted balancers only and the ban gate stays
  refused on a public listener without `trusted_proxies`.
- LOCAL/UNSPEC unattributed instead of Go's balancer address (decision 3) is
  a deliberate fail-closed difference, pinned by a test.

## Steps

- [x] `configuration`: `trusted_proxies`, validation (public only, Allowlist
  parse, `ip_reputation` coupling of decision 5), unit tests.
- [x] `application/tls.rs`: header read in the receive task; `Received`
  carries the client; `serve_received` sets `self.client`; tests for the four
  trust-gate outcomes over real sockets, including a slow trusted balancer
  that does not block a second client.
- [x] `application/edge.rs`: `install_bans` accepts the public listener;
  test that an enforcing gate bans the declared source, not the balancer.
- [x] Readiness facts and test.
- [x] SoT: runtime-application (public listener client address) and
  ip-reputation (trusted client address on a public listener); ledger rows
  internal/gate/proxyproto.go and cmd/gate/main.go (`-trusted-proxies`).
- [x] Qualification contract, docs-review hashes; scoped `cargo test`,
  Clippy and fmt; docs how-to for running behind an L4 balancer.

## Record (2026-11-21)

- One client-address predicate (`Application::attributes_clients`: the
  loopback PROXY v2 edge or a public listener) replaces "has an edge
  policy" for `install_bans` and `install_webauthn`; the loopback accept
  paths still use `edge_policy` unchanged.
- LOCAL from a listed balancer under an enforcing ban gate answers 503
  `ban_client_address` (the existing unattributed rule), pinned by
  `unix_trusted_balancer_declares_the_client`.
- Tests: `gateway::tests::service::trusted_proxies` (socket peer keyed,
  spoofed header from an unlisted peer ignored, declared source keyed and
  the balancer not, LOCAL unattributed, missing header closed, a silent
  listed peer not blocking a second client, configuration refusals). A
  mutation that keys the balancer instead of the declared source fails them.

## Verification

Source-bound unit and socket tests in `humanymous-runtime`. A product-lab
case (balancer container in front of `tgate` sending PROXY v2) is a separate
Docker gate and is not claimed by this plan.

## Not done here

- PROXY v1 (text) headers: Go accepted only v2.
- Per-balancer TLS or authentication of the balancer: Go had none; the
  allowlist is the trust boundary.
