# Rust nginx adapter (experimental Linux component)

New authored module and tests are Rust1.99.1/edition2024 in
`crates/nginx-adapter`. The official pinned ngx-rust SDK binds a pinned upstream
nginx C engine; it does not turn nginx into a Rust engine. The native module is
Linux HTTP1.1 only initially. No default production-qualified tuple is claimed.

Use `humanymous_guard /canonical/private/provision.json;` at the protected server
level. The supervisor obtains the real authority credential through its private
readiness pipe and writes an owner-only0600 file in a canonical0700 directory.
The provisioning schema is Configuration in the Rust request module; server
profile must be explicitly Nginx at authority startup (`"adapter":"nginx"`).
Never provide a decision credential through client headers, logs, argv or env.

The module snapshots complete body/target/ordered sanitized headers, uses shared
authority::client, and continues normal proxying only after a live bound ALLOW.
DENY403, CHALLENGE409 without recovery, failure503. Unknown/reserved paths and
unsupported framing/streams reject before origin. Uploads are bounded at1MiB;
file-backed/incomplete bodies, Transfer-Encoding, trailers, upgrades, gRPC and
non-HTTP1.1 are unsupported. Binary/content-encoded bytes and ordinary duplicate
headers are preserved. No HTML/CSP insertion or response-body filtering.

Configure buffered complete uploads, no post-authorization rewrite, no alternate
public origin path, `proxy_next_upstream off`, `proxy_cache off`,
`proxy_buffering off`, and `proxy_ignore_headers X-Accel-Redirect X-Accel-Buffering`.
The Rust Docker fixture writes an exact isolated configuration for these gates.
Operator-controlled nginx configuration remains a trusted integration boundary.

The root27-member locked snapshot passes7unit tests, native/pilot strict Clippy
and35actual Compose groups. Reproduce the component qualification with:

```
cargo test --locked -p humanymous-nginx-adapter --lib
docker compose -p humanymous-nginx-native -f adapters/nginx/compose.yaml build
docker compose -p humanymous-nginx-native -f adapters/nginx/compose.yaml run --rm native-pilot
docker compose -p humanymous-nginx-native -f adapters/nginx/compose.yaml down
```

The Docker runner is Rust, runs non-root/network-none/read-only, and contacts
only its own origin and Unix authority. Positive ALLOW tests use test-only frozen
synthetic evidence through the actual authority/library and real audit WAL;
production ApplicationHttp results are separate. Neither is full browser/human/
AT/product qualification. Shared peer/replay/audit checks are not cloned.

`legacy-njs/` preserves the prior b1e10ae handwritten njs/Python mock pilot without
new foreign-source edits. Its old24unit/28Docker result is historical, cannot
qualify the native module, and is excluded from the current runtime artifact.
See INTEGRATION.md and the native nginx Specfold plan for remaining gates.

The real production executable's application-http profile returned409 with no
origin. Positive ALLOW uses explicitly synthetic trusted evidence. Same-WAL
restart/credential reload passed with a local verified checkpoint; independent
witness, cross-user matrix, long-stream backpressure and full product/human/AT
remain open. A32-request batch admitted16/rejected16 with exactly16origin calls;
reload observations include503 transitions. These are measured service impacts.
