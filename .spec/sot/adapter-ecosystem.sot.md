---
id: sot:root/adapter-ecosystem
title: Cross-platform adapter ecosystem
status: active
created: 2026-11-06T21:21:09Z
updated: 2026-11-06T21:27:12Z
verify: manual
rel:
  depends_on: [sot:root/rust-migration]
---

# Cross-platform adapter ecosystem

## Rule

Document version: 0.1.0. This document fixes the adapter architecture and
acceptance contract; it is not an assertion that listed adapters are implemented.
The implementation state must remain explicit per adapter, OS and capability.

### Brief and invariants

Support adoption across operating systems, deployment platforms and application
languages without cloning the security engine into each framework. Survey real
hosting archetypes without an arbitrary target-count limit. Group implementations
by execution and protocol contracts rather than one package per framework name.
Keep scoring frozen and share Rust authority for policy, audit, challenges,
integrity, settings and enforcement. Preserve existing application semantics.
Management is local-only and never part of the public adapter routing surface.
Make missing observations explicit; downstream middleware cannot recreate the
client's original TLS connection after another service terminates it. Every
claimed combination requires conformance and deployment evidence. Defense-only
wargames target the isolated project lab and include human controls.

### Architecture decisions

1. A shared Rust data-plane authority owns verdicts and enforcement state.
   Language adapters translate framework lifecycle events and invoke the shared
   contract; they do not recalculate scoring, reimplement token crypto or own
   divergent hard-rule tables. Stateless framing/transport helpers may exist in
   each language when required by its ABI.
2. The Rust Gate reverse-proxy mode is the universal integration baseline for
   HTTP origins. Native middleware is an ergonomic companion for application
   context, explicit browser integration and local deployment orchestration.
   A framework that lacks a dedicated package can use a certified proxy topology.
3. Native ingress authorization protocols, nginx plugins, in-process libraries,
   local sidecars and managed-edge hooks are distinct integration modes. Their
   capability manifests must report the actual observations and mutation hooks
   available at that location. An authorization subrequest alone is not a full
   response-transforming proxy or a complete browser challenge integration.
4. Automatic activation means validated provisioning of the supported topology,
   shared security services and reserved routes. It never means silently guessing
   network facts, opening management remotely, rewriting arbitrary content or
   making missing required evidence optional. Unsupported combinations fail
   installation/route readiness with an actionable diagnostic.
5. Multi-process/native bindings must define ownership, concurrency, lifetime,
   error/timeout and upgrade compatibility before implementation. Prefer the
   local versioned service protocol when it removes unsafe duplicated bindings.
   A native module may use a narrow ABI shim, with security logic remaining Rust.

### Capability and trust contract

Each adapter ships a machine-readable descriptor with a stable adapter ID,
package/version, supported host/runtime versions, OS/architecture/libc tuples,
integration mode, contract/schema version and capability states. Capability
states are available, unavailable or conditional with explicit prerequisites;
unknown is not available. Store observed runtime capability evidence separately
from static package declarations. Bind a decision to configuration revision and
authenticated capture provenance, not an untrusted request header.

Capability groups include raw client TLS; original H2 settings/order; H1 raw
order/casing; H3/QUIC observation; authoritative peer identity; exact request body
bytes; browser collector/challenge serving; response transformation; WebSocket,
SSE, gRPC, streaming and trailers; audit durability; local management; and
connection/session correlation. None is inferred from a framework name alone.
The existing source implements TCP TLS/H2 observations, not H3/QUIC capture;
an H3 listener is unqualified until separately implemented and piloted.

| Deployment profile | Evidence boundary | Qualification rule |
| --- | --- | --- |
| Direct Rust TLS edge | Gate receives the original client TLS and raw HTTP connection | Full applicable existing L1-L7 capability requires browser integration plus verified capture/audit/enforcement wiring |
| Upstream-terminated HTTP origin | CDN/LB/proxy ends client TLS before Gate or middleware | Original TLS/H2 unavailable unless a separately authenticated supported capture channel supplies it; never classify proxy TLS as client TLS |
| Managed edge/serverless hook | Host supplies a constrained request/runtime API | Publish exact provider observations and execution limits; unsupported raw capture/body/stream hooks remain unavailable |
| Native library/module | Host ABI controls lifecycle and request access | Qualify worker/thread/reload behavior and the same capability contract; a native label alone gives no network fidelity |

Host and route readiness must check required capabilities before admitting
protected traffic. Missing required inputs fail closed without changing frozen
weights or inventing a new score. Operators may explicitly choose a documented
limited-capability deployment; its weaker observations remain visible in doctor,
audit and public support claims. There is no universal all-platform full-fidelity
claim and no silently downgraded strict route.

### Management and origin isolation

- Public data-plane listeners never register, proxy or alias management handlers.
  Collector, challenge and receipt endpoints are data-plane control routes, not
  administrative APIs. Use one shared route registry with explicit method/schema
  and collision checks rather than framework-specific ad hoc suffixes.
