# Same-source workflow measurements

Workflow: `.github/workflows/local-parity.yml`, job `compare`, UI name
`CI / Local parity`. Push is restricted to `codex/local-parity-benchmark`;
manual dispatch is also declared. Its concurrency group is separate from product
CI and never cancels an existing benchmark. No publication, OIDC exchange,
artifact upload or automatic execution-policy decision is included.

The control checkout contains this dependency-free Rust 1.99.1/edition 2024 tool.
The tool itself fetches the public workload commit into a fresh private Git
checkout. Default workload:
`8132117e7bf1445939d7dee9066fcf45ccf0915b`. It verifies control/workload HEADs,
clean source inputs, the workload lock hash and source stability. ACT must use
the new committed control checkout, not overwrite it with the old workload.
An inaccessible external `.git` worktree pointer fails verification; use an
actual accessible checkout. Do not pass a real GitHub token to ACT.

The authorized local Docker route is Windows `act.exe` using Docker Desktop's
Linux engine. The outer preflight uses
`C:/Program Files/Docker/Docker/resources/bin/docker.exe --context desktop-linux`
and records context, endpoint, server ID/version and OSType. Set the outer
`DOCKER_HOST` to `npipe:////./pipe/dockerDesktopLinuxEngine`, verify the actual
engine ID, then pass the same verified values into the benchmark container as
`HUMANYMOUS_BENCHMARK_LOCAL_PROVIDER=windows-act-desktop-linux`,
`HUMANYMOUS_BENCHMARK_DOCKER_HOST` and
`HUMANYMOUS_BENCHMARK_LOCAL_ENGINE_ID`. A context name alone does not select the
engine for ACT. The separate WSL-native Docker daemon is not a fallback. WSL
native Rust timings remain a separate comparison dataset. The Linux driver
records caller-provided engine identity and does not pretend to attest a Windows
named-pipe daemon from inside a container. Missing canonical local provenance
fails before workloads. Hosted Linux runners remain a distinct context.

Both venues use the same declared image:
`ghcr.io/catthehacker/ubuntu@sha256:62d572b92f9f32d3427b6d220ad1f9dca9c7b6ffad37d295425037dbff78abaf`.
Requested limits are four CPUs, 2 GiB memory and 2 GiB total memory plus swap
(no additional swap). The driver checks actual cgroup limits before preparation
or workloads; missing/unbounded limits fail instead of silently running wider.
The outer operator must still monitor host commit headroom. Cgroup values and
image requests do not prove VM identity or equivalent host hardware.

The pinned Rust action must bootstrap rustup because the image does not contain
it. The Rust driver runs the same explicit `apt-get update` and
`apt-get install -y --no-install-recommends time` in both venues, retaining setup
durations, exit codes and observed package version separately. Missing GNU time
or missing CPU/RSS metrics fail measurement; they are not recorded as zero.

The one job runs the literal command:

```text
cargo run --locked --manifest-path scripts/ci/local-parity/Cargo.toml --
```

Inputs are environment data, never shell source:

| Variable | Meaning |
| --- | --- |
| `HUMANYMOUS_BENCHMARK_SOURCE` | Exact 40-character lowercase workload commit |
| `HUMANYMOUS_BENCHMARK_CONTROL_SHA` | Exact control checkout commit |
| `HUMANYMOUS_BENCHMARK_PASSES` | `1` initial pass or `2` initial and same-job repeat |
| `HUMANYMOUS_BENCHMARK_IMAGE` | Declared container digest, recorded without an in-process image attestation claim |

Each pass runs these workloads sequentially with build jobs one and incremental
compilation disabled:

```text
cargo test --locked -p humanymous-collector --lib -- --test-threads=1
cargo build --locked --workspace --all-targets
cargo test --locked -p humanymous-runtime --test native_process -- --test-threads=1
```

The pinned source requires 26 collector tests and one native process test, with
zero ignored/filtered cases. The build is not a test denominator. Failures are
retained and later workloads are still attempted; changed source stops further
execution. Each completed workload emits machine JSON and a final report records
wall time, GNU time CPU/max-RSS values, actual nproc/UID/cgroup context, command
exit, observed counts and private stdout/stderr/resource-log SHA256 values.
GNU max RSS is not aggregate whole-machine or simultaneous child working set.
Raw logs remain runner-private and are not uploaded. ACT context is explicitly
local, never relabeled GitHub-hosted.

The same-job repeat observes a warmed target, but prior workloads affect later
ones even in the initial pass. `target_existed_before` is recorded;
`cold_cache_verified=false` and `cache_reuse_qualified=false` remain explicit.
Compare equivalent successful source/workload results and setup versions before
deriving any performance policy. This new tool is source-only until checked and
executed under the root's explicit resource allocation.

Started-phase records precede each workload so a container OOM or operator stop
does not masquerade as completed measurement. A killed container may have no
final report; retain the outer failure/exit and never invent missing metrics.
