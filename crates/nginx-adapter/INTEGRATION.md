# Rust nginx integration delta

## Root-coordinated adapter-family naming handoff

Target convention from sot:root/structural-development is
`humanymous-nginx-adapter` -> `humanymous-adapter-nginx`. This is a project naming
decision, not a Cargo syntax requirement or a completed rename. Keep current
names until the single root graph/source/lock/workflow/artifact migration.

The subsequent user clarification makes public consumer identity `humanymous`.
The adapter-family Cargo name is internal only. All current implementation-
specific import/loader names below are LEGACY inventory, not preferred public
names. Root must atomically map the public Rust facade to humanymous and the
nginx loader to a humanymous basename plus platform extension in a host/ABI-
isolated destination. Do not place conflicting host DLLs in one shared directory.
No registry ownership, facade implementation or publication is claimed here.

| Surface | Current contract | Coordinated impact or explicit compatibility decision |
| --- | --- | --- |
| Cargo package / selection | `humanymous-nginx-adapter`, `publish=false` | Change package/dependency/lock identities and every `cargo -p` build/test/Clippy/registry-policy selector together. No crates.io publication or existing registry alias is implied. |
| Rust library | No explicit `[lib].name`; default `humanymous_nginx_adapter` | Default target identifier becomes `humanymous_adapter_nginx`; update pilot/package binary imports and qualified calls. Pinning the old lib name would also retain the old Rust identifier, so it is a separate compatibility choice, not an automatic solution. |
| Compiler native output | `libhumanymous_nginx_adapter.so` | With the new default library name the output becomes `libhumanymous_adapter_nginx.so`. Update Docker build-output/install paths atomically. |
| Installed nginx module path (legacy) | `/opt/nginx/modules/libhumanymous_nginx_adapter.so` | Target a public humanymous basename, for example a root-approved nginx/ABI-isolated `humanymous.so`; exact destination/transition alias belongs to the common ABI canon. Map load/read/hash paths together. The old path is not the preferred public face. |
| nginx C ABI / directive | SDK `ngx_modules!` exports unmangled `ngx_modules`, `ngx_module_names`, `ngx_module_order`; module-name entry `ngx_http_humanymous_module`; `humanymous_guard` | Preserve these host-facing tables/name/directive unless separately mapped. The descriptor static is not independently declared no_mangle in authored source. A Cargo family rename does not authorize changing the nginx ABI or directive. |
| Explicit binaries | `humanymous-nginx-pilot`, `humanymous-nginx-package` | These `[[bin]].name` values do not automatically follow the package name. Retain until root maps binary names; any change must update Docker command/copy paths. Installed `/opt/pilot` is another explicit fixture contract. |
| Unsigned provenance package field | Exact `humanymous-nginx-adapter` in writer, validator and tests | Map to the new package identity in all three together. Preserve historical manifests/receipts; do not make a current validator accept arbitrary legacy identities. A manifest/version compatibility policy must be explicit. |
| IPC/report/fixture identity | `humanymous.nginx`, Nginx profile, `humanymous.nginx.pilot/0.2` and package/report schemas | Preserve protocol identities and required35 IDs unless the accepted contract changes. They are not Cargo package names. Image tag family `humanymous/nginx-native` is also independently selected by CI. |
| Source paths / consumers | `crates/nginx-adapter`, `adapters/nginx`, CI nginx runner, lab contract consumer | No directory move is approved here. Root decides paths and updates workspace membership, include paths, packaging contexts, docs/reviews associations and writer claims in the same migration. Shared authority/integrity/native-security providers are not adapter-family renames. |

Bounded search inspected this manifest, package writer/validator/tests, pilot
imports/load/hash paths, native exports/directive, Docker recipe, typed contract
and CI nginx selectors. This handoff changes documentation only. Requalify the
new selected graph, pure tests/Clippy, saved native output, installed loader path,
package identity checks and actual nginx/authority35 after migration; do not
inherit exact8132117 success into renamed bytes. Existing source IDs, raw commands,
hashes, private evidence and historical package names remain unchanged.

## Committed host ABI inventory