- Management binds loopback or permission-restricted local IPC only. Verify the
  actual transport peer, not Host, X-Forwarded-For, Forwarded, CF-Connecting-IP,
  PROXY headers from untrusted peers or an adapter-supplied local boolean.
- Locality is necessary but insufficient: require administration credentials,
  authorization, CSRF/Origin protections where relevant, rate limits and existing
  dual control for erasure, kill switch and permanent bans. Defend against browser
  localhost attacks, SSRF, DNS rebinding and other untrusted local tenants.
- Scope Unix socket permissions and Windows named-pipe ACLs to the service
  identity; container loopback/pod network sharing does not establish user trust.
  Never publish the admin port through containers, ingress, Services or proxies.
- Windows named pipes explicitly reject remote clients and apply a restrictive
  service/logon-SID DACL, including network-access rejection where applicable;
  default pipe ACLs and a local-looking pipe name are insufficient. Linux
  abstract sockets lack pathname permission protection and require an explicit
  authenticated peer-credential design. Test actual unauthorized peer access.
- Administrative and adapter decision/capture listeners use separate endpoints
  and credentials; possession of an adapter credential cannot administer a node.
- Origins must reject direct bypass of the enforcement path with verified network
  or service identity controls. Externally supplied internal decision/capture
  headers are stripped/rejected, and only authenticated configured peers may
  assert bound facts. Preserve legitimate application authorization/cookies.

### Application compatibility contract

Reserve a configurable explicitly reviewed namespace, validate collisions before
startup and never shadow an application's existing route. Preserve request
method, target/query semantics, duplicate header meaning, Host authority, body
bytes, cookies, response status and redirect behavior. Validate ambiguous framing
and hop-by-hop/forwarding headers at each protocol boundary. Proxy trust is an
operator-configured topology, never trust-all by default.

Do not consume a one-shot body twice or buffer unbounded uploads. Verify RIT over
the specified exact byte representation and enforce size/time limits without
silently replacing bodies. Content compression, trailers, multipart, chunking,
retries and signature-bearing requests require explicit tested semantics.
Do not retry non-idempotent origin requests after partial forwarding.

A route requiring complete-body integrity validates all required evidence before
forwarding any body bytes or invoking an origin handler that can cause side
effects. Bounded buffering/spooling has explicit memory/disk/time limits and
cleanup. Unbounded streaming, WebSocket and gRPC streams require a separately
specified preauthorization/stream-integrity contract. An unsupported contract
blocks route qualification rather than silently omitting integrity.

### Versioned evidence and decision envelopes

The wire contract is separate from frozen RIT formats. Do not silently replace
RIT canonicalization or token domains with a new message-signature scheme.
Every envelope binds schema/protocol version, adapter identity/version,
installation/tenant/origin, configuration digest, authenticated transport peer,
connection ID and request/stream ID, method, original target/authority, relevant
header/body digests, freshness/expiry and replay-resistant correlation value.
Reject mismatches, unsupported required fields, oversized/expired/duplicate
envelopes and cross-tenant/upstream replay. Multiplexed connection facts cannot
become per-request evidence, and reconnect/reload cannot transfer authority.

The evidence model separates actual peer from asserted client IP; original
external authority from upstream authority; raw target from routing-normalized
path; ordered duplicate headers from normalized maps; and observed-client TLS
from observed-upstream TLS/authenticated forwarded facts/not-observed. Body
evidence declares exact encoding layer, length, digest, complete/partial status,
capture limit and rejection reason. Parsed JSON serialization is not raw body.
Include deadline/cancellation and protocol/upgrade intent without logging raw
body, credentials or personal identifiers unnecessarily.

Decisions bind that same request and final route, distinguish transport/input
errors from ALLOW/CHALLENGE/DENY, and carry only explicitly allowed response/header
mutations and challenge references. They contain no admin credential, operation
or unrestricted debug trace. Cached decisions remain session/route/body/policy
bound; retries must not duplicate counters or authorize another request.
Default transport is authenticated local IPC. Managed-edge remote data channels
require explicit authenticated deployment, tenant isolation, replay protection
and rate limits; they never expose management. If a host cannot support those
requirements, that connector is unqualified rather than implicitly trusted.

