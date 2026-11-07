# Same-workflow local and GitHub comparison

## Executed configuration

Both executions passed all six requested suite records. Control source is
d610ab4dfeb2de6173091e13b0b54af5a7d878e3; workload source is
8132117e7bf1445939d7dee9066fcf45ccf0915b. The workflow is
`.github/workflows/local-parity.yml`, job `compare`, with the same dependency-free
Rust measurement driver in both venues.

- Local: verified act 0.2.97 Windows binary, Windows Docker Desktop desktop-linux.
- GitHub: [run 42360069900](https://github.com/modootoday/humanymous/actions/runs/42360069900),
  job 128366376073, completed success.
- Identical container digest:
  ghcr.io/catthehacker/ubuntu@sha256:62d572b92f9f32d3427b6d220ad1f9dca9c7b6ffad37d295425037dbff78abaf.
- Both actual cgroups: CPU quota 400000/100000, memory 2147483648 bytes,
  additional swap zero; UID zero. Cargo jobs one and incremental disabled.
- Rust/cargo versions, workload lock SHA-256 and GNU time package version match.
  The installed time package is 1.9-0.2build1; preparation is timed separately.
- No real GitHub token was supplied to act, no package was published, no OIDC
  exchange occurred, and no raw log artifact was uploaded.

## Wall time, seconds

| Suite | Local initial | GitHub initial | Local repeat | GitHub repeat |
|---|---:|---:|---:|---:|
| Collector library, 26 tests | 40.546 | 35.410 | 0.092 | 0.496 |
| Workspace all-target build | 100.332 | 157.570 | 3.572 | 6.022 |
| Actual runtime child-process test, one test | 17.536 | 31.047 | 4.743 | 9.881 |

Every test record reports zero failed, ignored, measured or filtered cases.
Build records have no test denominator. All inputs remained unchanged.
Local outer act elapsed time was 211.291 seconds. The GitHub job lasted 294
seconds; its run was created at 09:31:09 UTC and the job started at 09:31:12,
so its observed initial queue interval was three seconds. Whole-job timing also
includes different control-checkout and executor startup paths, not just Rust.

The initial three workload times sum to 158.414 local versus 224.027 GitHub
seconds. This single observation is about 29.3% shorter locally. Collector's
initial local run was slower, so the evidence does not support an unconditional
local-speedup claim. The second pass observes a previously used target; earlier
suites also warm later suites in the initial pass. Neither fully cold caches nor
bit-level cache reuse are certified. Do not combine initial/repeat observations
into a statistical median or claim stable performance from one paired job.

## Reported resource scope

| Initial suite | Local GNU max RSS, MiB | GitHub GNU max RSS, MiB |
|---|---:|---:|
| Collector | 332.7 | 370.2 |
| Workspace build | 485.5 | 520.4 |
| Runtime process | 445.9 | 480.6 |

GNU time maximum RSS is not aggregate simultaneous child memory, a container
peak or a Windows host commit figure. The driver's cgroup context was sampled
before workloads, so its memory.peak is not a final whole-job peak. A separate
live Docker sample observed the local container at 1.385 GiB, which is a sample,
not its peak. The host controller retained at least 10.302 GiB commit headroom
and did not abort. The benchmark container was removed; the pre-existing user
container remained running.

CPU quota matches, but nproc reports 32 locally and four on GitHub. CPU model,
kernel, storage and network differ. These measurements therefore compare actual
developer turnaround under declared limits; they do not isolate emulator
overhead or prove VM/hardware equivalence.

## Separate direct native baseline

The same old workload source was also measured without the workflow emulator:

| Environment/command | Seconds | Result and cache boundary |
|---|---:|---|
| Windows workspace all-target build, original PATH | 86.174 | Failed: CMake unavailable on PATH, not a Rust unit assertion |
| Windows same build, bundled CMake/Ninja on child PATH | 133.755 | Passed; existing registry and prepopulated target |
| Windows immediate same build | 0.634 | Passed; reused target |
| WSL native Linux all-target build | 126.130 | Passed; new Linux-native target, fresh toolchain/cache profile |
| Windows collector 26 library tests | 3.042 | Passed; prebuilt dependency/target state |
| Windows actual runtime process test | 17.405 | Passed; private fixture directory only, no service/account provisioning |

The original GitHub native build steps took 986 seconds on Windows and 206 on
Linux. They did not share the local cache starting state. Windows also compiles
native CEF contexts absent from Linux's default target, so these direct figures
cannot establish a universal OS or executor speed ratio. Windows measured
aggregate compiler working/private samples while Linux used GNU max RSS; those
two memory measures must not be compared as the same quantity.

The concrete unit-versus-build failure was environmental: Visual Studio already
contained CMake 3.31.6-msvc6 and Ninja 1.12.1, but their directories were absent
from PATH. A child-scoped PATH correction made the same source build pass.
Windows Docker similarly needed its bundle directory for the credential helper.
Preflight must check native tools/helpers before costly build work.

## Applied execution design

Use native Windows and WSL/Linux for supported, explicitly allocated compiler,
unit and real process/IPC checks. Use Windows Docker Desktop plus this same YAML
for repeatable selected Linux integration before pushing. Keep real GitHub runs
for unavailable OS/architecture tuples, genuine runner/event/permission behavior,
explicitly hosted-only contracts, and final integration evidence. Local ACT
environment variables never establish hosted provenance.

Maintain distinct OS/architecture/toolchain/lock/feature/profile targets. Preserve
safe dependency/compiler caches for iteration, run all-target and required
feature builds as well as library units, and serialize heavy CEF/native SDK
builds. The observed 2 GiB container budget applies to this pinned Linux profile,
not arbitrary CEF, NativeAOT, model or full-product workloads. Model initialization
remains separately bounded and held after its earlier memory-guard failures.

The benchmark quiet window is complete. Root resumes four explicitly assigned
bounded validation pipelines; idle slots are not self-authorization. General
resource guards remain in force. Final full-product gates are not waived by any
of these component successes. This environment is usable, but code changes after
the measured workload require their own current-source evidence.
