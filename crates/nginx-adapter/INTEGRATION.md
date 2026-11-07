# Rust nginx integration delta

Root owns Cargo.toml/Cargo.lock/toolchain and has registered crates/nginx-adapter
in its27-member working workspace. Package humanymous-nginx-adapter,
Rust1.99.1/edition2024, publish=false, rlib+cdylib. Inherits workspace packages
and existing serde/json/base64/sha2/hex/getrandom/zeroize. Reuses authority,
integrity; optional pilot uses audit/signals. Existing Tokio1.54.1 and rustix1.1.

One new optional Linux dependency is official nginx/ngx-rust revision
ebbcd514ee7a336fa4b622232170d8a16763a9ca (ngx0.5.0). `native` enables it;
`export-modules` enables native exported module table. Default unit checks do
not claim nginx ABI qualification. Build native against a configured nginx1.30.6
source/build pair, exact same flags as the runtime binary. No shared lock edits.

Public shared parser: integrity::adapter::{AdapterProfile::Nginx,
parse_with_profile}; implemented root64852c4. Exact producer contract lives in
Node-owned adapter-service-contract SoT. This crate does not impersonate Node.

Current working authority-owned shared client: ClientOptions{endpoint,credential,
server:&Peer,timeout_ms,adapter:AdapterProfile}; ExpectedBinding{id,tenant,
origin,revision,deadline}; async decide(options,envelope,expected) -> BoundResponse
with Verdict Allow/Deny/Challenge; BoundResponse.verdict()/expires() are inspection,
consume(self) rechecks wall+monotonic expiry and is used before origin release.
Owner validation/canonical checkpoint pending. No IPC validator/peer/replay/audit
clone or mock production fallback.

Status: root integrated the crate in the27-member working workspace. Root locked
Windows7unit/Clippy pass; immutable unmodified root Cargo/lock Linux snapshot
native+pilot builds/strict Clippy and35Compose groups pass. Actual production
authority process409, credential/reload and same-WAL authenticated restart pass.
No full product/human/AT claim. See native plan and sanitized
evidence:root/20261107052501 for source hashes, retries and remaining witness/
cross-user/sustained-load/backpressure qualification.

P1 source-only follow-up: CHALLENGE409 JSON/timer finalization and Rust JSON
assertions are authored; native build/Clippy/runtime NOT_RUN under resource hold.
The35-group result belongs to16c18f3 status/no-origin behavior. Future Docker
compilation defaults CARGO_BUILD_JOBS=1. Root must allocate the one heavy slot or
remote runner before execution; no cloud is provisioned by this checkpoint.
