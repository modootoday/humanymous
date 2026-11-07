# Rust Node-API adapter integration

Local container canon is Windows Docker Desktop: invoke the explicit Windows
docker.exe with --context desktop-linux. If PATH is missing, use
C:/Program Files/Docker/Docker/resources/bin/docker.exe; do not switch to WSL
Docker. Before an allocated build/run, retain context endpoint, server ID/version
and OSType privately. SDK clients such as act require explicit Desktop DOCKER_HOST.
Hosted Linux and WSL-native Rust are separate execution surfaces. Historical
Linux component receipts without engine identity are not Desktop provenance.
No Docker preflight/build/run was executed during the benchmark quiet window.

Latest packaging source exposes actual hostPilotEnabled and uses one native
initializer for read-only capability receipts and optional host-pilot startup.
Production-load assertion rejects host-pilot binaries before fixture code. The
Rust package inspector loads generated CJS/ESM under Node/Bun with closed stdin,
fixture/loader overrides cleared, and requires one actual Rust capability receipt
with exact false. Old binaries that ignore the mode and emit no receipt reject.
Two pure receipt-validation definitions cover absent/duplicate/invalid/oversized
and non-boolean states. Summary006 records default and host-pilot all-targets
check/Clippy success for this initializer/package code. Shared audit source moved
during the batch, so full immutable-closure qualification is not established.
Node parser units, actual native imports and framework cases remain NOT_RUN.
Actual child timeout and
stream memory supervision remain the invoking runner's responsibility; accepted
receipt sizes are checked after capture, not claimed as streaming bounds.

Hosted source813 job128361689495 completed native build/inspection stages then
failed at missing qualification.json. It did not run host_process groups before
that gate; summary004 in evidence:root/20261107075243 preserves the distinction.
No qualification receipt has been invented from loader success.

Rust CLI install-test ARCHIVE adds a separate clean npm archive gate. Explicit
allocated pilot opt-in is required. It reads at most100MiB, copies those exact
hashed bytes to a fresh target/node-native/install directory, and invokes npm
offline with lifecycle scripts/audit/funding/lockfile disabled. It verifies the
installed package identity/exports, loads installed generated CJS/ESM under both
Node/Bun (four cases), then resolves the package through Node --require/--import
with closed stdin (two cases, no authored JS). All six require actual production
feature receipts. Success emits archive SHA256 and component-only install status;
it does not establish generation lineage, frameworks, or whole-product support.
The source passed the summary006 Windows compilation/lint gates, with the shared
closure limitation above. Actual installation/import has not executed; staging
inspection is not clean install.

Current followup consumes framework-adapter shared config/envelope/entropy and
process admission. NAPI error/host lifecycle remain here. Common schema adds only
optional syncInputDeadlineEnforced=false; original Node required fields remain.
Credential buffer is zeroizing. Shared migration is compile/unit/runtime NOT_RUN;
previous Windows52/Linux36 receipts do not validate this new source. The admission
unit moved to shared framework context; Node pure envelope3/template1 remain.

Root Cargo delta: add `crates/node-adapter` to workspace members. The crate's
manifest carries exact Node-API dependencies: napi=3.16.1 (default-features=false,
napi8,dyn-symbols,tokio_rt), napi-derive=3.7.0, napi-build=2.6.0. Other dependencies
reuse root-resolved serde/json/base64/sha2/hex/getrandom/tokio1.54.1 and existing
integrity/authority. Root alone updates Cargo.lock. Rust1.99.1/edition2024.

Safe Rust host callbacks use upstream Node-API implementation; no authored JS/TS,
eval, script string or duplicated authority client. The ABI macro's generated
unsafe code is upstream generated glue; authored implementation uses safe APIs.
The shared authority owner provides authority::client for actual IPC decisions.
Build output cdylib must be copied to an exact `.node` path by Rust packaging.
Node/Bun host pilots are authored as Rust Node-API callbacks, loaded by the host
with `--require <absolute .node>`; they never pass code to `--eval`.

Bun1.3.14 direct .node preload is treated as import and refused. Its Rust host
fixture uses pinned upstream-generated CJS preload and an unchanged upstream
module entrypoint. Node loads .node directly. Bun Hono is actual Bun.serve; Node
Hono uses upstream getRequestListener. Bun Express normalizes rawHeaders names,
so it uses fetch-normalized provenance and cannot satisfy strict raw routes.

Implementation and current results are recorded in the Node adapter Specfold
plan. Existing handwritten JS package remains legacy until native replacement
passes the measured framework/authority topology gates. No native support claim
follows from compiling the cdylib alone. No npm publish authorized here.

Pending CI interface: native build/generation uses the Rust package driver
`cargo run --locked -p humanymous-node-adapter --bin humanymous-node-package -- build`
from a checkout with `npm ci --ignore-scripts` in adapters/node. It invokes exact
upstream @napi-rs/cli3.10.12 twice for generated commonjs/esm loaders and their
declarations, with --release and Cargo --locked. No authored loader source.
`inspect` rejects missing/empty/non-LF glue and missing native binaries and records
generator version and SHA256. Candidate targets in npm napi configuration are
build targets only, not a support matrix. Pack must include only generated native
outputs plus README/LICENSE/capabilities/metadata, excluding legacy src/test,
node_modules, host-pilot binary and private evidence. Windows/Linux/macOS and ARM
runtime claims require independent actual execution before promotion.

The accepted receipt is target/node-native/qualification.json, schema
humanymous.node-native-qualification/1: source_sha,package_manifest_sha256,
package_version,provenance_sha256,full_native_qualified,host_runs,
authority_docker,advertised_tuples. Runs retain exact versions/OS/arch,
test_count/failed/ignored, authority artifact SHA256, evidence_profile,
ALLOW/CHALLENGE/DENY/error counts and origin_effect_count. Synthetic evidence
cannot qualify production browser ALLOW. Tuples exactly match descriptor
releaseQualifiedTuples; missing/empty receipts block publication. No qualified
receipt has been generated. Publish consumes same-run validated archive hashes.

Memory-priority hold: no owned process/session/container remains active. No new
cargo/rustc/Docker/global-spec work until root reschedules it. host-all-006 passed
5groups/52cases on Windows with20synthetic effects/audit records. Earlier Linux
image f20dbad10f628... passed5groups/36cases under nonroot/network-none/read-only/
resource limits. Later HEAD/304/Request cloning, capabilities, package-surface and
generation-lock edits are written but unvalidated. Old success does not cover
these bytes. Resume final source tests, current Linux Docker, pack/type/advisory/
source-proof gates; do not publish. Shared Cargo/toolchain/authority remain with
their respective owners. New source is a P1 checkpoint, not overall completion.
