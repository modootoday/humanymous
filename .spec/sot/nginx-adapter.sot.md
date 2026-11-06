---
id: sot:root/nginx-adapter
title: Experimental nginx bounded request adapter
status: active
created: 2026-11-07T00:40:00Z
updated: 2026-11-07T00:40:00Z
verify: manual
rel:
  governs: [adapters/nginx/**]
  depends_on: [sot:root/adapter-service-contract]
---

# Experimental nginx bounded request adapter

## Rule

Implement an njs js_content gate followed by a named internal proxy location.
No C/FFI or scoring. This is an experimental Linux nginx compatibility slice,
not whole humanymous protection. Shared adapter/0.1 currently permits Node only;
use distinct `humanymous.nginx-pilot/0.1`, never impersonate Node. Integration
requires authority-owner approval/schema extension, authenticated peer/ACL,
replay/audit and required-signal enforcement. Mock ALLOW is not Rust validation.

Bound complete transfer-decoded content-encoded body at 64 KiB; unavailable
body fails closed. Only GET/POST on exact /echo, /deny, /challenge, /bad,
/timeout, /cookies, /stream pilot inventory. Unknown and reserved /_humanymous
routes reject. Reject upgrades, CONNECT, trailers, transfer-encoding, duplicate
headers, Content-Encoding uploads, non-ASCII header values, unsafe/ambiguous target and mismatched Host. Fixed configured authority
is example.test. No rewrites after decision; proxy_pass without URI retains
original request target and body. No request retries or decision cache.

Use an internal IPC-only location, fixed Unix socket, fixed decision path,
credential injected from protected nginx configuration (fixture credential only
in pilot). No TCP authority fallback. No management methods or public admin.
Production socket ownership/mode, credential provisioning and authority peer
verification are open blockers. No client header can establish local identity.

Forward only explicit content-type, authorization, cookie, accept and user-agent
headers plus configured Host and generated framing. Decision binds this exact
header projection, method, untouched target, authority, body length/base64/SHA256,
nonce, deadline and configured tenant/origin/revision. Other headers including
forwarded/x-forwarded and x-humanymous are discarded on both legs. Projection is
an explicit capability loss: arbitrary request header passthrough is unsupported.
Raw TLS, original H2/H3 and original client provenance are unavailable.

Match exact bounded response fields and request identity, expiry and verdict.
Non-200, timeout, malformed/mismatched response or body error => 503 no-store.
DENY =>403; CHALLENGE=>409 with no origin contact. Recovery endpoints are absent.
Only ALLOW redirects once to a private named origin location. The subrequest
response cannot mutate origin headers. Origin responses use standard nginx
proxy semantics; preserve repeated Set-Cookie, no HTML/CSP transformations,
proxy buffering off for response streams. Upgrade/gRPC/uploads streaming are
unsupported; SSE response support is limited to normal proxy behavior.

## Why

Official njs requestBuffer/rawHeadersIn/subrequest/internalRedirect APIs and
nginx proxy_pass_request_headers permit a small auditable boundary. Bounded
js_content avoids pretending an auth_request status alone binds an unread body.
Sources consulted 2026-11-07: https://nginx.org/en/docs/njs/reference.html,
https://nginx.org/en/docs/http/ngx_http_js_module.html,
https://nginx.org/en/docs/http/ngx_http_proxy_module.html.

## Critique reflected

Blue: authority alone owns verdict; no duplicated detector. Red: public direct
origin location, spoofed peer, body/target reuse and response mismatch must fail.
SRE: finite body/header/subrequest/time limits and no proxy retry. DPO: no access
log or envelope persistence, fixture token only. Evaluator: pilot protocol cannot
claim shared-contract or Rust compatibility; real browser/a11y flow remains open.
P0 production blockers: real authority integration, peer ACL/replay/audit and
required capture signals. P1: runtime matrix, disconnect/load/resource tests,
response streaming cancellation/backpressure, HTTP2 framing and browser recovery.

## Verification

Docker internal network only; real nginx njs plus mock Unix authority and origin.
Unit boundary cases and negative network cases must check zero origin effects.
No Go edits; full product Docker gate remains unrun by this scoped pilot.

