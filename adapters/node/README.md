# Experimental Rust Node adapter

Rust Node-API callbacks connect Express and Hono to the single Rust authority.
No second scoring engine, RIT implementation or public administrative router is
implemented here. Pinned upstream NAPI CLI generates the CJS/ESM loader and
declarations. Historical handwritten src/ and test/ files are excluded from the
archive. New implementation and tests are Rust1.99.1/edition2024.

Qualification is in progress. Windows and isolated Linux component measurements
are separate from full product acceptance; releaseQualifiedTuples remains empty
and npm publication is gated. Later source edits await resumed verification.

`createExpressServer(optionsJson, app)` returns an owned HTTP server. Its guard
runs before the existing app's parsers/effects and rejects unsupported upgrade/
CONNECT events before application callbacks. Do not add competing server
listeners or rewrite authorized routing later. `express(optionsJson)` is
advanced root middleware: install before every parser/effect; independent
upgrade listeners are outside its coverage.

`hono(optionsJson, app, streamModule)` returns an outer Fetch handler. Supply the
real upstream node:stream module. Node Hono uses the upstream Node adapter; Bun
Hono uses actual Bun.serve. Exact complete body validation precedes app.fetch.
The original Request's metadata and cancellation are retained with sanitized
headers and verified body bytes. The latest metadata-cloning edit is not yet
validated after a resource-priority hold.

Strict options JSON requires installation, tenant, origin, revision, endpoint,
credential, server, routes, timeoutMs and maxBody. server is the actual authority
peer: Unix kind/uid or Windows kind/sid/pid, where pid is the authority process.
A trusted supervisor delivers the fresh credential privately. Only restricted
local filesystem sockets/named pipes are supported; no TCP/admin fallback or
retry. routes is a complete pathname inventory. Unknown/reserved paths fail
closed; the reserved /_humanymous subtree currently serves no script or recovery.

maxBody is at most1MiB, timeoutMs at most30seconds, and body/decision ownership
is process-bounded to64. ALLOW is consumed once and checked for expiry immediately
before the app call. DENY returns403, CHALLENGE409 JSON, dependency/input
errors503 no-store. Missing required observations never fabricate an ALLOW.

Verified Node HTTP1 raw headers retain order/casing. Bun1.3.14's HTTP shim
lowercases rawHeaders, so Bun Express explicitly uses normalized evidence;
strict raw-observation routes remain unready. Hono Fetch is normalized. Original
TLS/H2/H3 and authenticated client capture are unavailable from middleware alone.
Internal/forwarding trust assertions are removed while app authorization/cookies
and response Set-Cookie meaning are preserved.

Uploads are bounded complete bodies. Upload streaming, request trailers,
WebSocket, CONNECT and gRPC remain unqualified. App-owned SSE/download response
streams pass through without HTML rewriting or whole-stream buffering.
`htmlHook(nonce, scriptPath)` returns explicit template markup and a CSP source;
it serves no script and does not modify CSP, compression, signatures, ETags,
Content-Length, caches or HEAD/204/304. Latest HEAD/304 test additions are pending.
`capabilities()` reports the runtime observation profile and unavailable planes.

Build from the checkout with npm ci --ignore-scripts, then npm run build. The
Rust driver uses exclusive generation ownership and exact upstream CLI3.10.12.
Generated outputs are ignored under native/. Missing outputs make npm prepack
fail closed. Native packaging/source/provenance/archive checks still need their
final run. Node >=22 is an install floor, not an all-version/OS support claim.
There are no runtime npm dependencies; native platform requirements and release
receipts remain explicit. No npm publication was performed.
