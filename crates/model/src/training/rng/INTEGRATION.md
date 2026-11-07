# Offline Go RNG compatibility boundary

Rust source implementation only; no execution receipt accompanies this checkpoint.
Go1.26.3 rng.go/rand.go/normal.go algorithms and tables are translated under the
original BSD license retained verbatim in LICENSE. Copyright headers are preserved.
Root integration owns any aggregate THIRD_PARTY_LICENSES.md/NOTICE update before
packaging or publication; this slice does not edit shared attribution files.

One GoRng owns seed normalization,607word wrapping feedback and primitive draw
count. Cold initialize draws27W1+1W2 normals per hidden row. Warm initialize
validates/copies without draws. The same object shuffles an existing ID order;
neither operation reseeds or restores the original permutation between epochs.
No serving Runtime, threshold provider, SGD trainer or bootstrap CLI is wired.

Public compound methods stage state/output and commit only on success. Explicit
budget/shape/schema/nonfinite failures preserve previous state and caller order.
Requested malformed warm state is rejected rather than silently cold-started;
this is the previously declared fail-closed compatibility delta. Hidden1..256,
shuffle length0..100000 and explicit primitive draw budget1..1000000 are supported.
The draw ceiling bounds this kernel API, not the still-unimplemented full training
budget; workloads exceeding it must be separately designed/qualified, not silently
reseeded or split into independent streams. No unbounded retry is exposed.

Tests are partitioned by evidence origin:

- Existing Go seed0 Int63 and NormFloat64 output goldens: retained upstream source
  expectations, not a newly executed oracle. The normal fixture retains that label.
- Full state, cold/warm initialization and mixed/multi-epoch shuffle: Rust-derived
  reference comparisons. The slow model seeds by full-width modular multiplication
  and rotates a vector, while the kernel uses quotient recurrence/circular indices.
  Both use the same audited upstream tables. Normal branch code is shared when
  checking draw streams; this is a correlated-reference limitation, not proof of
  independently observed Go internal states or cross-language SGD parity.
- Injected primitive tapes: algorithm boundary cases for rounding retry, rejection,
  normal tail signs and exhaustion, not observed seed-stream transcripts.

Copied .txt fixtures retain unchanged upstream source solely as immutable table
data. Tests parse comma-separated literal arrays; no Go code is compiled, evaluated
or executed. License and provenance remain with the Rust module/evidence bundle.
Normal-tail ln/exp and initialization sqrt rounding still need actual supported
target qualification. Do not claim bit parity on untested architectures or promote
the prior parser/evaluator results onto this new source.

Next allocated gate: fmt FIRST, scoped rng tests, model library/fixture regression,
check and strict Clippy, plus source/tool/selected registry closure. All12new tests
and every command above are NOT_RUN. Later unchanged-Go oracle comparison and
process pilots require their own allocation; no binaries, downloads or GPU run here.
