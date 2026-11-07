# Browser GPU boundary research

Research and proposed contracts only, reviewed2026-11-08 KST. This document
derives from browser-ui UI-37/38/39. Root owns project-wide selection policy,
dependency/pin decisions and implementation admission. Performance owns its
comparison matrix. No backend switch, GPU work, driver/SDK installation, new
dependency, authored shader or unsafe exception is authorized by this research.
Existing CPU experiments continue unchanged.

## Actual baseline and independent owners

Meari1482b746ebc0d867f25d17d01c48c12df888ca6c window preparation/driver and
checked-window types select VelloCpuWindowRenderer. The selected softbuffer
path rasterizes and converts pixels on CPU. Optional offscreen GlHost does not
replace that window driver. A real GPU texture rendered and read back to CPU
for softbuffer is not the proposed direct GPU-window presentation path.

CEF156.0.15/Chromium156.0.8078.80 is a separate windowed ALLOY child. Desktop
uses set_as_child, not windowless/shared-texture rendering, and no_sandbox=0.
Browser GPU-process presence in the ce24 manual report does not establish which
CEF graphics features used hardware. No current chrome://gpu feature-status
observation qualifies that manual run. Do not change CEF flags to make Meari
appear accelerated. CEF sandbox, resource guards and external-navigation locks
remain independent of the shell backend.
Do not convert the CEF child to offscreen/shared-texture mode to solve shell
responsiveness. Such a cross-process surface change needs its own reviewed
rendering/sandbox/ownership design and would not inherit the windowed baseline.

The human reports Meari-only150-200ms-plus trailing resize while CEF content seems
fine. It is an estimate, not a benchmark. ce24 was dev/unoptimized. UI38 f9a7fe92
is an unexecuted viewport-source-cache elision candidate, not measured native
event coalescing, a qualified event-backlog bound, acceleration or latency repair.
Await the exact released Meari tag/commit and reviewed public renderer API before
root considers a pin change. Do not edit the immutable Cargo git cache.

## Proposed selection boundary

These are conceptual per-UI modes, not shipped public enum/schema names or a new
default. Root reconciles them with cross-cutting policy before implementation.
Only trusted host configuration may select a mode/budget; target documents, URLs
and untrusted renderer messages must not control GPU admission. Engine-private
adapter/backend observations do not become website-visible evidence or scoring
signals merely because the qualification browser can read them.

| Requested UI mode | Proposed eligibility/result |
|---|---|
| CPU only | Instantiate the CPU UI renderer without requesting a UI GPU device. CEF's separate policy is unchanged. |
| Auto, hardware preferred | Use a qualified hardware/window tuple with measured responsiveness benefit and bounded memory/queue cost; otherwise retain CPU with a visible reason. Choose per lifecycle, not by oscillating every frame. |
| Require hardware UI | Refuse initialization if hardware backing, compatible surface, features/limits, safe lifecycle or qualified tuple is absent/unknown. No silent CPU/software-adapter success. |

Correctness, source/input/AX/native-child lifetime and required sandbox outrank
throughput. Then compare responsiveness, total transfer/queue cost, power and
memory. Integrated GPUs may be suitable; discrete GPUs are not automatically
faster for a small shell. Power preference is a hint, not hardware proof.

Public UI status should identify the UI scope and selected CPU/hardware mode,
with a bounded fallback reason such as unavailable, incompatible surface,
resource budget, device loss or unqualified tuple. It must not expose raw device
IDs, paths or user input. Keep raw adapter diagnostics private. A required-mode
refusal is a renderer initialization result, never an automation verdict or a
reason to weaken accessibility or fail-open enforcement.

DeviceType::Cpu means software; VirtualGpu/Other remain unknown hardware backing
unless separately qualified. IntegratedGpu/DiscreteGpu metadata is useful
classification, not an anti-spoof or security attestation. Reject Noop/test-only
backends as real hardware evidence. force_fallback_adapter=false does not prove
the selected adapter is hardware. Inspect the actual selected adapter and its
surface capabilities; a vendor/name string alone is insufficient. WARP remains
software even though it implements Direct3D. Headless or locked desktop absence
is not repaired merely by selecting CPU.

## Upstream capability versus project qualification

The API column comes from tagged wgpu v30.0.1, not a selected dependency. Its
workspace manifest declares Rust1.93/edition2021, while the wgpu package manifest
overrides its minimum to1.87.0. Both declared floors are below the mandated
Rust1.99.1 compiler; no transitive build/link/ABI qualification was performed.
Architecture and vendor combinations below are pilot candidates,
not a published support list.

