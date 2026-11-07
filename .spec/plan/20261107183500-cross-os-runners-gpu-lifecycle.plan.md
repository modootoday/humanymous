---
id: plan:root/20261107183500
title: Native GPU capability observations and retained execution lifecycle
status: draft
created: 2026-11-07T18:35:00Z
updated: 2026-11-07T18:35:00Z
rel:
  depends_on: [sot:root/cross-os-runners, sot:root/rust-migration, sot:root/structural-development]
---

# Native GPU resource contract proposal v0.1

Root assigned research and this shared native capability/lifecycle interface.
Cross-cutting CPU/GPU priority and graph decisions belong to structural canon.
This is NOT an accepted backend, dependency, shader or execution decision.
Rust1.99.1/detection freeze/CPU experiment source remain unchanged. No GPU load,
context/device allocation, driver/SDK install, model switch, authored shader,
FFI/unsafe source or ABI v1 reserved-field reuse is authorized here.

## Observed baseline and eligibility

Current root Cargo.toml/lock searches found no wgpu/wgpu-core/wgpu-hal/naga/ash/
metal/CUDA package. This is not proof that browser/driver processes never use GPU.
One read-only Win32_VideoController CIM query reported one NVIDIA controller.
Exact adapter/driver metadata is private and hash-registered.
It is NOT a loaded backend, compute feature/VRAM budget/physical-attestation or
performance observation. AdapterRAM is uint32 and cannot represent an arbitrary
large VRAM capacity; do not turn it into a reservation budget. Microsoft also
documents inaccurate WMI values on non-WDDM hardware [S10].

Initial eligibility proposals, not activation:

| Work | Proposed status | Reason/required owner decision |
| --- | --- | --- |
| Admission/ACL/token/SCM, erasure/kill/ban policy, audit ordering | CPU-only boundary | GPU enumeration grants no principal/provider/NativeProof; state transitions remain owned CPU contracts. |
| Frozen detection/scoring/verdicts and canonical crypto/serialization | No GPU migration in this slice | Parallel order/float/encoding changes cannot silently change frozen results; explicit owner/spec/parity decision first. |
| Large independent nonnormative batch analysis or approved inference | Research candidate only | Exact kernel/model/data/type profile, privacy, end-to-end measurements and parity contract required. Existing CPU runs finish unchanged. |
| Cargo/build/docs/SCM/process orchestration, tiny branchy requests | CPU default | No proposed generic speedup; overhead/correctness evidence would be required per actual workload. |
| Browser rendering | Existing browser-owned path | A working renderer says nothing about native compute or this resource owner; no backend flags changed. |

## Primary source snapshot (retrieved 2026-11-08 KST)