Protocol sources: [HTTP Message Signatures](https://www.rfc-editor.org/rfc/rfc9421.html),
[HTTP digests](https://www.rfc-editor.org/rfc/rfc9530.html),
[Forwarded](https://www.rfc-editor.org/rfc/rfc7239.html).
These inform the new transport threat model, not a mandate to rewrite legacy RIT.

Browser insertion is negotiated and scoped to compatible eligible HTML. Respect
CSP, nonces/hashes, cache variants, compression, integrity headers, ETags,
Content-Length, 204/304/HEAD and immutable/signed responses. Explicit template or
framework hooks are preferred when automatic transformation cannot preserve the
contract. Never inject HTML into API, gRPC, SSE, WebSocket or arbitrary downloads.
API clients receive a documented machine-readable outcome; they cannot complete
a browser-only challenge implicitly. Preserve accessible Pass recovery and the
intentional coherent-browser T4 ceiling.

### Conformance and promotion gates

All integration modes run the same contract corpus where applicable, plus host
lifecycle-specific tests. Every case declares applicable capabilities; required
unavailable cases block promotion, and skips remain visible in the denominator.

Required suites cover:

- Header case/order provenance, duplicates, spoofed peer/local/internal headers,
  authority mismatch, framing ambiguity, forwarding trust and origin bypass.
- Reserved-route collision, encoded/normalized path disagreements, mount/base
  prefixes, methods, CORS preflight, cookies, redirects, caching and session scope.
- Exact bodies and RIT, streaming upload/download, compression, limits,
  cancellation, deadlines, backpressure, trailer preservation, disconnects and
  non-idempotent retry prohibition.
- HTTP/1.1, supported H2/gRPC and WebSocket/SSE lifecycles; H3 only when qualified.
  Auth subrequests must prove which bytes and lifecycle phases are not available.
- CSP-aware HTML integration and explicit-template mode, challenge recovery,
  no-script/API outcomes and keyboard-only/reduced-motion human controls.
- Local-only management under direct/proxied/container deployments, CSRF/SSRF/
  rebinding attempts, unauthorized local process identity and dual-control cases.
- Restart, reload, multi-worker behavior, policy revision changes, key continuity,
  audit sink outage, resource exhaustion, sidecar failure and unsupported versions.
- Native Windows/macOS/Linux pilots and isolated Linux Docker deployment/e2e,
  including the actual framework/proxy/runtime versions advertised as supported.

No target becomes supported from documentation alone. Track states separately:
researched, specified, implemented, conformance-passed, deployment-piloted and
released. A release manifest names exact tested tuples and capability profiles.
Do not claim the entire Cartesian product of surveyed OS/platform/language names.

### CLI, packages and delivery

Doctor discovers actual ingress/TLS termination, runtime/ABI, OS/architecture,
libc/service manager, admin bindings, route conflicts, trusted peers and browser
integration readiness. Install --dry-run produces an explicit plan and capability
diff without changing files, services, privileges or network configuration.
Real install is repeatable, verifies generated configuration with the target
server's checker, preserves operator files and provides rollback/error evidence.

Package by reusable ABI/contract family. Node/Bun Express/Hono support and nginx
plugin integration remain mandatory named deliverables from the migration goal.
Other dedicated SDKs/modules are selected by the ecosystem matrix below; universal
Gate integration does not require inventing a separate security engine per
language. The final implementation plan must list every required artifact and
its qualification gate, not hide work under an unlimited future-adapter promise.
npm and crates.io releases use the scoped OIDC delivery contract in
[[sot:root/delivery]], with same-commit conformance, docs freshness and packaging
checks. Provider registry bootstrap prerequisites remain explicit.

## Ecosystem matrix

This is an open-ended qualification catalog, not a numerical cap. A new target
must map to an existing contract family or justify a new lifecycle/transport
family with tests. Dedicated security implementations per framework are refused.
All entries below are researched/specification scope, not implemented support.

### Language and framework families

| Family ID | Adopter archetypes | Binding delivery decision and qualification boundary |
| --- | --- | --- |
| lang.node-stream | Node HTTP/HTTPS, Express/Connect, Koa, Fastify, Nest | One Node stream transport core; Express wrapper, Koa middleware and Fastify lifecycle plugin reuse it; Nest integrations qualify actual Express/Fastify host |
| lang.fetch | Hono, Bun.serve, Deno.serve, Web Request/Response servers | One Fetch contract core plus runtime-specific authoritative peer/upgrade bindings; Node/Bun Hono are mandatory pilots; Deno adds native runtime pilot |
| lang.js-ssr | Next, Nuxt/Nitro, SvelteKit, other SSR/SPA hosts | Gate deployment profiles plus explicit loader/template hooks; framework hooks do not imply full process-wide network protection |
| lang.python-asgi | FastAPI/Starlette, Django ASGI, Quart, compatible Sanic servers | One ASGI adapter for HTTP/WebSocket/lifespan scopes; framework registration recipes and genuine receive/send cancellation tests |
| lang.python-wsgi | Flask, Django WSGI, Pyramid, Bottle, Falcon WSGI | One WSGI adapter with iterable close/start_response/body handling; no WSGI WebSocket claim |
| lang.jvm-servlet | Java/Kotlin/Groovy, Spring MVC, Tomcat/Jetty/Jakarta | Servlet filter family, explicit REQUEST/ASYNC/ERROR/FORWARD evaluation and async-dispatch ownership |
| lang.jvm-reactive | Spring WebFlux, Reactor Netty | Separate nonblocking reactive integration; no blocking local RPC or unbounded DataBuffer materialization |
| lang.jvm-pipeline | Ktor, Play/Pekko, Clojure Ring | Ktor plugin and Play/Ring lifecycle profiles reuse the JVM decision client; map vs stream and sync/async distinctions qualify separately |
| lang.dotnet | ASP.NET Core MVC/Minimal API/Razor, Blazor Server, C#/F#/VB | ASP.NET Core middleware and hosting integration; response-start, cancellation and forwarded-header trust tests; standalone WASM browser apps are not servers |
| lang.php-psr | PSR-7/15, Slim, Mezzio | PSR-15 middleware plus compatible message factories; stream cursor/seekability and exception ordering tests |
| lang.php-framework | Laravel, Symfony, Drupal/WordPress and FPM hosting | Shared PHP contract client, framework registration hooks and Gate/FPM server recipes; CMS hooks alone cannot cover static files or requests handled before CMS |
| lang.ruby-rack | Rack, Rails, Sinatra, Hanami | Rack middleware/gem with Rack::Lint, close/stream/hijack and host version qualification |
| lang.go-http | External user Go net/http, chi, Gin, Echo, Fiber/fasthttp | Certified universal Gate profile; no new Go SDK in the final source tree. Pilots use disposable external fixtures so the user's Go-code cleanup requirement remains intact |
| lang.rust-service | Tower/Axum/Tonic, Hyper and Actix | Shared Rust embedding contract, Tower Layer/Service and Actix Transform; readiness, backpressure and error/body ownership tested |
| lang.beam | Elixir Plug/Phoenix/Bandit/Cowboy, Erlang inets | Certified universal Gate profile now in scope; Plug/native integration candidate requires separate upgrade/lifecycle qualification |
| lang.swift | Vapor/SwiftNIO | Gate profile; native AsyncMiddleware candidate must preserve event-loop nonblocking behavior |
| lang.dart | Shelf, Dart HttpServer | Gate profile; Shelf integration candidate must retain one-shot body streams |
| lang.crystal | HTTP::Handler, Kemal | Gate profile; Handler candidate follows lifecycle-specific tests |
| lang.perl | PSGI/Plack/Catalyst/Dancer | Gate profile; PSGI candidate distinguishes immediate/delayed/streaming responses |
| lang.native-other | C/C++ Beast/Drogon/Crow, Haskell WAI, OCaml, Zig, Nim, R/Julia servers, legacy CGI/FastCGI | Universal HTTP Gate profile; native ABI/bindings only after concrete lifecycle need and qualification; CGI/FastCGI stay behind an actual HTTP gateway |
| lang.static-client | Static sites, object storage origins, SPA/mobile/desktop/IoT API consumers | Gate protects an HTTP origin; browser SDK only where executable browser integration exists. Nonbrowser API clients do not acquire interactive-human semantics |

Official contract sources: [Node HTTP](https://nodejs.org/api/http.html),
[Express proxy trust](https://expressjs.com/en/guide/behind-proxies/),
[Fastify hooks](https://fastify.dev/docs/latest/Reference/Hooks/),
[Koa](https://koajs.com/), [Bun server](https://bun.sh/docs/runtime/http/server),
[Deno HTTP](https://docs.deno.com/api/deno/http-server/),
[Hono WebSocket](https://hono.dev/docs/helpers/websocket),
[Hono v5](https://hono.dev/docs/guides/migrating-to-v5),
[Next proxy convention](https://nextjs.org/docs/app/api-reference/file-conventions/proxy),
[ASGI](https://github.com/django/asgiref/blob/main/specs/www.rst),
[WSGI](https://peps.python.org/pep-3333/),
[Django middleware](https://docs.djangoproject.com/en/6.0/topics/http/middleware/),
[FastAPI middleware](https://fastapi.tiangolo.com/advanced/middleware/),
[Servlet filters](https://docs.spring.io/spring/reference/7.1/web/webmvc/filters.html),
[Spring reactive](https://docs.spring.io/spring-boot/reference/web/reactive.html),
[Ktor plugins](https://ktor.io/docs/server-custom-plugins.html),
[Play filters](https://www.playframework.com/documentation/3.0.x/ScalaHttpFilters),
[Ring concepts](https://github.com/ring-clojure/ring/wiki/Concepts),
[ASP.NET Core middleware](https://learn.microsoft.com/en-us/aspnet/core/fundamentals/middleware/?view=aspnetcore-10.0),
[PSR-15](https://www.php-fig.org/psr/psr-15/),
[Laravel middleware](https://laravel.com/framework/docs/12.x/middleware),
[WordPress parse_request scope](https://developer.wordpress.org/reference/hooks/parse_request/),
[PHP FPM](https://www.php.net/manual/en/install.fpm.php),
[Rack](https://rack.github.io/rack/3.2/SPEC_rdoc.html),
[Go net/http](https://pkg.go.dev/net/http),
[Tower](https://docs.rs/tower/latest/tower/trait.Layer.html),
[Axum](https://docs.rs/axum/latest/axum/middleware/index.html),
[Actix](https://actix.rs/docs/middleware),
[Plug](https://plug.hexdocs.pm/readme.html),
[Erlang inets](https://www.erlang.org/doc/apps/inets/http_server.html),
[Vapor](https://docs.vapor.codes/advanced/middleware/),
[Shelf](https://pub.dev/packages/shelf),
[Dart HTTP](https://api.dart.dev/dart-io/HttpServer-class.html),
[Crystal Handler](https://crystal-lang.org/api/latest/HTTP/Handler.html),
[Plack](https://metacpan.org/pod/Plack::Middleware),
[Boost.Beast](https://www.boost.org/doc/libs/latest/libs/beast/doc/html/index.html).

### Required deliverable families and ordering

The following waves are dependency ordering, not permission to drop required
families or merge an incomplete migration. Registry package names/ownership and
supported version ranges are checked before release; conceptual IDs are stable.

| Wave | Required artifact scope | Exit gate |
| --- | --- | --- |
| Foundation | Rust Gate/Core shared authority, local decision/capture contract, browser SDK, conformance runner, capability descriptors and CLI doctor/install | Native OS startup/installation pilots and actual full Rust Docker human/attack flow |
| Initial native adapters | Node stream + Fetch cores, Express/Hono Node/Bun, nginx plugin, Rust Tower/Actix integration | Same contract corpus, actual runtime/plugin lifecycle and existing-service pilots; npm OIDC packaging for Node |
| Standard application ecosystem | Python ASGI/WSGI, JVM Servlet/reactive/common client, ASP.NET Core, PHP PSR-15/common client and Ruby Rack | Per-family package and representative host pilots; no package advertises untested optional host capabilities |
| Framework completion | Fastify/Koa/Nest, Deno/Hono, SSR loader hooks, Django/FastAPI, Spring/Ktor/Play/Ring, Laravel/Symfony/CMS installation recipes | Reuse family cores; registration/order/body/stream compatibility fixtures per advertised host |
| Server/platform completion | Certified proxy/ingress/platform profiles from the platform matrix, origin isolation and local-management installation policies | Real server configuration validation, topology-specific native/container pilots and full Docker authority |

BEAM/Swift/Dart/Crystal/Perl/other-language universal Gate profiles are required
ecosystem coverage. Their dedicated native packages are explicit candidates,
not required first-release SDKs and not current native-support claims. Promote
one only when a real lifecycle requirement cannot be met by the certified proxy
profile and its conformance/pilot matrix is supplied. This decision is semantic,
not an arbitrary cap on languages or frameworks.

### OS and architecture qualification

| Target family | Adoption environments | Artifact and installation decision |
| --- | --- | --- |
| os.linux-gnu | Debian/Ubuntu, RHEL/Rocky/Alma/Fedora, SUSE/openSUSE, Arch; x86_64/aarch64 | Native Gate/CLI archives and service profiles; declare minimum glibc, shared libraries, CA/DNS/timezone and service-account behavior |
| os.linux-musl | Alpine/minimal OCI; x86_64/aarch64 | Separate musl artifacts and native/container DNS/TLS/filesystem/locking/reload pilots |
| os.linux-other-cpu | armv7, riscv64, ppc64le, s390x, loongarch64 and further targets | Candidate builds only until toolchain/dependency and real hardware pilots qualify; Rust target availability is not product support |
| os.windows | Windows Server and desktop development; x86_64/aarch64 MSVC | Rust Gate/CLI archive/installer and Windows Service; ACL/pipe/network rejection, service account, spaces/Unicode/reparse points and rollback pilots |
| os.macos | Apple Silicon and Intel | Native Gate/CLI, launchd user-agent/system-daemon profiles; explicit signing/notarization policy |
| os.bsd | FreeBSD first, NetBSD/OpenBSD individually | Gate/CLI candidate qualification, rc.d/jails/socket permissions and real OS pilots; Linux containers are not BSD proof |
| os.unix-other | illumos/Solaris and other Rust-capable Unix | Conditional candidate registry, actual dependency/service-manager/hardware qualification before release |
| os.browser-wasm | Supported real browsers | Collector/WASM artifact only; no server installation or administration authority |
| os.wasi-isolate | WASI/managed edge isolate | Provider-specific limited runtime profile; not a native daemon replacement |

Baseline release gates require Linux/macOS/Windows native validation, with ARM
and musl qualification recorded independently. Other OS/CPU combinations remain
explicit candidates until piloted; do not silently label them supported via a
successful cross-compile. CLI diagnostics must emit these distinctions.

Sources: [Rust target support](https://doc.rust-lang.org/rustc/platform-support.html),
[systemd execution](https://freedesktop.org/software/systemd/man/latest/systemd.exec.html),
[Apple launchd](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html),
[FreeBSD rc](https://docs.freebsd.org/en/articles/rc-scripting/),
[FreeBSD jails](https://docs.freebsd.org/en/books/handbook/jails/).

### Server, ingress and orchestration families

| Family | Required profile/artifact | Qualification boundary |
| --- | --- | --- |
| server.nginx | Native nginx plugin plus auth_request and full-proxy configurations | Plugin ABI/version/access/filter/reload tests; subrequest is a limited access decision and cannot claim full body RIT or response transformation |
| server.openresty | Lua phase bridge profile to shared Rust services | Phase-specific asynchronous/body/filter restrictions; no duplicated Lua scoring/state authority |
| server.apache | mod_proxy/UDS and PHP/FastCGI deployment profiles | Prefix exclusion order, upgrade/output-filter/auth coexistence; not called a native module |
| server.iis | Native Rust Gate/IIS/HTTP.sys and ASP.NET Core/ARR profiles | HTTP.sys/IIS/Kestrel termination distinctions, app-pool recycle and service ACL |
| server.caddy | forward_auth and reverse_proxy profiles | GET precheck is a decision hook, not whole-body validation or HTML insertion |
| server.traefik | ForwardAuth/route middleware profiles | Explicit body forwarding/limits can break streaming; forwarded trust, cookies and response limits tested |
| server.haproxy | Gate path and SPOE/SPOA bridge profile | Agent timeout/failure/header mutation and original-vs-proxy connection provenance |
| server.envoy | ext_authz gRPC v3/HTTP adapters; ext_proc qualified processing profile | Raw-body/partial-body limits, route-cache mutation, filter order, cancellation/fail-closed; no unqualified work-in-progress features |
| platform.kubernetes | Gate sidecar/Deployment/Service, Gateway API and legacy Ingress profiles | TLS terminate/passthrough distinction, readiness/rolling/PDB/audit storage/namespace and origin isolation |
| platform.istio | CUSTOM ext_authz provider profile | CUSTOM/DENY/ALLOW composition, source principal and ingress-vs-sidecar observations |
| platform.linkerd | Gate/app adapter plus Linkerd authorization profile | Linkerd CRDs are not Envoy extension APIs; qualify its actual policy boundary |
| platform.docker | Nonroot Gate/sidecar Compose profile | Admin port not published; internal networks, volumes, IPv6, health, restart and graceful shutdown |
| platform.podman | Rootless OCI/user-service profile | UID mappings, low ports/network and host permissions separately qualified |
| platform.ecs | ECS/Fargate awsvpc task-sidecar profile | Sibling containers share localhost; security-group and admin identity are independent |
| platform.nomad-other | OCI/sidecar deployment template family | Candidate until actual namespace/discovery/rolling lifecycle and local-management pilot |

nginx on Windows is not the default production integration: upstream documents
beta status, worker limitations and absent UDP/QUIC support. Windows support is
provided through native Rust Gate/IIS profiles; nginx Windows is a separately
limited candidate. nginx auth_request interprets 2xx as allow, 401/403 as deny
and other statuses as errors. Map CHALLENGE through the explicit recovery
contract, not a blind status passthrough.

Envoy ext_authz whole-body integrity requires bounded complete raw bytes; partial
body metadata cannot satisfy complete-body RIT. The final authorized route is
bound to the decision, and later filter route-cache clearing/re-routing must
not bypass authorization. Disable failure-mode allow for protected routes.
ext_proc processing modes need individual stable-version qualification; no
work-in-progress duplex mode enters a support claim by association.

Sources: [nginx modules](https://docs.nginx.com/nginx/admin-guide/installing-nginx/installing-nginx-open-source),
[nginx auth_request](https://nginx.org/en/docs/http/ngx_http_auth_request_module.html),
[nginx Windows](https://nginx.org/en/docs/windows.html),
[nginx proxy semantics](https://nginx.org/en/docs/http/ngx_http_proxy_module.html),
[nginx WebSocket](https://nginx.org/en/docs/http/websocket.html),
[OpenResty phases](https://openresty.org/en/lua-nginx-module.html),
[Apache mod_proxy](https://httpd.apache.org/docs/2.4/en/mod/mod_proxy.html),
[IIS hosting](https://github.com/dotnet/AspNetCore.Docs/blob/main/aspnetcore/host-and-deploy/iis/index.md),
[Caddy forward_auth](https://caddyserver.com/docs/caddyfile/directives/forward_auth),
[Traefik ForwardAuth](https://doc.traefik.io/traefik/middlewares/http/forwardauth/),
[HAProxy SPOE](https://github.com/haproxy/haproxy/blob/master/doc/SPOE.txt),
[Envoy ext_authz API](https://www.envoyproxy.io/docs/envoy/latest/api-v3/extensions/filters/http/ext_authz/v3/ext_authz.proto.html),
[Envoy route-cache warning](https://www.envoyproxy.io/docs/envoy/v1.37.9/configuration/http/http_filters/ext_authz_filter),
[Envoy ext_proc modes](https://www.envoyproxy.io/docs/envoy/v1.39.3/api-v3/service/ext_proc/v3/external_processor.proto),
[Gateway TLS](https://gateway-api.sigs.k8s.io/guides/user-guides/tls/),
[Kubernetes Ingress](https://kubernetes.io/docs/concepts/services-networking/ingress/),
[Istio external authorization](https://istio.io/latest/docs/tasks/security/authorization/authz-custom/),
[Linkerd policy](https://linkerd.io/docs/reference/authorization-policy/index.html).

### Managed hosting and cloud families

| Family | Integration decision | Explicit capability boundary |
| --- | --- | --- |
| cloud.vm | EC2/Lightsail, Azure VM, GCE and VPS native/OCI Gate profiles | Direct-edge only when Gate actually receives original client TLS |
| cloud.aws-nlb | TCP passthrough Gate profile | TCP listener preserves encrypted stream; TLS listener termination changes observation owner |
| cloud.cdn-lb | ALB/CloudFront and other CDN/reverse-LB origin profiles | Treat as trusted-terminator/application; never label upstream TLS as client fingerprint |
| cloud.lambda | API Gateway authorizer/event bridge and Function URL/Web Adapter profiles | Versioned event is not raw HTTP stream; cache/session/route/policy bindings and cold-start/state/streaming limits |
| cloud.run | Cloud Run ingress-container/sidecar profile | Platform terminates TLS; HTTP/h2c to container does not restore client hello |
| cloud.functions | Cloud Functions/Firebase Hosting and Azure Functions isolated-worker profiles | Event/trigger/rewrite/static/cache boundary; no native local-admin equivalence in constrained hosts |
| cloud.azure-containers | Azure Container Apps ingress/sidecar profile | Separate platform TLS termination from internal app transport |
| cloud.workers | Cloudflare Fetch adapter with explicit cf metadata mapping | Provider hash/length/L4 facts are not full raw ClientHello; unavailable metadata stays unavailable |
| cloud.vercel | Node/Fetch Routing Middleware profile | Runtime/body/route restrictions; no native service/pipe or public admin workaround |
| cloud.netlify | Functions/Edge Request/Response and execution-chain profile | Rewrites/cache/framework hooks/split testing may bypass execution; route coverage must be proven |
| cloud.render | Rust process/OCI application profile | Public PORT and local admin separate; managed TLS ends before application HTTP |
| cloud.railway | HTTP service and TCP proxy profiles | Managed HTTP TLS vs raw TCP topology qualify independently |
| cloud.fly | Gate machine/container profile | Default termination and TLS passthrough are different profiles |
| cloud.heroku | Router/application process profile | Router/idle timeout, streaming and dyno lifecycle/state constraints |
| cloud.other | New PaaS/CDN/edge providers | Add without target-count cap, but map actual provider contract and pilot before promotion |

Managed runtimes incapable of local administration do not expose remote admin as
a substitute: administration stays at the locally accessible owning Rust node,
and the constrained adapter declares that local control feature unavailable.
All-feature routes require a topology that supplies their capabilities; the
existence of an edge Fetch wrapper does not qualify such a route automatically.

Sources: [NLB listeners](https://docs.aws.amazon.com/elasticloadbalancing/latest/network/load-balancer-listeners.html),
[API Gateway authorizers](https://docs.aws.amazon.com/apigateway/latest/developerguide/http-api-lambda-authorizer.html),
[Cloud Run contract](https://docs.cloud.google.com/run/docs/container-contract),
[Workers request metadata](https://developers.cloudflare.com/workers/runtime-apis/request/),
[Vercel runtime](https://vercel.com/docs/functions/runtimes/edge),
[Vercel routing middleware](https://vercel.com/docs/routing-middleware),
[Netlify limits](https://docs.netlify.com/build/edge-functions/limits/),
[Netlify execution chain](https://docs.netlify.com/build/edge-functions/api/),
[Firebase configuration](https://firebase.google.com/docs/hosting/full-config?hl=en),
[Azure HTTP bindings](https://learn.microsoft.com/azure/azure-functions/functions-bindings-http-webhook),
[Render services](https://render.com/docs/web-services),
[Railway networking](https://docs.railway.com/networking),
[Fly networking](https://fly.io/networking/),
[Heroku routing](https://devcenter.heroku.com/articles/http-routing).

### Protocol and workload coverage

| Workload family | Required adapter contract and pilot |
| --- | --- |
| HTTP/1.0 and 1.1 | Framing checked before origin, duplicate CL/TE/obs-fold/CRLF, chunking and Expect/continue, absolute target/authority consistency |
| HTTP/2 | Pseudo-header rules, duplicate/order provenance, forbidden connection fields, H1/H2 conversion, reset/cancellation and trailers |
| HTTP/3/QUIC | Explicit candidate until separate capture/QPACK/reset/migration pilot; never infer original client H3 through a downstream H2 connection |
| mTLS | Distinguish direct certificate from authenticated forwarded certificate; forged XFCC/Client-Cert rejected |
| gRPC unary and streaming | Metadata/binary metadata/deadline/status/trailers; client/server/bidirectional stream, cancellation, half-close and backpressure; no HTML challenges |
| gRPC-Web and Connect | Binary/text trailer envelopes, CORS, unary GET/POST, compression and stream framing; separate from ordinary gRPC claims |
| WebSocket | Pre-upgrade decision, Origin/session/subprotocol and CSRF boundary, frame tunnel/close/reconnect qualification; frame inspection is a separate capability |
| SSE and LLM streams | First-event timing, heartbeat/flush, Last-Event-ID/reconnect, disconnect and no whole-stream buffering/insertion |
| Multipart and large uploads | Bounded pre-origin validation/spool or explicit streaming contract, fragmented boundaries/cleanup/size/time limits |
| Compressed/signed/webhook bodies | Exact selected byte layer, decompression limits, payment/provider signatures and duplicate delivery; no parse/reserialize mutation |
| Downloads and caches | HEAD/204/206/304, Range/If-Range/ETag/digest, attachment and cache/Vary semantics; byte-preserving noneligible responses |
| GraphQL | HTTP GET/POST/batching/persisted-query and subscription WebSocket transports qualify separately |
| WebDAV/CalDAV/git smart HTTP | Unusual methods, locks/conditional headers, large/binary bodies and auth/redirect lifecycle; not assumed from REST compatibility |
| CONNECT/WebTransport/MASQUE | Initial unsupported/explicit pass-through-only candidate; no full security-layer claim without dedicated design/pilots |
| SMTP/MQTT/AMQP/raw TCP | Outside web-adapter product coverage; standard HTTP origin proxy support does not imply these protocols |

Sources: [HTTP/1.1](https://www.rfc-editor.org/rfc/rfc9112.html),
[HTTP/2](https://www.rfc-editor.org/rfc/rfc9113.html),
[HTTP/3](https://www.rfc-editor.org/rfc/rfc9114.html),
[gRPC lifecycle](https://grpc.io/docs/what-is-grpc/core-concepts/),
[gRPC cancellation](https://grpc.io/docs/guides/cancellation/),
[Connect protocol](https://connectrpc.com/docs/protocol/),
[WebSocket](https://www.rfc-editor.org/info/rfc6455/),
[SSE](https://html.spec.whatwg.org/dev/server-sent-events.html),
[Fetch](https://fetch.spec.whatwg.org/),
[CSP3 draft](https://www.w3.org/TR/CSP3/),
[SRI draft](https://www.w3.org/TR/sri/),
[Envoy forwarded certificate handling](https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_conn_man/headers.html).
Draft standards are not recorded as stable guarantees; qualify the actual
supported browsers and shipped host versions.

### Critique reflected and residuals

The language reviewer identified two P0 gaps in the first draft: verification
could occur after origin side effects, and decisions lacked request/tenant/body/
stream/freshness binding. Both are now explicit mandatory contracts above.
Platform/protocol reviewers additionally identified Windows pipe remote access,
shared container loopback, auth-subrequest body gaps, Envoy route-cache bypass
and silent response/CSP mutation. Each is now a conformance/installation blocker.
The artifact matrix resolves the P1 ambiguity between universal proxy profiles
and dedicated native packages; the final Go source-cleanup objective takes
precedence over adding a new in-repository Go SDK.

No numerical latency/throughput budget is invented from research. Measure baseline
and adapted workloads, then record accepted p50/p95/p99, first-byte/first-event,
memory/spool/CPU and error budgets per deployment profile before qualification.
T4 coherent-browser automation remains an intentional ceiling. Managed-edge
capability gaps, changing provider APIs and untested CPU/OS tuples remain visible
residuals, not waived implementation gates. No third-party probing occurred.

Management sources: [Windows pipes](https://learn.microsoft.com/windows/win32/ipc/named-pipes),
[pipe ACLs](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights),
[Unix sockets](https://www.man7.org/linux/man-pages/man7/unix.7.html),
[Kubernetes Pods](https://kubernetes.io/docs/concepts/workloads/pods/),
[ECS shared network](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task-networking-awsvpc.html).

## Why

The user explicitly requested unrestricted breadth of hypothetical adopter
research through subagents, followed by a canonical Cross-OS/Cross-Platform/
Cross-Lang adapter ecosystem decision. Existing source/deployment inspection
shows an nginx origin behind Gate, not a delivered general nginx plugin or a
cross-language adapter family. The design separates investigation scope from
verified shipping claims and closes trust/fidelity gaps before implementation.

## Verification

Read-only language, platform and protocol subagents supplied independent official
source research. The follow-up adversarial review initially returned needs-work;
after the P0/P1 deltas above, its second review returned ready with no remaining
architecture P0/P1. Status active means accepted design/development scope only.
Wire schemas, registry ownership, exact support tuples, conformance and
native/Docker pilots remain implementation/release gates. Replace manual design
verification with executable descriptor/contract coverage checks as they land.
