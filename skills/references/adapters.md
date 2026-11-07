# Connect supported service adapters

Begin with the installed adapter's capabilities and qualification receipt for the
exact runtime/framework/OS/architecture. A package descriptor's candidate tuple,
source directory or compiled native module is not a support matrix. Do not
generate foreign-language wrapper glue or introduce a second scoring engine.
If the requested language/framework is not qualified, report that boundary and
identify the owning adapter contract rather than inventing an installation path.

The experimental local protocol is `humanymous.adapter/0.1`, not a public web
standard or a replacement for frozen RIT. Rust authority owns decisions,
provenance, replay, credentials and audit. The deployment must provision endpoint,
scope, bearer/private-input and actual server peer identity (UID or Windows
SID/ACL plus authority PID) through the protected deployment mechanism. Never
choose a producer profile or server trust identity from incoming request fields.

For Node/Bun, the authored native package is `humanymous-node-adapter`. Its
accepted API includes `express(optionsJson)`, `hono(optionsJson, app, streamModule)`,
`htmlHook(nonce, scriptPath)` and `capabilities()`. Configure only the exact
installed schema. Middleware must precede application parsers/effects. Node
raw-header and Fetch-normalized provenance differ; Bun Express normalization
cannot satisfy a route requiring original raw casing/order. A generated npm
loader is upstream glue, not proof that a tuple was actually qualified.

For nginx, verify the installed native module's exact nginx ABI and provision
`AdapterProfile::Nginx` at the authority. `nginx-raw` describes ordered/cased HTTP/1
ingress headers; `nginx-normalized` does not preserve that claim. Neither supplies
the original client TLS/H2, a peer or a connection identity. Do not relabel nginx
as Node or rename a legacy mock wire to satisfy authorization.

Version0.1 authorizes complete bounded requests only, with one consumed response
bound to the request/deadline immediately before the single application
continuation. Stream authorization, remote authority transport, capture upload
and admin methods are outside that version. Response streaming/SSE must preserve
normal service behavior; it does not grant streaming-request authorization.

Verify dependency refusal, peer/ACL mismatch, timeout/cancellation, replay and
normal duplicate headers/Set-Cookie/body/SSE behavior in an owned environment.
Count actual origin/audit effects. A synthetic ALLOW is a test fixture, not
production browser/human qualification. Do not advertise framework support from
schema-only validation. Report unresolved cross-user, load/backpressure or full
product gates without weakening enforcement.
