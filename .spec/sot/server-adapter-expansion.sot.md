---
id: sot:root/server-adapter-expansion
title: Rust server and framework adapter expansion
status: active
created: 2026-11-06T23:46:21Z
updated: 2026-11-06T23:55:20Z
verify: manual
rel:
  depends_on: [sot:root/adapter-ecosystem, sot:root/adapter-service-contract, sot:root/adapter-authority, sot:root/nginx-adapter]
---

# Server adapter expansion

Version 0.1.0, researched 2026-11-07 KST by three read-only agents. The user
clarified that additional adapters mean Node/nginx-like server and framework
integrations. Browser CDP/CEF observers are a separate laboratory tool concern.
New integrations below are researched/proposed, not implemented or qualified.

## Rust ownership and current baseline

Keep scoring, crypto, policy, replay, challenge, state and audit in one Rust
authority. Prefer Rust Gate plus existing server configuration for language-
independent HTTP origins. No copied detector per language/framework. Under the
latest Rust-authored-code requirement, new Lua/njs/Go/C#/Python middleware is not
silently selected. Existing JS/njs prototypes remain historical experimental
work until replaced or explicitly reconciled. A native Rust binding's generated
loader and a user's own registration code require an explicit artifact/source
classification; an addon is not automatically zero foreign-language integration.

Existing Express/Hono Node/Bun adapters have mock-conformance tests and subsequent
limited real Rust-authority Docker evidence. Neither establishes whole product
protection or releaseQualifiedTuples. Existing nginx is njs js_content with
humanymous.nginx-pilot/0.1, 64 KiB complete-body limits and mock authority only;
it is not a Rust native module or compatible with the Node service by renaming it.

## Prioritized ecosystem matrix

| Priority | Integration family | Rust-authored path and qualification boundary |
| --- | --- | --- |
| P0 | HTTP origins across languages | Rust Gate before origin, configured proxy, origin bypass denial and authenticated upstream; topology determines original capture |
| P0 | Existing Node/Bun Express/Hono | Complete actual runtime integration; preserve Node HTTP1 versus Fetch fidelity and replace/reconcile authored JS bridge |
| P1 | nginx native | Official nginx/ngx-rust dynamic-module candidate, version/build ABI and worker-lifetime/FFI gates [S1] |
| P1 | Envoy | Rust ext_authz v3 bridge; full body versus partial, final-route binding, strict failure mapping [S2] |
| P1 | HAProxy | Rust SPOA bridge with explicit deny ACL on missing/error/expired state; agent failure is not automatic deny [S3] |
| P1 | Rust Tower/Axum/Hyper/Tonic and Actix | Shared Rust integration, readiness/backpressure, ownership/cancellation/upgrade; Tonic streaming requires new contract [L1] |
| P1 | NodeHTTP1 Koa/Nest Express/Node adapters | Common request bridge, lifecycle-specific registration; Rust Node-API feasibility pilot [N1-N4] |
| P1 | Fastify/Nest Fastify | One parser-before-body lifecycle bridge, encapsulation/order and single completion [N5] |
| P1 | Fetch Hono/Deno.serve/Bun.serve | Shared normalized profile, runtime-specific IPC/cancellation and stream qualification [N6] |
| P2 | Caddy / Traefik | Rust Gate or limited external-auth bridge; default auth precheck is not exact-body protection [S4, S5] |
| P2 | Apache HTTPD / IIS-ARR / YARP | Proxy configuration to Rust Gate; preserve identity/termination semantics; native foreign-language plugins not default [S6, S7] |
| P2 | Python ASGI FastAPI/Starlette/Django | Rust Gate first; native middleware is a separate language-binding decision, not shipped Rust-only support [L2] |
| P2 | WSGI Flask/Django | Separate sync stream/iterable profile; do not claim ASGI/WebSocket parity [L3] |
| P2 | JVM Servlet Spring MVC/Tomcat/Jetty | Dispatch/async/read-body lifecycle; native Java interface binding separate from HTTP coverage [L4] |
| P2 | Spring WebFlux/Reactor Netty | Nonblocking IPC and pooled-buffer release/cancel; not a Servlet wrapper [L5] |
| P2 | ASP.NET Core MVC/Minimal/Razor | Windows/Linux HTTP-origin coverage; native delegate requires binding decision, bounded rewind/abort tests [L6] |
| P2 | PHP PSR7/15, Laravel/Symfony/CMS | Rust Gate -> nginx/Apache -> FPM; direct FastCGI is a different transport [L7] |
| P2 | Ruby Rack/Rails and Elixir Plug/Phoenix | HTTP-origin first; close/hijack, partial body consumption and upgrade differ [L8] |
| P3 | Go net/http/chi/Gin/Echo/Fiber, SwiftNIO/Vapor, Dart Shelf and new servers | Register exact HTTP-origin tuples first; no new Go SDK in the final Rust tree |

The list is open-ended. Admit new targets by actual lifecycle/transport contract
and evidence, not framework popularity or a fixed number of adapters.

## Contract changes required before implementation

Current adapter/0.1 accepts only node-raw and fetch-normalized headerMode, and
capture/stream are null. ASGI ordered duplicate header values do not guarantee
original casing; WSGI/Servlet/Rack/PSR/Go maps have other losses. Never claim
node-raw by relabelling a normalized framework view. Fetch mode is not a universal
foreign-language escape hatch. Design a versioned typed-provenance delta first.

Required shared properties:

- Authenticated adapter ID/version, installation/tenant/origin/revision, actual
  transport peer and credential epoch. Headers never grant capture authority.
- Exact method/target/authority/route and body representation/completeness/digest;
  casing/order/duplicates are independent observed capabilities.
