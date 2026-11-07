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
cargo run --locked -p humanymous-lab-runner -- audit --evidence EXISTING_LOCAL_SPECFOLD_BUNDLE
cargo run --locked -p humanymous-lab-runner -- provider-preflight LOCAL_MANIFEST
```

Each run requires a fresh bundle. Existing source-manifest/lab-report is refused;
an existing audit-report is also refused. No retry may replace a failed result.
`run` builds from an immutable allowlisted source copy and uses unique owned
Docker project/image tags. Host code coordinates Docker only. All HTTP/negative
clients execute in hardened containers, without shell/script execution. Exact
files and hashes are source-manifest.json, resolved-compose.json,
runtime-image.json and per-service/network inspect JSON. Docker CLI output is
registered as private docker-transcript.json. New Dockerfiles live in
deployments/lab/images; build/rust remains another owner's read-only surface.

Output lab-report.json schema `humanymous.product-lab/2` has evidence_types,
cases[{id,status,detail}], components.runtime (actual binary pilot report),
components.wargame, components.independent_witness, optional components.collector,
required_unavailable, full_product_qualified and
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

First v2 checkpoint4f89ec3: Rust runner unit4 and Clippy passed; actual Docker
topology-only25 assertions passed, including live cross-namespace direct-IP
negatives and absence of a default IPv4 route. `--require-full` exits2. This is
not a runtime pass: components.runtime is explicitly not_run. The first default
runtime snapshot failed because a concurrently authored audit service module
was declared before its source file existed; that failure is retained. Fresh
runtime source, collector and independent witness must be executed separately.
The topology controls image and runtime process image are separate targets/tags;
the controls image contains only the Rust runner. Do not reuse old CVE results
for these new runtime images; supply a fresh source/image/database-bound scan.

V3 integration consumes test-support26d9925/e30e0e9 bounded capture/duplex and
auditd001ee6 shared client/log/codec. Local process-lifetime duplication is gone.
Root27-member registration includes test/lab/runtime; its standalone lock is
retired. One Installation factory shares fresh synthetic sealed custody/config
and independently counted origin with the original process and wargame profiles.
The new wargame path delegates entirely to attacker1ee2c92 OwnedLab/defender APIs.
The origin counts contact before validation. Actual setup errors preserve all
26 cases as19errors/7unsupported, never a mock success or missing report.

Private source54017 remains unchanged:33orchestration assertions, actual runtime
origin2/audit4 and26scheduled=19evaluated/7unsupported,0errors/0mismatches.
Latest fresh source60627 passed71orchestration assertions plus that same complete
wargame matrix. It additionally executed separate UID65533 witness and UID65532
writer, four disjoint named key/head/WAL volumes, actual shared external Log
co-signing before two counted origin effects, same-identity/head container
recreation, authentic older-head refusal and populated key-volume EACCES from a
different UID. Witness control-plane unavailability is intentionally injected
after actual process shutdown; a typed broker Unavailable closes the shared Log
with origin0 and an ambiguous durable record. No signature is fabricated.

These are component results: no full Gate capture/browser/human/AT, no external
physical-host custody independence, no persistent cross-supervisor anchor and
no runtime-binary sidecar transport. New builder emits only static musl Rust
binaries in scratch images; collector remains its own Firefox/toolchain image.
Rust unit6/Clippy currently pass; the builder also runs Linux runtime-pilot unit
and Clippy. Fresh image/Cargo/secret scans after the final wargame corrections
remain required. Source60627 contains the code at its own immutable freeze,
not later report-only/case parsing changes.

Failure lineage:50621 concurrent audit module source gap;55200 first writer setup
failure had a retry-path evidence-reuse error. Its original dirty source graph
is lost; failure report/transcript/image are55537 failure-* only.55200 now binds
the separate retry source and its empty-attach failure.60036 retains the later
injected-EOF timeout failure.60627 is a new acknowledged-pipe/broker-unavailable
attempt, never retroactive success for any failed attempt. All raw/private files
must be Specfold-attached; public summaries omit startup secrets and raw bodies.

Provider preflight is local, non-mutating and always exits 2: explicit opt-in,
owned exact hostname, bounded budget/expiry, cancellation janitor, qualified Gate
source, credentials presence, and Cloudflare Access/domain constraints are
necessary, not sufficient. No cloud API, paid resource or public Tunnel is
created. Cloudflare uses cf CLI when no Wrangler configuration exists.