Read at4c4c6251602c0a9412264ca1e07d45c29dcd809c, no builds/renames. The nginx
diff from8132117 then contained only integration/README documents; this does not
qualify the current graph. Primary sources: Cargo.toml, src/native.rs,
src/request.rs, src/report.rs, src/package.rs, adapters/nginx/Dockerfile.native.

- Legacy package humanymous-nginx-adapter has no explicit lib name. Rust default
  is humanymous_nginx_adapter; rlib/cdylib output includes Linux
  libhumanymous_nginx_adapter.so. Internal future package/library defaults are
  humanymous-adapter-nginx / humanymous_adapter_nginx, not the public facade.
- The pinned SDK src/lib.rs ngx_modules macro exports unmangled DATA tables
  ngx_modules, ngx_module_names, ngx_module_order. Name entry:
  ngx_http_humanymous_module. The authored descriptor static has no independent
  no_mangle annotation; no separate authored init export exists. nginx consumes
  generated ngx_module_t/ngx_http_module_t and extern-C callback pointers for
  postconfiguration, configure, body readiness and timers. SDK
  nginx-sys/src/lib.rs default supplies nginx_version/NGX_RS_MODULE_SIGNATURE:
  engine/configure-specific fields, not a portable project ABI version. The
  loader tables are justified host ABI names, not implementation product brands.
- Wire: humanymous.adapter/0.1; humanymous.nginx/0.1.0-experimental.1; nginx-raw
  means parsed ordered application HTTP, no original TLS/H2/connection/capture
  provenance. Report0.2 and unsigned package0.1 are separate schemas. Root maps
  any wire/directive compatibility change explicitly.
- nginx owns live pointers, pools/events and callback serialization. Only copied
  Rust data crosses to Tokio, initialized in a worker request after fork. Pool
  cleanup aborts tasks/deletes timers. Compare the snapshot and consume the
  shared bound result once before proxy continuation. JSON finalizes after the
  SDK wrapper, with no cell borrow crossing finalization. ABI/live-allocation
  validity remains a host premise. Unwind guards do not cover OOM/abort/SIGKILL
  or memory corruption. Errors400/413/503, DENY403, CHALLENGE409/no-store JSON
  without recovery are distinct from nginx parser errors.
- Actual qualification: exact8132117, hosted Linux Docker HTTP1.1, nginx1.30.6,
  ngx ebbcd514, Rust1.99.1, x86_64 GNU/bookworm recipe,35groups and actual
  module/engine/image hashes in evidence:root/20261107052501. Missing: other OS/
  arch/musl/vendor-configure tuples, TLS/H2/H3, streaming upload/upgrade/gRPC,
  sealed startup, signed installed release, full product and human/AT.

No general ABI stability or future source qualification is claimed. Public
identity humanymous, host/ABI-isolated loader destinations and the Rust facade
require root's common ABI canon/critique before implementation; existing loader
and implementation-specific imports are legacy, not target public names.

Latest hosted qualification: exact8132117e7bf1445939d7dee9066fcf45ccf0915b,
run42355168729/job128358355202,35scenario groups passed/0failed/0ignored/0not_run,
cleanup0 and source/image identity unchanged. The pinned build enforces native,
pilot/package builds and strict Clippy; separate local pure14/check/Clippy passed.
See hosted-813 receipts in evidence:root/20261107052501. Source-only status notes
below are historical. No future worktree, sealed-startup or full product claim.

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

Current unqualified extension: pilot-contract.json fixes35 required scenario IDs,
report0.2 and source-bound Compose env/argv. CI and product-lab consume that
contract separately from product readiness. A package feature builds the Rust
provenance writer without native symbol linkage; Docker preserves native .so
before building pilot/package. Manifest0.1 binds actual module/engine/lock and
engine configure/source/SDK/toolchain tuple. It remains unsigned and unqualified
until independent actual hosted load receipt. Pilot compares exact manifest
inputs before success output. JSON keepalive repetitions and these new pure
tests/report/package builds are NOT_RUN; source16c18f3 receipts do not cover them.
The owner holds adapter-nginx; Apache and Compose lifecycle remain other owners.