- Atomic nonce reservation, deadline, scope and durable audit before origin.
  Reject post-ALLOW rewrite, replay, automatic retry/cache/failover reuse.
- Independent profiles for header precheck, bounded complete request, streaming,
  upgrade, capture, collector and response transformation; unsupported is explicit.
- Timeout/down/malformed/mismatch/body loss/audit failure => no protected origin
  effect. CHALLENGE needs a real recovery flow, not arbitrary HTTP status mapping.
- Preserve Set-Cookie duplicates, HEAD/204/304, CORS/CSP, compression/ETag/length,
  streaming flush, cancellation/backpressure and once-only handler invocation.
- UDS/Windows pipe identity per runtime/OS. Remote auth service requires a newly
  designed authenticated workload transport; no silent TCP fallback or public admin.

## Mechanism-specific findings

nginx/ngx-rust is an official Rust module candidate with unstable APIs and version-
specific build requirements. --with-compat is not unlimited ABI compatibility.
Avoid blocking work in nginx workers; qualify request-pool lifetimes, panic/FFI,
reload and shutdown. nginx Windows upstream limitations prevent assuming equal
production support. Native module presence does not supply raw TLS/H2 capture. [S1]

nginx auth_request examples omit body and map only specified status classes.
Caddy forward_auth normally uses a GET precheck. Traefik forwardBody interrupts
streaming. Envoy partial messages cannot qualify a complete-body policy; authorize
after final routing or revalidate later mutation. HAProxy needs explicit ACL
handling of agent processing errors. None is automatically full Humanymous. [S2-S5, S8]

NodeAPI is a candidate for Rust-authored native bindings, not a duplicate authority.
napi-rs v3 starts at Rust1.88, but locked target closure and async host-object/
stream lifetime need real tests. Bun describes broad NodeAPI support, not perfect
compatibility. Deno native FFI escapes its language sandbox. A wasm codec cannot
replace OS peer authentication or durable state. [N7]

Fastify's onRequest has no body; preParsing sees the pre-parser stream and replacement
must satisfy encoded-length/body-limit rules. Koa/Nest registration, proxy trust,
async completion and runtime-specific Fetch support need explicit conformance.
Never hash a parsed JSON/multipart reconstruction as original entity bytes. [N1-N6]

## Conformance and implementation order

1. Complete Rust Gate/shared session/capture/collector and durable key/state ownership.
2. Accept versioned ingress/provenance/body/stream contracts and shared synthetic vectors.
3. Complete current Node real integration and implement Rust nginx/standard auth bridges.
4. Extend HTTP-origin deployment recipes and only qualified native binding profiles.
5. Run native module install/reload/peer pilots plus Docker product e2e and AT controls.

Shared negative corpus: wrong peer/scope/tenant/body/target, stale/replay/restart,
authority down/timeout, observer loss, unsupported stream, route rewrite and
audit failure => independent protected origin counter zero. Valid authorized
machine APIs remain valid without fabricated browser telemetry. Same exact source
and configuration/engine hashes bind results. All new target cases are not_run;
scoring freeze, accessibility and T4 limits remain unchanged.

## Sources

- S1 https://github.com/nginx/ngx-rust ; https://nginx.org/en/docs/configure.html ; https://nginx.org/en/docs/windows.html
- S2 https://www.envoyproxy.io/docs/envoy/v1.40.0/api-v3/extensions/filters/http/ext_authz/v3/ext_authz.proto ; https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/ext_authz_filter
- S3 https://github.com/haproxy/haproxy/blob/master/doc/SPOE.txt
- S4 https://caddyserver.com/docs/caddyfile/directives/forward_auth
- S5 https://doc.traefik.io/traefik/middlewares/http/forwardauth/
- S6 https://httpd.apache.org/docs/2.4/en/mod/mod_proxy.html
- S7 https://learn.microsoft.com/en-us/aspnet/core/fundamentals/servers/yarp/authn-authz?view=aspnetcore-10.0
- S8 https://nginx.org/en/docs/http/ngx_http_auth_request_module.html
- N1 https://nodejs.org/api/http.html ; https://nodejs.org/api/http2.html
- N2 https://expressjs.com/en/guide/behind-proxies.html
- N3 https://koajs.com/
- N4 https://docs.nestjs.com/techniques/performance
- N5 https://fastify.dev/docs/latest/Reference/Hooks/
- N6 https://hono.dev/docs/getting-started/nodejs ; https://hono.dev/docs/helpers/streaming ; https://docs.deno.com/runtime/fundamentals/http_server/ ; https://bun.sh/docs/runtime/http/server
- N7 https://nodejs.org/api/n-api.html ; https://napi.rs/docs/introduction/getting-started ; https://bun.sh/docs/runtime/node-api ; https://docs.deno.com/runtime/fundamentals/ffi/
- L1 https://tower-rs.github.io/tower/tower_service/trait.Service.html
- L2 https://asgi.readthedocs.io/en/latest/specs/www.html ; https://www.starlette.io/middleware/
- L3 https://peps.python.org/pep-3333/
- L4 https://jakarta.ee/specifications/servlet/6.1/
- L5 https://docs.spring.io/spring-framework/reference/core/databuffer-codec.html
- L6 https://learn.microsoft.com/en-us/aspnet/core/fundamentals/middleware/request-response?view=aspnetcore-10.0
- L7 https://www.php-fig.org/psr/psr-7/ ; https://www.php-fig.org/psr/psr-15/ ; https://nginx.org/en/docs/http/ngx_http_fastcgi_module.html
- L8 https://github.com/rack/rack/blob/main/SPEC.rdoc ; https://plug.hexdocs.pm/Plug.Conn.html
