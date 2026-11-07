# Rust product-lab runner contract

Current implementation is authored Rust 1.99.1/edition 2024. Root owns workspace
registration: member `test/lab/runner`, package/binary `humanymous-lab-runner`,
workspace serde_json/sha2/getrandom only; no new production dependency version.
No root Cargo/toolchain changes belong to this lane.

```
cargo run --locked -p humanymous-lab-runner -- run --evidence EXISTING_LOCAL_SPECFOLD_BUNDLE --require-full
cargo run --locked -p humanymous-lab-runner -- run --evidence EXISTING_LOCAL_SPECFOLD_BUNDLE --topology-only --require-full
cargo run --locked -p humanymous-lab-runner -- run --evidence EXISTING_LOCAL_SPECFOLD_BUNDLE --collector --require-full
cargo run --locked -p humanymous-lab-runner -- cleanup
cargo run --locked -p humanymous-lab-runner -- provider-preflight LOCAL_MANIFEST
```

`run` builds from an immutable allowlisted source copy and uses unique owned
Docker project/image tags. Host code coordinates Docker only. All HTTP/negative
clients execute in hardened containers, without shell/script execution. Exact
files and hashes are source-manifest.json, resolved-compose.json,
runtime-image.json and per-service/network inspect JSON. Docker CLI output is
registered as private docker-transcript.json. New Dockerfiles live in
deployments/lab/images; build/rust remains another owner's read-only surface.

Output lab-report.json schema `humanymous.product-lab/2` has evidence_types,
cases[{id,status,detail}], components.runtime (actual binary pilot report),
optional components.collector, required_unavailable, full_product_qualified and
release_ready. Exit 0 means scoped cases passed; --require-full returns 2 while
full product gates are unavailable; failures return 1. Never relabel exit 2 as
product success. Collector is optional Firefox component evidence, never a real
human/AT or Meari+CEF product browser. No sandbox-disabling option is supplied.

Topology controls use three distinct internal IPv4 bridges, no public ports,
root filesystem read-only, nonroot 65532, cap-drop ALL, no-new-privileges, no host
or Docker API mounts, bounded resources. A live origin positive control is
paired with direct origin-IP and cross-bridge canary negatives. Metadata and
gateway API negatives cover exact addresses/ports, not all host/IPv6 paths.
Inside-client absence of a default IPv4 route complements finite probes without
third-party traffic. Cleanup targets only the generated project's resources.
An external runner may call cleanup for owner+grammar+expiry-matched projects;
no scheduler is installed by this tool.

Actual runtime remains loopback-only. Its child, Rust client and counted owned
origin share a network-none container and sealed audit state. This proves the
bounded session/RIT/audit-before-origin/restart path, not cross-namespace L5
product topology, raw TLS/H2, browser collection or independent witness state.
No external signed target attestation endpoint exists. Wargame will use a
supervisor-owned child handle/private readiness mode only; remote/full-Gate
binding remains unsupported. `test/lab/runtime` consumes shared
session::canonical, not a copied signer. Attacker/defender API integration is a
separate sequential red-catalog lane stage.

Existing run.py/cleanup.py/audit.py, clients/*.py and authority/pilot.mjs are
historical foreign-language prototype debt. Canonical compose.rust.yaml and the
new runner never execute them. Retire them after old evidence consumers move;
their older successes cannot qualify new source. Node prototype is not the
current Rust adapter implementation.

First v2 checkpoint: Rust runner unit4 and Clippy passed; actual Docker
topology-only25 assertions passed, including live cross-namespace direct-IP
negatives and absence of a default IPv4 route. `--require-full` exits2. This is
not a runtime pass: components.runtime is explicitly not_run. The first default
runtime snapshot failed because a concurrently authored audit service module
was declared before its source file existed; that failure is retained. Fresh
runtime source, collector and independent witness must be executed separately.
The topology controls image and runtime process image are separate targets/tags;
the controls image contains only the Rust runner. Do not reuse old CVE results
for these new runtime images; supply a fresh source/image/database-bound scan.

Provider preflight is local, non-mutating and always exits 2: explicit opt-in,
owned exact hostname, bounded budget/expiry, cancellation janitor, qualified Gate
source, credentials presence, and Cloudflare Access/domain constraints are
necessary, not sufficient. No cloud API, paid resource or public Tunnel is
created. Cloudflare uses cf CLI when no Wrangler configuration exists.
