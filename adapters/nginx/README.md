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

Historical source16c18f3's root27-member locked snapshot passed7unit tests,
native/pilot strict Clippy and35actual Compose groups. The subsequent JSON timer,
strict report and packaging changes remain unqualified until the hosted run.
The CI owner supplies HUMANYMOUS_SOURCE_REV (checkout commit),
HUMANYMOUS_SOURCE_MANIFEST (frozen source-manifest SHA256), and
HUMANYMOUS_NGINX_IMAGE (source-bound local image tag). Missing values fail closed.
The required component's fixed command interface is pilot-contract.json:

```
cargo test --locked -p humanymous-nginx-adapter --lib
docker compose -p humanymous-nginx-native -f adapters/nginx/compose.yaml build
docker compose -p humanymous-nginx-native -f adapters/nginx/compose.yaml run --rm native-pilot
docker compose -p humanymous-nginx-native -f adapters/nginx/compose.yaml down --remove-orphans
```

The Docker runner is Rust, runs non-root/network-none/read-only, and contacts
only its own origin and Unix authority. Positive ALLOW tests use test-only frozen
synthetic evidence through the actual authority/library and real audit WAL;
production ApplicationHttp results are separate. Neither is full browser/human/
AT/product qualification. Shared peer/replay/audit checks are not cloned.

Run these process/IPC/ABI checks on the standard hosted runner under the shared
delivery policy. A successful report requires all35 unique scenario IDs, exact
source/lock identities, exit0 and no failed/ignored/not-run groups. Counts denote
scenario groups, not Rust harness tests. Challenge now includes JSON and four
requests on one keepalive connection; that extension is still NOT_RUN.

The image carries package-manifest.json with bounded module/engine hashes,
Cargo.lock SHA256 and exact toolchain/target/nginx/configure/SDK/source inputs.
The Rust writer uses a separate package feature graph after saving the native
module. The pilot compares actual bytes and engine build identity. This is
unsigned build provenance; abi_load_qualified and signed_release remain false.
The actual hosted module-load receipt is separate from package metadata.

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
