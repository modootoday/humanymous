# Meari 0.3.0 release and Browser upgrade review

Review date: 2026-11-08 KST. Evaluate the official release against the current
`1482b746ebc0d867f25d17d01c48c12df888ca6c` pin. Preserve one Rust implementation,
CEF windowed-child ownership, required sandbox, accessibility and source-bound
qualification. This review changes no package/lock/source or renderer selection.
Root owns adoption and coherent execution; upstream results are separate evidence.

## Verified release identity

Official annotated tag `v0.3.0` has tag object
`52c612a1020b213f35f36e9741c1661ff999dc8d` and resolves to commit
`ef3d2d0a8acd2bc3c01ce40258d70a33c7e819a4`. The tagger timestamp is
2026-11-07T20:32:12Z (2026-11-08 05:32:12 KST); GitHub reports the tag unsigned.
The official comparison is six commits ahead of our pin and lists 46 changed
files. At observation, the GitHub tag-release endpoint returned 404 and the release
collection was empty. Therefore the verified publication is a tag/CHANGELOG plus
registry packages, not a separately observed GitHub Release announcement or asset.
[Tag object](https://api.github.com/repos/modootoday/meari/git/tags/52c612a1020b213f35f36e9741c1661ff999dc8d),
[comparison](https://github.com/modootoday/meari/compare/1482b746ebc0d867f25d17d01c48c12df888ca6c...v0.3.0).

The registry independently reports unyanked 0.3.0 packages for meari, meari-native,
meari-dom and meari-shell, published around 2026-11-08T03:53Z. Checksums are in the
attached JSON. Registry metadata alone does not prove tarball-to-tag byte equality,
publisher signatures, package installability or our distribution closure.
[meari-native registry record](https://crates.io/api/v1/crates/meari-native/0.3.0).

Twenty selected upstream text files were preserved privately and checked against
their official Git blob IDs. This is a bounded source review, not an exhaustive
dependency/security audit. Connector-added trailing newlines were removed before
hashing against the official blobs; raw test logs were not transformed or fetched.

## Compatibility and measured limits

| Boundary | Source-backed finding | Humanymous qualification |
|---|---|---|
| Default renderer | `window` remains CPU; GPU needs `window-gpu` and explicit choice. CPU resize coalescing/layout changes also ship. | CPU-first upgrade candidate; no local compile/pilot. |
| Driver API | Existing preparation/attach flow remains; `with_renderer`, renderer status and frame timing APIs are added. | Current Browser uses these existing driver APIs, not a concrete shell renderer type. Static source compatibility is plausible, uncompiled. |
| Breaking API | Native `meari-shell::View` gains private resize state, preventing downstream struct literals. | No direct shell View literal found in inspected Browser source. Browser's own tabs `View` is a different type. Broader workspace callers still need Root's check. |
| GPU rendering | Vello-to-texture-to-surface path has no CPU readback; one frame in flight, callback-held resources, nonblocking polling and explicit timeout/error. | No Windows hardware, driver-loss, surface or CEF-child proof. Surface calls may still block. |
| Rendering parity | GPU rejects filters/backdrop filters and custom/resource brushes. | Current typed SVG icons, fonts, focus rings, clipping and all generated scenes require real parity tests; absence of CSS filter strings is insufficient. |
| IME/AX/DPI | Pending size is flushed before input hit testing and AX actions; geometry observations are kept timely. | Ordering changes require real keyboard, measured Hanja, candidate placement/selection, AX and DPI regressions. |
| Timing | Stages distinguish event receipt/layout/raster/copy/surface/present return; no compositor display acknowledgement. | Keep displayed latency unknown. GPU encoding, queue completion and present return are different observations. |

The new public APIs and their boundaries are documented in the
[pinned renderer guide](https://github.com/modootoday/meari/blob/ef3d2d0a8acd2bc3c01ce40258d70a33c7e819a4/.spec/guide/native-window-renderer-selection-and-latency-probe.guide.md)
and [CHANGELOG](https://github.com/modootoday/meari/blob/ef3d2d0a8acd2bc3c01ce40258d70a33c7e819a4/CHANGELOG.md).

Upstream reports Linux x86_64 tests and synthetic present-return measurements.
Its hardware-GPU attempts all refused the CPU adapter; software-GPU runs had only
one presented resize sample and were not an improvement recommendation. Windows
CEF HWND/focus/IME/AccessKit/DPI/shutdown, Wayland, real device loss and display
visibility are explicitly unverified there. These reports neither resolve our
150–200 ms user-observed Meari lag nor qualify this Browser. Upstream test counts
and software-renderer observations are not added to our passing-test denominator.
[Pinned upstream evidence](https://github.com/modootoday/meari/blob/ef3d2d0a8acd2bc3c01ce40258d70a33c7e819a4/.spec/evidence/20261107174455-native-window-rendering-validation/index.evidence.md).

## Toolchain and distribution

The tag pins Rust 1.99.1 and edition 2024, matching our authored-code convention.
It selects wgpu **30.0.0**, Vello 0.11.0, anyrender_vello 0.15.0 and pollster 0.4.0
for the opt-in GPU path. This corrects the earlier speculative wgpu 30.0.1 research;
no such speculative version should be substituted for the actual release graph.
Registry-declared Rust minima are 1.87.0 for wgpu 30.0.0, 1.89 for Vello 0.11.0,
and 1.91.0 for meari-dom/meari-shell 0.3.0. Meari/meari-native registry records have
no declared minimum. These numbers are not a transitive MSRV build result.
[Toolchain](https://github.com/modootoday/meari/blob/ef3d2d0a8acd2bc3c01ce40258d70a33c7e819a4/rust-toolchain.toml),
[native manifest](https://github.com/modootoday/meari/blob/ef3d2d0a8acd2bc3c01ce40258d70a33c7e819a4/crates/meari-native/Cargo.toml).

The maintained shell is a patched fork of its recorded upstream baseline, with
updated provenance hashes; it is not an unmodified crates.io shell snapshot.
GPU dependencies, target-specific backend libraries, shader/compiler resources,
licenses and notices need a new package/SBOM/distribution inventory. CEF remains
independent: no engine update, shared-texture/OSR conversion, sandbox switch,
permission or external-navigation change follows from this upgrade.

## Recommended Root candidate and ordered gates

1. **CPU candidate first:** update all three merged Browser git dependencies
   (meari, meari-native, meari-dom) together to exact commit
   `ef3d2d0a8acd2bc3c01ce40258d70a33c7e819a4`; retain `window` and empty defaults.
   Use the immutable revision rather than a floating tag. Keep the registry source
   switch separate. Root must preserve the pre-upgrade graph/receipts and resolve
   the coherent lock; intentional upstream dependency changes are not zero-drift.
2. Run pure/default and native-ui document tests, changed-source formatting and
   strict Clippy, role graph isolation and actual SDK role builds. Confirm the
   bootstrap stays free of Meari, default stays free of CEF/UI, and CPU consumers
   have no opt-in wgpu activation. Rejoin the corrected artifact-ledger callers,
   source/metadata/lock epoch and reviewed contract replica before hosted pilots.
3. Compare old/new CPU release and dev profiles separately with the same synthetic
   workload and full Browser. Validate UI-38 retained nodes/source currency,
   crowded tabs, SVG icons, resize gutters, max/restore/minimize and parent/child
   geometry across DPI. Record dropped/coalesced resize counts and presentation
   gaps, not only surviving frame percentiles. Preserve raw first failures.
4. Run real keyboard, focus, Korean composition/Hanja candidates, AX actions and
   shutdown/sandbox pilots on the exact CPU candidate. Actual candidate-window
   position/selection and descendant teardown remain independent gates.
5. **Separate GPU candidate:** enable only trusted desktop/UI feature wiring for
   `window-gpu`, select `WindowRendererChoice::Gpu` before attachment, and observe
   status after surface initialization. Never enable it in the minimal bootstrap.
   `GpuAllowSoftware` is diagnostic only, not hardware evidence or an automatic
   fallback. The release has no automatic hardware-preferred fallback policy;
   any Auto policy remains a separately owned Root decision.
6. Before hardware-required admission, resolve the classification gap below. Then
   test actual adapter/backend, scene parity, memory/queue budget, zero-sized
   surfaces, timeout/device loss, resize-in-flight, suspend/resume and shutdown,
   CEF/input/IME/AX/DPI coexistence, package closure and visible latency.

## Focused critique and unresolved blockers

The local review applied blue/red/SRE/privacy/evaluator perspectives without
claiming an independent multi-agent panel or execution. `adversarial-critique`
verdict: **needs-work before GPU adoption**; CPU upgrade is also unqualified.

- **P1 hardware classification:** upstream `Gpu` rejects `DeviceType::Cpu`, but
  `VirtualGpu`/`Other` are not rejected by that check. Public status exposes an
  adapter string and software boolean, not a typed hardware attestation. Therefore
  `selected=VelloGpu` plus `software_fallback=false` cannot satisfy our stricter
  required-hardware contract alone. Root must obtain structured classification or
  preserve unknown/refusal; do not parse an adapter display string into trust.
- **P1 platform evidence:** no upstream Windows CEF/IME/AX/DPI or actual hardware
  result exists in the inspected release evidence. Require the named native gates;
  do not turn source/registry/test reports into runtime support.
- **P1 visual fidelity:** unsupported GPU scene operations can refuse a valid UI.
  Keep CPU available through explicit trusted policy and qualify actual SVG/font
  scenes; never silently omit paint or bypass checked-scene validation.
- **P2 operability/privacy:** renderer initialization uses blocking device setup,
  and platform surface calls may throttle. Preserve explicit error/timeout and
  device lifetime; retain numeric content-free timings publicly and raw adapter
  details privately. Keep display acknowledgement unknown and all scoring frozen.

These findings are reflected in browser-ui UI-40 and the ordered candidate gates.
No local Cargo resolution/build/test/Clippy, SDK/GPU/process/browser pilot, driver
installation or publication was performed. Shared manifests/lock and Root's
isolated assembly were read only. Rust-only and sandbox remain mandatory.
