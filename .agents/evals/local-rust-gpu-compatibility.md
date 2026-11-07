# Local LLM GPU compatibility proposal

Status: source/research only, after the unchanged64-attempt CPU study terminated.
No GPU model, CUDA context pilot, backend switch, SDK/driver install, new shared
dependency, authored shader/native helper or unsafe exception was executed.
Cross-project selection policy remains root-owned in the structural canon.

## Exact evidence and primary sources

The completed CPU study uses mistral.rs0.9.5 and official Qwen3-1.7B-Q8_0. The
upstream tag resolves to commit465191d0b0a7e2432d327b0de06204ab3f94d3a2. Its
[workspace manifest](https://github.com/EricLBuehler/mistral.rs/blob/465191d0b0a7e2432d327b0de06204ab3f94d3a2/Cargo.toml)
declares Rust1.94 minimum and Candle0.11 at revision
66a8cf184a5a519671454066b1b9efd446ec9f5c. This does not reveal the compiler used
to build the downloaded binary. Our authored tooling remains Rust1.99.1/edition2024.
The [tagged core features](https://github.com/EricLBuehler/mistral.rs/blob/465191d0b0a7e2432d327b0de06204ab3f94d3a2/mistralrs-core/Cargo.toml)
provide CUDA and Metal; no Vulkan backend feature is present in this manifest.

Read-only Windows NVML observations, recorded in evidence:root/20261107184338:

| Observation | Actual value / limit |
| --- | --- |
| Device | NVIDIA GeForce RTX4070Ti; reported compute capability8.9 |
| Driver/NVML/KMD |610.88; earlier Windows inventory32.0.16.1088 is not a CUDA runtime version |
| Snapshot memory |12282MiB total,10836MiB free,1159MiB used; not a reservation or model-fit proof |
| CUDA UMD report |13.3; driver capability, not installed toolkit or successful CUDA inference |
| Actual GPU model/context pilot |NOT_RUN; usable context, loader, kernel coverage and process VRAM remain unqualified |

[NVIDIA's driver matrix](https://docs.nvidia.com/datacenter/tesla/drivers/latest/cuda-toolkit-driver-and-architecture-matrix.html)
distinguishes driver compatibility from toolkit installation and records Ada8.9.
No nvcc/toolkit installation or supported CUDA context was verified here. These
queries do not establish that Windows, a container or another OS can load a model.

## Eligibility matrix

The [tagged hardware table](https://github.com/EricLBuehler/mistral.rs/blob/465191d0b0a7e2432d327b0de06204ab3f94d3a2/docs/src/content/docs/reference/hardware-support.md)
and [release assets](https://github.com/EricLBuehler/mistral.rs/releases/tag/v0.9.5)
define candidates, not our supported product tuples:

| OS/architecture/vendor | Upstream route | Our current proof |
| --- | --- | --- |
| Windows x86_64 CPU | Official CPU prebuilt | Actual study completed; quality benefit unqualified |
| Windows x86_64 NVIDIA | CUDA source-build candidate; no Windows CUDA prebuilt listed | NVML visibility only; SDK/build/link/context/model NOT_RUN |
| Linux x86_64 NVIDIA/Ada8.9 | CUDA13.0 sm89 official artifact exists | Candidate only; current Windows driver report does not prove container/OS exposure |
| Linux ARM64 NVIDIA Grace | Upstream architecture-specific CUDA assets | No matching host, driver/runtime or model pilot here |
| macOS ARM64 Apple | Official Metal prebuilt | No matching host or actual pilot here |
| macOS x86_64 Intel | Upstream source-build route, no listed prebuilt | No supported model/backend tuple established here |
| Linux x86_64/ARM64 CPU | CPU releases | No current-profile execution in this study |
| Vulkan/AMD/Intel GPU or software adapter | No backend established by this pinned runtime's feature table | Ineligible for this profile; an alternative runtime requires separate review |

The tagged Windows installer prefers its CPU prebuilt. Its source-build branch
can select CUDA, but installer source is not a build qualification. Current live
documentation may describe a newer guided installer; do not replace tagged facts
with that behavior. No installer script was executed during this research.

Relevant immutable release asset identities:

- Windows CPU archive: SHA25664d567c299a5c97ef265aa0362649aafb968af47706810df640f74fe8ea4702a.
- Linux CUDA13.0/sm89 archive: SHA256c4e287c371b1a48090e5ce78b53bfc09ca52fc2689fa475dae73fb6c571e8deb.
- macOS ARM64 Metal archive: SHA256dcfb2e6c43692e391e3bfa8a0f1e3140447a492d5af0ae00731476eb67641815.

These are reference pins already associated with0.9.5, not authorization to add
a dependency or load another backend. Physical NVIDIA visibility is distinct from
a software rendering adapter; do not call WARP/lavapipe/CPU-backed fallback a GPU
speedup. Actual backend/device identity must be observed inside the future process.

## Proposed interface and selection boundary

Keep the current external runtime/loopback protocol boundary; the Rust driver
controls finite prompts, read-only skill returns and receipts. No GPU ownership or
numeric policy moves into deterministic authority/scoring/security kernels. An
in-process Rust SDK integration would be a new reviewed graph/ABI decision, not
an incidental switch. Public naming and variant packaging remain root contracts.

Proposed profile requests distinguish cpu_only, prefer_gpu and require_gpu.
cpu_only retains the proven CPU route. prefer_gpu may select an explicitly
eligible, budgeted, actually observed device after a separate accepted pilot; any
CPU fallback is recorded as CPU, never GPU evidence. require_gpu refuses when the
backend, device/driver/kernel/dtype or budget is unknown. No silent software
adapter, dtype/quantization/context change, multi-GPU split or hidden CPU layer
offload may satisfy a GPU-required comparison. This is a local proposal, not a
new global selection priority or implemented selector.

Receipts need requested/actual backend, binary/features provenance, OS/architecture,
device identity, driver/runtime versions, model/tokenizer/template hashes, actual
tensor/cache dtype and placement, budget/queue/cancel states and fallback reason.
Our CPU prebuilt's full compiled-feature inventory was not independently inspected.

## Matched experiment and resource proposal

Use a **new** source/resource allocation after an implementation decision. Existing
CPU semantic receipts are a reference, not a clean CPU/GPU performance benchmark;
other admitted lightweight jobs were present. Establish fresh matched CPU and GPU
runs with the same frozen driver, tasks, arms, repeats, GGUF, embedded-tokenizer
behavior, official template, context8192/maxseq1/greedy0/output768 and prefix0.
Keep the failed CPU seed setting absent. Preserve all incomplete cases. A different
tokenizer fallback or hidden placement makes the run ineligible for matched claims.

For the first matched GPU profile, propose paged attention **off**, as on CPU.
The [tagged paging guide](https://github.com/EricLBuehler/mistral.rs/blob/465191d0b0a7e2432d327b0de06204ab3f94d3a2/docs/src/content/docs/guides/perf/paged-attention.mdx)
documents CUDA/Metal auto-paging versus CPU's standard cache; leaving auto enabled
would change the memory policy. Explicit shared KV goals require paging. Therefore
do not pretend --kv-cache controls an off-mode standard cache. A later paging
comparison is a separate factor, not part of the first matched backend experiment.

Qwen's declared28 layers,8 KV heads and head_dim128 imply8192*28*8*128*2*2 =
939524096bytes (896MiB) for one BF16 standard KV sequence. This is a theoretical
calculation, not observed allocation/dtype. The1,834,426,016-byte weight file is
not total VRAM: dequantization/workspace, attention intermediates, driver context,
allocator reserve and any host/device copies remain additional and unmeasured.
The snapshot's10836MiB free is not guaranteed at launch.

Propose initial reviewed limits of one model/one sequence, host admission
commit<85%/free>20GiB, the existing owned-stop88%/<16GiB thresholds, monitored host
private8GiB and provisional device-allocation6GiB with a2GiB free-device floor.
These are proposal numbers, not grants or hard OS/driver caps. Recheck fresh
availability before model placement and each prefill peak; preserve other owners'
GPU allocations. A root allocation must confirm limits and original deadlines.
Do not repeat earlier4B/8B automatic-GPU KV allocation failures under a larger guard.

Measure cold load/HtoD and warm request prefill/decode separately. JSON/tokenization,
request dispatch, queueing, HtoD/D2H and synchronization can dominate small requests;
model residence amortizes weight transfer only after actual measured reuse. Report
latency distributions, token throughput and memory samples with source/load state,
not an assumed speedup. GPU power samples alone are not whole-host energy cost.

## Numeric, cancellation and evidence gates

CPU/GPU quantized kernels, accumulation order, BF16/FP16/FP32 handling and ties can
change generated tokens even at greedy temperature0. Do not require or claim
bitwise text parity without evidence. Keep model/quantization fixed, capture any
actual dtype/kernel/placement change and test safety-rubric outcomes separately.
The current same-Qwen judge fails counterexamples; it cannot certify GPU parity or
quality benefit. An independently calibrated grader or explicit reviewed rubric
assessment is required before an effect claim.

Bound queue length/concurrency to1 and bind each request to original source/run
and deadline. Socket timeout is not proof that device work stopped. Cancellation,
drain, synchronization, OOM, process exit and device-loss behavior need actual
provider receipts before safe reuse or unload. On in-flight loss, preserve partial
outputs and refuse a completed-case claim; do not silently replay tools on CPU.
Only a separately recorded fresh CPU attempt can be a fallback. Never reset a
shared GPU, kill foreign processes, change clocks/power/global affinity or provoke
a driver reset as an unapproved fault test.

Planned gates, all GPU execution currently NOT_RUN:

1. Rust pure eligibility/budget/require-GPU/fallback/refusal accounting tests.
2. Exact physical device/binary/driver/runtime/model/context pilot, observed VRAM
   and host budgets, then actual bounded cancel/drain and supported fault probes.
3. Fresh matched CPU/GPU study with complete expected denominators and a reliable
   grader; include unsupported/fallback/OOM/truncated/unknown outcomes.
4. Separately allocated genuine OS/architecture and Docker-only product gates if
   a later product integration is proposed. Component inference is not product
   e2e, provider migration, detection change or complete M11 qualification.

Local critique reflected: device presence is not kernel readiness; free-memory
snapshots are not reservations; theoretical KV is not a measured allocator cap;
speed is not safety parity; software adapters are not hardware acceleration;
cancel/unload and device-loss remain unqualified until actual supported probes.