Versioned wgpu30.0.1 is a RESEARCH REFERENCE ONLY, not a proposed pin. Its API
documentation lists Rust MSRV1.87, below project1.99.1, but this is not a compiler/
dependency/OS support test. Native backend support is conditional on actual driver,
target, features and limits [S1]. Tagged release is v30.0.1/40f4a34 short commit;
the release and full commit
[40f4a34ebaf56f9a046231f54125ad046239d3f3](https://github.com/gfx-rs/wgpu/commit/40f4a34ebaf56f9a046231f54125ad046239d3f3)
were readable, direct tag-tree fetch was restricted [S2].

| ID | Primary source | Narrow implication |
| --- | --- | --- |
| S1 | [wgpu30.0.1 crate](https://docs.rs/wgpu/30.0.1/wgpu/) | Cross-platform backend/API candidates and feature configuration; no pin/compatibility promotion. |
| S2 | [tagged v30.0.1 release](https://github.com/gfx-rs/wgpu/releases/tag/v30.0.1) | Versioned reference; not an implementation allocation. |
| S3 | [Queue30.0.1](https://docs.rs/wgpu/30.0.1/wgpu/struct.Queue.html) | Submission and submitted-work callback are separate; callback progress needs the backend's polling mechanism. |
| S4 | [Device30.0.1](https://docs.rs/wgpu/30.0.1/wgpu/struct.Device.html) | Poll may block in Wait mode; device-loss notification exists. Shared queue access can invalidate an empty observation. |
| S5 | [DeviceType30.0.1](https://docs.rs/wgpu/30.0.1/wgpu/enum.DeviceType.html), [AdapterInfo](https://docs.rs/wgpu/30.0.1/wgpu/struct.AdapterInfo.html) | Backend-reported CPU/integrated/discrete/virtual/other classifications are observations, not attestation. |
| S6 | [DownlevelFlags](https://docs.rs/wgpu/30.0.1/wgpu/struct.DownlevelFlags.html), [Limits](https://docs.rs/wgpu/30.0.1/wgpu/struct.Limits.html) | Compute capability, workgroup/storage/buffer limits must be checked, not inferred from graphics success. |
| S7 | [Microsoft WARP](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/directx-warp), [feature levels](https://learn.microsoft.com/en-us/windows/win32/direct3d12/hardware-feature-levels) | Software D3D is distinct from hardware; feature functionality is not a speed guarantee. |
| S8 | [Vulkan devices/queues](https://docs.vulkan.org/spec/latest/chapters/devsandqueues.html) | Physical/logical devices, queue submission and lost-device handling are distinct; finite return is not a project millisecond bound. |
| S9 | [Apple command-buffer status](https://developer.apple.com/documentation/metal/mtlcommandbuffer/status), [blocking wait](https://developer.apple.com/documentation/metal/mtlcommandbuffer/waituntilcompleted%28%29), [memory recommendation](https://developer.apple.com/documentation/metal/mtldevice/recommendedmaxworkingsetsize) | Scheduled/completed/error differ; blocking waits exist; recommended working set is approximate, not reserved capacity. |
| S10 | [Win32_VideoController](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-videocontroller) | Read-only OS inventory and its precision caveats. |
| S11 | [WGSL floating-point evaluation](https://gpuweb.github.io/gpuweb/wgsl/#floating-point-evaluation) | Allowed numeric behavior needs an explicit parity policy; CPU bit identity is not automatic. |
| S12 | [GitHub GPU runner specifications](https://docs.github.com/en/actions/reference/runners/larger-runners), [hosted runners](https://docs.github.com/en/actions/concepts/runners/github-hosted-runners) | GPU venues are separately provisioned; an ordinary OS runner label is not actual hardware evidence or a free-resource grant. |
| S13 | [NVIDIA CUDA compatibility](https://docs.nvidia.com/deploy/cuda-compatibility/latest/) | CUDA driver/toolkit/GPU compatibility is its own matrix, not implied by generic GPU presence. |

Mutable vendor/spec URLs require a fresh version/driver/spec snapshot before any
implementation/pilot; do not combine current docs with an unrecorded old runtime.
No third-party tutorial or benchmark supplies a compatibility/authority claim.

## OS / architecture / vendor / venue matrix

All entries below are CANDIDATE TEST TUPLES, currently unqualified. No backend
enumeration/device/dispatch was executed. Vendor name never bypasses required
feature/limit checks or Root reservation. x64 means x86_64; ARM64 means aarch64.

| Host tuple | Candidate backend/vendor | Required observation and residual |
| --- | --- | --- |
| Windows x64 | D3D12 or Vulkan; NVIDIA/AMD/Intel as actually available | Actual loader/driver/device creation, compute flag and requested limits; distinguish WARP/software/virtual/unknown. CIM model name alone insufficient. |
| Windows ARM64 | D3D12/Vulkan only with matching runtime/driver | Target build, user-mode driver and real adapter require separate evidence; x64 success/emulation is not native ARM64 success. |
| Linux x64/ARM64, headless | Vulkan with installed matching ICD and authorized device access | Compute may be surface-free; no display is not automatically no GPU. No /dev chmod, device mount, new group/driver or container privilege changes inferred. Software ICD remains software. |
| macOS ARM64 | Metal on Apple device families | Actual family/limits/OS deployment profile; shared CPU/GPU memory must not be counted as two independent pools. |
| macOS x64 | Metal on actual supported Intel/AMD system | Separate family/driver/OS qualification. Apple Silicon evidence does not cover it. Vulkan portability is an additional layer/profile, not default native Metal equivalence. |
| Native GL/GLES fallback | Only an explicitly qualified compute-capable context | A graphics/WebGL2 baseline does not prove compute; headless context setup and downlevel flags need independent checks. Not an automatic compatibility expansion. |
| Browser wasm32 | WebGPU under actual browser availability/security policy | Separate event-loop/permission/loss owner; native poll/worker contracts cannot be cast onto it. WebGL2 is not a compute fallback. No WebGPU fingerprint used as human/native identity. |
| CUDA specialist | NVIDIA plus exact supported OS/architecture/toolkit/driver | Optional specialist decision by workload owner; not a generic AMD/Intel/Metal fallback and not installed/selected here. |
| CI/container/VM | Root-admitted exact hardware or explicitly software venue | Record actual adapter class/backend/driver, visibility/permissions and image. Build, Noop, virtual or software success is not hardware acceleration evidence. |

## Interface sketch: observation, admission and custody are separate

Provisional internal Rust concepts; no new crate/path/manifest/API implementation
or stable ABI is approved. GPU lifecycle belongs to a resource owner, not token/
ACL privilege ownership. Native-security/Authority must not issue an admission
grant from these records. ABI v1 layout/reserved fields remain untouched.

```rust
// Sketch only, no implemented/exported GPU capability.
enum ComputePreference { CpuOnly, PreferMeasuredGpu, RequireGpu }
struct AdapterObservation { /* bounded redacted backend/driver/type/features/limits */ }
struct GpuReservation { /* opaque coordinator lease, device-generation scoped */ }
struct GpuAttempt { /* SAME device/queue/buffers/maps/callbacks/poller custody */ }
enum SubmitState { NotSubmittedKnown, SubmittingUnknown, Submitted, Retiring }
enum PollState { Pending, Completed, CancelRequested, DeviceLost, Failed }
// settle(self) -> SettledGpuAttempt OR RetainedGpuFailure containing SAME owner.
```

Observed class is explicitly HardwareReported, SoftwareReported, VirtualReported
or Unknown, with backend provenance and bounded fields. It is NOT hardware
attestation. RequireGpu refuses software/Noop/unknown if hardware is required by
the accepted profile; virtual passthrough needs an explicit separate profile.
Never relabel software execution as a successful hardware fallback.

Observed limits and supported features are not granted/reserved resources. Exact
workload requirements include compute support, requested buffer/storage/workgroup
limits, alignment, data types and any optional subgroup/precision feature. Check
before allocation and freeze requested features/limits in the receipt. Unsupported
is explicit and gives no new driver/SDK installation or backend-switch authority.

Root coordinator admission precedes backend effects. Reservation fixes positive
finite host/device/staging/readback/callback/queue/in-flight and time budgets,
checked aggregate arithmetic, workload/source/input commitments and device epoch.
Do not use total VRAM, WMI AdapterRAM or a recommended working set as an enforceable
free-memory reservation. Unknown accounting requires conservative refusal or a
separately accepted bounded probe, not optimistic allocation. Integrated/unified
memory and concurrent GUI/model consumers share system resources. Cross-backend
aliases of the same reported device must not obtain duplicate independent leases.

## Retained submission / cancellation / loss / drain rules

1. Establish ONE original run/cleanup pair before any backend startup/allocation.
   Backend/driver initialization, pipeline creation and waits may block; require
   separately owned supervision. No universal syscall or preemption bound.
2. Keep backend handles private: wgpu handles are cloneable [S1], so a wrapper
   cannot prove exclusive drain if queues/resources/callback senders escape.
   One submission authority, finite immutable command profile, monotonically
   scoped submission/generation IDs; stale callbacks cannot settle a new epoch.
3. Transition to SubmittingUnknown BEFORE a potentially effectful submit. An
   exception/error before receiving an ID is not proof that nothing was queued.
   Upload staging, accepted submission, device completion, readback visibility,
   output validation and external application commit remain different facts.
4. Poll gathers bounded ready observations without Wait/hidden joins, dynamic
   error growth or payload loss. Dedicated poller/callback/driver worker handles
   remain retained. Terminal/error/loss state has an independent fixed path; it
   cannot be lost behind full result payload queues. No user authority callback.
5. Cancel prevents new submits and requests best-effort retirement. It is NOT GPU
   preemption or successful drain. A dropped future/timeout/last public handle,
   callback notification or host process exit alone is not completion proof.
6. DeviceLost is sticky and forbids further use/submission in that epoch. It is
   an error observation, not success, zero work or implicit safe recreation. No
   driver reset/TDR change, elevated repair or automatic new-device lease.
7. Final settlement records every accepted/in-flight ID and terminal disposition,
   maps unmapped/closed, buffers/device release acknowledgements available in the
   selected backend profile, all callback/poller workers actually complete+join,
   and lease release after actual known retirement. Missing backend guarantees
   remain Unknown; do not invent a portable release acknowledgement.
8. A failed/expired/pending finalization returns the SAME owner and reservation.
   Error/Drop must not silently detach workers or release quota while work may
   remain. Recovery-owned supervisor must be designed before any actual pilot;
   ordinary Rust caller abandonment remains Unknown. No unsafe Send workaround.

## CPU fallback and selection proposal for Root synthesis

CpuOnly performs no GPU probe/init. RequireGpu refuses absent/unsupported/denied/
unqualified adapter, software where hardware is required, lost device or missed
reservation; it NEVER silently executes CPU. PreferMeasuredGpu is eligible only
for an accepted workload/tuple with numerical parity and measured crossover; it
otherwise selects CPU before backend effects. No backend/model change mid-run.

Fallback to CPU after an error is permitted only at a KNOWN safe effect boundary:

| GPU state | CPU re-execution rule |
| --- | --- |
| Known no submission; no external commit; all created GPU resources known retired | Eligible only after reservation/owner settlement and remaining original budget check. |
| Submitted pure work, outputs quarantined | No concurrent replay by default. First retire known work/maps/callbacks and release the old lease; then exact pure CPU recomputation may be admitted within the SAME remaining budget. |
| Submission uncertain, device-loss cleanup unknown or external commit begun | No fallback/retry that could duplicate effects; preserve Unknown/fail closed unless an independently accepted idempotency/commit protocol resolves it. |

Fallback never renews run/cleanup or changes expected output semantics. GPU
completion does not commit output; CPU validation/consumer policy owns that step.
Detection freeze is unchanged: tolerances cannot justify different verdicts.

Measure end-to-end cold+warm wall time, p50/p95/p99 where sample counts support
them, throughput, queue contention, encode/pack, allocation/pipeline warmup,
host-to-device transfer, dispatch, synchronization, readback/unpack and actual
cleanup. Kernel-only timer, vendor theoretical bandwidth or GPU utilization is
not a speedup. Break-even batch/shape and shared-memory pressure are per tuple.
Floating reductions/order/FMA/subnormal/NaN/Inf/integer overflow/data layout and
optional f16/subgroup behavior need explicit CPU reference and differential
vectors. Bit-exact canonical/security decisions remain CPU-only until their
owner accepts a proof that preserves existing semantics, not a relaxed tolerance.

## Verification and residual gates

L1 (future Rust only): pure selection/require-GPU refusal, supported-limit checks,
checked budgets, no-submission versus uncertain submit, cancellation/late/stale
callback races, full terminal mailbox, lost device, failed partial allocation,
safe replay/commit exclusion, conserved deadlines and retained reservation.
Fake adapters/Noop test the model only, never count hardware passes.

L2 actual (separate allocation/implementation decision): each OS/architecture/
vendor/backend/driver tuple; real compute+readback versus CPU, software and missing
device negatives, queue/contention/memory caps, cancellation at each boundary,
actual completion and poller joins, device-loss behavior with a separately safe
fault-injection plan, process crash/recovery and source/tool/driver/input pins.
Never simulate a loss and report an actual driver reset/loss test. Actual physical
GPU inventory, throughput, numeric parity and deterministic release currently0.

L3 consumer: exact admitted owner, same failure custody, no ABI v1 mutation or
GPU/TokenUser coupling. L5 Docker product e2e remains Docker-only, requires an
explicit GPU-capable venue and own real GPU evidence; ordinary Docker/CI/CPU
component passes do not cover it. No new container mount/privilege grant here.

Single-author panel: Blue separates observations/leases/security; Red rejects
software-as-hardware, uncertain-submit replay and stale callbacks; SRE retains
all workers/quotas through loss; DPO keeps adapter/driver IDs private and caps
payload retention; Evaluator separates pure model/software/actual GPU/product.
Adversarial verdict NEEDS-WORK before implementation: backend/package/driver/
kernel language approval, exact resource caps, recovery supervisor, loss/release
guarantees and independent Root/X/consumer review remain open. This draft does
not close Native producer/SCM/host admission or pending interactive gates.