| OS/architecture candidate | Graphics API eligibility | Vendor/runtime boundary | Current project evidence |
|---|---|---|---|
| Windows x64 | D3D12/Vulkan; GL best effort | AMD/Intel/NVIDIA hardware requires actual driver/surface qualification; WARP/software is separate | Actual CPU Meari/windowed CEF baseline only; new GPU UI untested |
| Windows ARM64 | Same API families are candidates | Qualcomm/other adapters require matching native packages and drivers | Current Windows-x64 CEF tuple does not qualify ARM64 |
| Linux x64/ARM64 | Vulkan; GL/GLES best effort | NVIDIA/Mesa/other stacks, Wayland/X11 and software ICDs are separate tuples | Current Browser desktop implementation is Windows-gated; no Linux native UI support claim |
| macOS x64/ARM64 | Metal; Vulkan via MoltenVK; GL via translation constraints | Apple/Intel/AMD availability depends on actual OS/device/driver | No Browser native macOS implementation/pilot qualification |
| Browser wasm | WebGPU or WebGL2 best effort | Browser adapter permissions/capabilities and software fallback differ | Not Meari native HWND/CEF child integration |
| VM/RDP/server/headless | API availability is not hardware proof | Virtual/unknown/software adapters and missing display surface need distinct results | No hardware or interactive native qualification inferred from hosted CI |

Android/iOS availability in upstream wgpu is not a mobile Browser SDK design or
grant. Vendor-family rows do not certify particular drivers, minimum OS versions,
translation layers or packaging. Freeze those exact values per approved pilot.

## Interface and lifecycle proposal

Expose renderer selection through Meari's single-use NativeWindowPreparation /
NativeWindowDriver owner and public window-renderer abstraction. Browser passes
typed preference and budgets and consumes a capability/selection observation;
it must not create another competing window loop or transplant a child lease.
Retain existing CPU behavior for compatibility until a separately reviewed mode
is chosen. Any optional feature and upstream Rust change require a root-approved
version-exact isolated proposal. No authored shader/foreign implementation or
new unsafe surface-handle exception follows from an upstream GPU backend existing.

One current viewport generation and one latest pending resize are proposed.
A candidate GPU budget allows at most two in-flight frames, subject to the final
backend's capabilities and measured profile; this is not a current allocation.
Swapchain latency preferences are not proof of an OS queue bound: enforce and
observe the owner's in-flight resources separately and leave display queue depth
unknown unless measured.
Drop obsolete unsubmitted work, not resources still used by submitted GPU work.
Keep textures, buffers, descriptors and source/context epochs alive through
observed completion. Timeout is not cancellation or proof of GPU drain. No
indefinite per-frame wait may block the UI thread. A separate bounded cleanup
deadline must retain unresolved ownership rather than claim release.

Check target dimensions, sample count, row/texture alignment, buffer/texture limits,
atlas growth and all temporary/in-flight allocations with checked arithmetic
against configured host/device byte budgets. The existing16384 geometry maximum
does not grant a maximum-size allocation. Suspend zero-size/minimized/occluded
presentation, retain the logical owner and resume from a fresh current extent.
Resolve outstanding surface textures before reconfiguration; never configure
while an acquired frame is still alive. Handle timeout, outdated/lost surface,
validation and device loss separately, with bounded retry and explicit failure.
Resource accounting continues during cancellation and recovery.
Own-UI allocation bounds do not bound CEF's separate GPU-process/renderer memory.
Record concurrent CEF workload and total resource pressure when comparing modes;
unknown shared VRAM/driver allocation is not counted as zero.
On unified-memory devices, host and GPU allocations compete for shared capacity
and bandwidth. Join ownership/accounting so aliased observations are not counted
twice, while genuinely separate staging/texture allocations still count. Opaque
shared-driver accounting remains unknown; UMA is not proof of free transfer.

GPU initialization/device loss must preserve the same parent HWND and CEF child
lifetimes, hit testing, focus, IME candidate placement, AccessKit tree/actions,
DPI and maximize/restore/close order. Revalidate child clipping/z-order and avoid
painting over the native child. Auto recovery may use CPU only if that transition
is proven safe; otherwise close/restart through the existing lifecycle. Required
hardware mode must report loss/refusal rather than silently relabel CPU. Changing
backend must invalidate stale prepared frames and retained presentation proofs.

## Cost, parity and evidence gates

