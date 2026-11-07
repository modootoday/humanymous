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
- flat files in `header-compiler/`, each with prefix `c-` and `cpp-`:
  `version.txt`, `version-stderr.txt`, `target.txt`, `target-stderr.txt`,
  `stdout.txt`, `stderr.txt` (for example `c-version.txt`, `cpp-stdout.txt`);
- `source-before.json` and `source-after.json`.

The owner confirmed this flat filename table; the fixed ID table is the sole
output-path authority. Empty stderr files retain
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
and numeric layout syntax. Bounds: 512 lines/64 KiB, 256 bytes per line, depth eight,
integers at most 65536. The initial proposed 32-record cap was disproven by the
R3 Debian18 observation: C has 10 root records, C++ has 34. A candidate finite
Debian18 profile therefore needs an explicit 34-root-record cap and reviewed
exact auxiliary table; this correction does not admit the profile automatically.
Required four HumanymousNative V1 records and
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
and successful exits retain their typed argv recipe plus exact original argv
digest. The owner accepts this template with the exact header-path placeholder
mapped through a fixed schema to file ID/canonical root-relative path, while the
actual argv remains private. A template is not an original argv capture and must
be labelled accordingly. Both clang executables
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

Transport source is now authored in scripts/ci/delivery/src/header_proof.rs and
header_proof/codec.rs, test-only until production admission and private IO are
wired. It consumes limits from the owner's canonical header-proof-profile.json,
buffers 16 original byte arrays, uses canonical bounded base64, strict JSON and
fixed ID/order/sequence/total/length/hash checks, and requires one manifest-bound
completion. Empty files retain one explicit empty chunk. All-payload admission
runs before the begin frame is constructed; receivers call admission again only
after complete decoding. Four pure tests are authored for independent vectors,
binary/empty/multichunk round trips and hostile/truncated/mutated protocol data.
They have NOT been compiled or run. This core is not an exporter command, a
privacy validator, a fresh-private-directory provider or a hosted receipt.
Production typed tuple/profile/source admission, compiler identity capture,
official run/job binding, private reconstruction and workflow integration remain.

2026-11-07: Draft only. Header dispatch remains held by integration owner. Native
ABI R1 readiness failed before container start due to Docker local-log settings;
R2 started with compress=false but first `command -v clang` exited 127 without a
path in that exact image environment. Neither attempt observed compiler output
or compiled the header. Their raw receipts remain separate and do not establish
exhaustive filesystem absence or hosted qualification. Native ABI owner then
completed an independently allocated R3 pinned compiler-image measurement with
the unchanged approved header: C11/C++11 compile exits zero, C stdout2443 bytes/
79 lines/10 root records, C++ stdout12430 bytes/337 lines/34 root records, all
compiler stderr empty. Clang18.1.8 resolves to the same binary for C/C++ and its
before/after digest matches. The original profile report is privately retained
in 20261108-native-abi-clang-fixed-header-r3; Delivery read that report but has not
independently rehashed all command originals or promoted its raw output public.
Known /usr/include paths, unnamed records, unions and nested anonymous names
need a finite reviewed table; the earlier struct-only proposal is insufficient.
The ABI owner accepts an even narrower exact-byte privacy profile as a candidate:
bind approved header SHA, actual resolved compiler SHA/metadata/target, exact
Debian image and system-header closure, and C/C++ output digests respectively
`e3a3fd24fa31257890b787dce3a83c3f14f152377c6ccd46f93844967a190bec` and
`757dfc4219187e9b568d7cad1ed76de8f597096f4bae358302566d127f4442a3`.
Integration/X must independently approve the public byte profile before source
admission. Exact matching avoids an extensible AST syntax wildcard; every unknown
Ubuntu/tool/header tuple still refuses retention. Owner semantic four-record/
17-field checks remain mandatory. R3 generated neither owner scope nor source
snapshots, so it is not an existing 16-file hosted receipt and none is fabricated.
Its canonical private bundle is now 20261107173322-native-abi-clang-fixed-header-r3;
the earlier noncanonical path was moved by its owner with raw bytes preserved.
Container fixture stopped with137 and owned removal/absence was observed by the
owner; this is not graceful exit0. No hosted owner example ran and no Delivery
execution allocation follows from this supplementary grammar measurement.

Panel perspectives were applied in this chat without delegation: blue retains
one owner recipe; red rejects path/chunk/unknown-output smuggling; SRE requires
bounded retention and terminal cleanup accounting; DPO excludes private originals
outside the closed public set; evaluator separates execution, proof completeness
and full-product readiness. Adversarial critique verdict: needs-work. P1 blockers
before implementation/dispatch acceptance are the reviewed auxiliary AST table,
source-count measurement and compiler identity receipt. Owner filename and typed
argv-template agreement were subsequently received and incorporated above. These
are mandatory contract requirements;
no wildcard or weaker execution-only dispatch resolves them.
