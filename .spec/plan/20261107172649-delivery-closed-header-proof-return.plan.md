---
id: plan:root/20261107172649
title: Delivery closed header proof return
status: draft
created: 2026-11-07T17:26:49Z
updated: 2026-11-07T17:26:49Z
---

# Delivery closed header proof return

## Goal

Prepare a Rust-owned, closed success-only return of original public header proof
bytes through retained Actions logs. This draft authorizes neither dispatch nor
an implementation allocation. Existing component execution and retained-proof
completeness are separate outcomes. No artifacts, permissions, registry writes,
generic file export, private Specfold bundle or executable bytes are added.

The owner recipe remains `header_conformance.rs`; the Delivery component must
retain its genuine hosted Linux requirement, C11/C++11 commands, layout checks,
source-before/after comparison and exact success/scope markers. A local grammar
measurement cannot qualify the hosted component.

## Steps

1. Agree the closed payload/profile contract below with the native ABI owner and
   integration owner. Acquire actual Linux compiler output grammar separately;
   do not guess auxiliary builtin/system records.
2. After source allocation, implement pure byte validators, bounded emitter and
   reconstructor in Delivery. Keep all data buffered and validated before the
   first payload chunk. Add resolved clang/clang++ path and executable hashes
   before/after to the typed identity receipt, without exporting executable bytes.
3. Test hostile/truncated/duplicate/oversized protocol cases and original byte
   round trips under an allocated Rust unit window. Review source and privacy
   tables before an exact-source hosted dispatch is allocated.
4. Retrieve official run/job/log metadata, reconstruct in a fresh private root,
   rehash original bytes and register the local closure. Preserve first failures.
   No incomplete receipt may satisfy the retained-proof gate.

### Closed original payloads

Exactly 16 fixed file IDs in fixed order map to these filenames:

- `native-abi-v1.h`;
- `header-compiler/scope.txt`;
- each `header-compiler/c/` and `header-compiler/cpp/`: `version.txt`,
  `version-stderr.txt`, `target.txt`, `target-stderr.txt`, `stdout.txt`, `stderr.txt`;
- `source-before.json` and `source-after.json`.

The exact owner filenames must be confirmed against the recipe before coding;
the fixed ID table is the sole output-path authority. Empty stderr files retain
length zero and the SHA256 of empty bytes. Optional cbindgen version original
would require a separately versioned 17-file profile; the initial profile instead
retains typed actual version `cbindgen 0.29.4`, exit 0 and executable hash.

Original bytes are never normalized, redacted in place, or silently replaced by
a projection. Compiler stdout contains nonempty AST layout records. Unknown
warnings or other data refuse payload retention while preserving the separately
observed execution outcome and file hashes.

### Privacy admission and bounds

All payloads pass strict duplicate-key/schema/path validation, byte caps and the
reviewed per-file grammar before any chunk is printed. Proposed limits: header
128 KiB; each source snapshot 2 MiB; other files 64 KiB; total decoded 6 MiB.
These are design bounds, not measured Linux maxima. Version output is tighter:
ASCII/LF, at most 2048 bytes/eight lines, reviewed anchored Ubuntu/Debian/upstream
clang version prefix, Target, Thread model posix and InstalledDir limited to
`/usr/bin` or `/usr/lib/llvm-<digits>/bin`. No controls, ESC, user paths or arbitrary
URLs. Targets are exactly one LF-terminated line, either
`x86_64-pc-linux-gnu` or `x86_64-unknown-linux-gnu`. Version/target/compile stderr
are exactly empty in the initial retention profile.

The existing 4600-byte original header SHA256
`1b96ad497c7c17f4fcd032b08080d783a0bbfa31287e20c9821195776709d10a`
is one approved exact-byte candidate. A different Linux header must pass an
independently reviewed public-source grammar; hash mismatch never permits an
arbitrary comment dump. Rehash the original before copy/mount; preserve it.

Compiler AST grammar admits only individually reviewed record names/types/fields
and numeric layout syntax. Bounds: 512 lines/64 KiB, 256 bytes per line, 32 records,
depth eight, integers at most 65536. Required four HumanymousNative V1 records and
17 fields remain checked by the owner. The existing layout parser skips unknown
lines and therefore is not a privacy validator. Extra builtin/system records
require actual observation and a finite reviewed table, never a name wildcard.

Source snapshots retain the exact `humanymous.delivery-source/1` original schema,
including all recursive governed public source files and canonical Git objects.
Admit only canonical repo-relative paths in `source::SCOPES`, exact key/type/hash
forms, bounded counts and bytes, and no absolute/private/environment values.
Require a clean candidate and the exact canonical candidate path set so untracked
files cannot be smuggled through a broad public prefix. Snapshot count caps must
be measured before acceptance. If the full original does not fit, a new explicit
projection schema needs owner equivalence review; it cannot be called original.

### Versioned log transport

Use one versioned fixed-profile envelope bound to exact source/run/job/OS/arch,
runner image, Rust/Cargo/lock/config, owner/recipe hashes and a closed environment
whitelist. Do not dump environment or tool installation logs. Fixed commands
and successful exits retain their typed argv recipe; an argv template is not an
original argv capture and must be labelled accordingly. Both clang executables
need resolved path plus SHA256 before/after for a full compiler identity claim.

Manifest entries have fixed file ID, raw length and raw SHA256. Deterministic
base64 chunks carry file ID/sequence/total; proposed decoded chunk size is 3072
bytes. Completion contains the canonical manifest digest and a single final
marker only after all owner, source, privacy and whole-envelope checks pass.
Public failure output is limited to finite statuses/reasons and hashes.

The Rust receiver rejects missing/duplicate/gapped/reordered/extra/unknown chunks,
oversize or invalid base64, inconsistent totals/lengths/digests, multiple complete
envelopes and trailing protocol records. Proposed input-log cap is 16 MiB and
protocol-line cap 16 KiB. It hashes decoded bytes without normalization and writes
only fixed filenames into a freshly created private output root. Partial output
never obtains a completion receipt. It does not traverse a caller-supplied bundle.
Truncated retained logs fail the proof-return gate. Official API metadata must
bind source, run, job and conclusion; log transport is not a signed attestation.

## Record

2026-11-07: Draft only. Header dispatch remains held by integration owner. Native
ABI R1 readiness failed before container start due to Docker local-log settings;
R2 started with compress=false but first `command -v clang` exited 127 without a
path in that exact image environment. Neither attempt observed compiler output
or compiled the header. Their raw receipts remain separate and do not establish
exhaustive filesystem absence or hosted qualification. Native ABI owner is now
coordinating a user-requested compiler-bearing venue; Delivery will not pull,
install, launch or assume that allocation.

Panel perspectives were applied in this chat without delegation: blue retains
one owner recipe; red rejects path/chunk/unknown-output smuggling; SRE requires
bounded retention and terminal cleanup accounting; DPO excludes private originals
outside the closed public set; evaluator separates execution, proof completeness
and full-product readiness. Adversarial critique verdict: needs-work. P1 blockers
before implementation/dispatch acceptance are the unobserved auxiliary AST table,
exact owner filename table, source-count measurement, compiler identity receipt
and argv-template agreement. These are mandatory contract requirements above;
no wildcard or weaker execution-only dispatch resolves them.