Compare optimized CPU control, UI38 viewport-source-cache elision and a true GPU-window
candidate independently. Include layout/source preparation, raster, RGBA format
conversion/upload/readback, command submission, shader/pipeline cold start,
swapchain acquisition, queue age, completion and presentation-call costs. A GPU
can lose on small work or copy/queue overhead. Warm throughput is insufficient.
Handler return, last submitted frame, GPU completion and actual displayed frame
are different observations. Existing UI38 timings do not cover all of them.

Graphics parity needs frozen fonts (the historical pilot used host-font fallback),
viewport/DPI, color space, alpha/premultiplication, clipping, text shaping,
antialiasing, transforms and hit geometry. Define reviewed image tolerances and
accessibility/action invariants; do not promise byte-identical CPU/GPU pixels.
Do not move scoring, thresholds, crypto or numeric authority into a UI shader or
GPU compute path. Detection freeze and deterministic canonical CPU semantics
remain unchanged. Optional graphics must not affect verdict bands.

| Gate | Required proof | Current status |
|---|---|---|
| Pure selection/lifecycle | CPU skips GPU initialization; fallback/require refusals; budgets; generation/drop/loss/cancel state tests | Proposed, no new tests executed |
| Native interface/build | Exact Meari release, API/feature/lock closure, Rust1.99.1 and target build, unsafe boundary review if needed | New release/API unqualified |
| Actual hardware pilot | Real adapter classification, compatible window surface, bounded in-flight work, cold/warm resize and device-loss recovery | Not run |
| Native integration | CEF child, AX/keyboard/IME, multi-DPI/refresh, minimize/restore/close and resource cleanup | GPU path not run; prior CPU observations retain their limited scope |
| Product e2e | Owned Docker product gates plus required actual OS/hardware pilots | No GPU result; Docker/software rendering cannot replace hardware or OS proof |

Preserve exact failed, unsupported, missing and software-adapter denominators.
Keep current CPU experiments unchanged. Do not turn missing display or backend
telemetry into zero latency, hardware success, BrowserReady or NativeFull.

## Primary sources and limitations

- [wgpu tagged v30.0.1 platform table](https://raw.githubusercontent.com/gfx-rs/wgpu/v30.0.1/README.md)
- [wgpu tagged v30.0.1 manifest](https://raw.githubusercontent.com/gfx-rs/wgpu/v30.0.1/Cargo.toml)
- [wgpu tagged v30.0.1 package manifest](https://raw.githubusercontent.com/gfx-rs/wgpu/v30.0.1/wgpu/Cargo.toml) overrides the workspace Rust minimum.
- [wgpu30.0.1 adapter options](https://docs.rs/wgpu/30.0.1/wgpu/type.RequestAdapterOptions.html), [device types](https://docs.rs/wgpu/30.0.1/wgpu/enum.DeviceType.html), [adapter information](https://docs.rs/wgpu/30.0.1/wgpu/struct.AdapterInfo.html), [power preference](https://docs.rs/wgpu/30.0.1/wgpu/enum.PowerPreference.html)
- [wgpu30.0.1 surfaces](https://docs.rs/wgpu/30.0.1/wgpu/struct.Surface.html), [acquisition outcomes](https://docs.rs/wgpu/30.0.1/wgpu/enum.CurrentSurfaceTexture.html), [polling](https://docs.rs/wgpu/30.0.1/wgpu/type.PollType.html), [device loss callback](https://docs.rs/wgpu/30.0.1/wgpu/struct.Device.html)
- [Microsoft WARP guide](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/directx-warp) is official living documentation, not a frozen driver/runtime certification.
- Meari1482 pinned Cargo.toml, window/driver.rs, window/preparation.rs and verification/checked_window.rs are the current local primary source; cef-rs224a785/CEF156 pinned SDK headers and Browser source establish the separate windowed child/sandbox configuration.
- [AnyRender GPU main source](https://raw.githubusercontent.com/DioxusLabs/anyrender/main/crates/anyrender_vello/src/window_renderer.rs) was reviewed as mutable candidate evidence only. It currently calls wait_indefinitely per frame. It is not a released/pinned compatibility choice; removing that wait alone is not a resource-lifetime fix.

Critique verdict: needs-work before implementation. Missing exact released Meari
GPU API/pin, bounded completion implementation, actual hardware/runtime matrix,
image/AX parity and end-to-end display measurements prevent a support or speedup
claim. Research completeness does not authorize any execution/backend change.
