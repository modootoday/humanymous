---
id: plan:root/20261107135332
title: Audit key custody Linux retained source admission
status: draft
created: 2026-11-07T13:53:32Z
updated: 2026-11-07T13:53:32Z
rel:
  governs: [crates/audit/src/verification/scan]
  depends_on: [sot:root/audit-key-custody, plan:root/20261107103624]
---

# Audit key custody Linux retained source admission

Version0.3. Root accepted the narrow prospective Unix WAL creation fix and the
Linux diagnostic premises below; validation requires a new explicit A grant.
Native producer implementation follows a separate concrete source-slice freeze.
No dependency change or production route is authorized. Phase-B qualification
remains exactly [[evidence:root/20261107133648]]. Current native Source remains
Unsupported and scanner composition remains test-only.

## Problem and proposed scope

Supply the private two-pass inspector with actual retained Linux files while
preserving its final source/transcript barrier, frozen bytes and diagnostic
precedence. Proposed first slice: Linux x86_64, existing current-user private
audit directory, existing cooperative writer lock, trusted fixed mount namespace,
and quiescent whole-directory startup. Filesystem observation is ext-family
(f_type0xef53), shared by ext2/3/4. It proves neither exact ext4 nor physical local
media. Other operating systems/filesystems and unavailable required safe APIs
return Unsupported. No fallback accepts pathname/mtime as object identity.

This is observed stability under those premises. It is not an atomic snapshot,
hostile same-UID mutation defense, persistent volume identity or independent
freshness/custody proof. No repair, healthy owner, VerifiedScan, Log mutation,
authority effect, S2/S3 consumer or production startup is added by this slice.

Research input: private recipe at
`.spec.local/root/evidence/20261107-native-abi-linux-audit-source-research/recipe.md`.
Research inspected rustix1.1.6 safe APIs; this proposal itself runs no OS probe.
Audit already declares Unix rustix fs/process. No new manifest/lock edge is
proposed. Rust1.99.1/edition2024 and forbid(unsafe_code) remain mandatory.

## Ownership and reader lifetime

Private NativeSource and every positioned Reader retain one Arc<NativeContext>.
Context owns the writer guard, directory/provider pins and all payload pins.
Thus dropping Source cannot unlock while an issued Reader survives. Reader
retains the same absolute deadline/cancellation view through a lifetime-compatible
private context; no reset per operation. Neither context nor Source exposes
File, &File, AsFd, raw descriptors, Deref, Seek, public clone or identity tokens.
Read/Seek on &File would escape this boundary too.

Readers implement only Read/BufRead, with initialized finite buffer and private
checked logical offset. Positioned read_at/pread targets the original held File.
No clone+seek reader: duplicate descriptions share the kernel offset. A clone
used solely for private lock metadata observation is separately counted and
never offered as a reader. Prefer a narrow observation helper if it avoids that
extra descriptor without exposing the guard's File. ExclusiveFile remains the
single lock implementation; its explicit unlock must occur only after the last
context/reader reference drops.

Captured end bounds reads. Short read is permitted; zero before end is
UnexpectedEof. Appended/truncated/replaced objects fail final validation rather
than publishing a silently clamped prefix. Interrupted retries poll the same
Control. Buffers, offsets, aggregate file lengths and conversions are checked.
No detached blocking helper thread is introduced.

## Opened-object and namespace admission

1. Use safe openat2 whole-path resolution rejecting symlinks/magic links and
   aliases. A trusted absolute root may itself be a mounted data directory;
   do not blindly impose NO_XDEV from the filesystem root. After admission,
   children resolve relative to the held directory with BENEATH|NO_XDEV and
   NO_SYMLINKS|NO_MAGICLINKS, RDONLY|CLOEXEC|NONBLOCK. Root opens as DIRECTORY.
   Missing required kernel support refuses the profile without weaker fallback.
2. Bind fstat and fstatfs to each SAME held descriptor, including directory,
   lock and payloads. Require regular payload type, expected current effective
   UID, exact0600 payload/lock and0700 directory permissions, payload nlink1,
   nonnegative bounded size, ext-family type and admitted device domain.
   Reuse keys::custody::permissions::check through a narrow held-directory
   helper; do not copy its policy or substitute path-only privacy inspection.
3. dev/inode are scoped to retained objects and namespace lifetime. Do not use
   private Fsid layout, transmute, Debug hashes or a caller provider label.
   Root requires same-descriptor statx EMPTY_PATH/MNT_ID with required returned
   mask. Bind/compare mount IDs on held root and payload objects; absence refuses
   with Unsupported, with no fstype+dev fallback. This is namespace/lifetime
   mount binding, not a persistent mount UUID or physical-media attestation.
4. Require pre-existing `.audit-writer.lock`; open read-only, validate, acquire
   ExclusiveFile nonblocking. Never create/unlink/repair it or its directory.
   Actual local Linux qualification must establish read-only exclusive locking
   and refusal while the real writer holds the same lock. Cooperative flock
   does not exclude a hostile owner bypassing the protocol.
5. Keep a retained trusted parent and validate the root's current name relative
   to it at every barrier. Also resolve the configured absolute root for
   identity comparison under the trusted namespace premise, never for payload
   reads. An ancestor/name replacement must refuse. Parent/temporary handles
   belong in the resource ledger. Unsupported root-parent configurations refuse.

## Membership and resource policy

Use held-directory iteration with finite entry count, name length, segment count
and checked aggregate length. Segment names remain exactly audit-NNNNNNNNNN.log,
indices1..N contiguous, sidecar checkpoints.jsonl optional, lock mandatory.
Bounded index bitmap/inventory stores identities, not decoded history or paths
proportional to record count. Reject malformed segment names, duplicates,
symlinks, cross-mount/nonregular entries and unapproved names. The initial
allowlist is those three categories plus directory iterator dot entries only.
This stricter dedicated-directory profile is explicit; legacy Wal's ignored
unrelated names are not silently reclassified as compatible. Co-located vault,
backup or other product files need an accepted bounded naming policy first.

Admit a finite FD budget BEFORE opening payloads: retained parent/root/lock,
optional lock observer, N segment pins, optional sidecar, iterator/provider and
temporary comparison descriptors. Calculate exact API-specific peak before
implementation acceptance; one active cursor does not mean one open FD.
Readers share pins and add no payload descriptor. Never evict/reopen a payload
to work around exhaustion. EMFILE/ENFILE is a clean refusal, not permission to
raise limits. A4096-segment policy does not establish4096 available descriptors.
For this proposal, use safe rustix::fs::RawDir::new with a fixed4096-byte
MaybeUninit buffer on a separately opened, securely resolved `.` directory
descriptor. RawDir borrows/owns that descriptor without another internal open;
consume borrowed names before advancing. Refuse buffer-too-small rather than
growing it. Cached rustix1.1.6 fs/raw_dir.rs documents this safe caller-buffer
API. This avoids Dir::read_from's implicit additional open and owned entry names.

Proposed scanner-owned peak is N+P+6: parent1, root1, lock guard1, metadata-only
lock observer1, retained segments N, optional sidecar P, iterator1 and at most
one temporary comparison descriptor1. Close each temporary before another open;
absolute-root comparison and relative-name comparisons reuse that one slot.
Same-descriptor fstat/fstatfs/statx add none; positioned readers add none.
If a narrow guard observation helper removes the observer, reserve the same
conservative bound initially. Keys/witness/transport and other application FDs
are additional to this scanner ledger and cannot be claimed covered by it.
Actual API behavior and process-level exhaustion still require qualification.

All Vec/bitmap/String capacities, Arc/context/buffers and descriptor inventory
are included in the ledger. Proposed128MiB remains unmeasured and is not RSS.

## Revalidation and publication

Each unchanged barrier polls Control, samples the SAME held objects and compares
type/dev/inode/length/mtime/ctime/nlink/uid/gid/mode and selected provider facts.
Ignore atime and free-space counters. Re-enumerate bounded membership and sidecar
presence; securely open current names temporarily and compare against original
held pins. Include lock name and root name. Check directory epoch around this
work. Any error/change poisons the scan and prevents any diagnostic publication.
Source methods poll during every bounded enumeration/open/revalidation loop.

Retain existing A/B transcript, counts, sidecar presence, torn-tail and final
Control checks. Final source observation precedes returning Report/Work.
Matching metadata and raw transcripts do not rule out write/restore ABA between
observations or a writer racing both passes. Whole-directory quiescence and
trusted namespace are explicit deployment premises, never a caller bool that
constructs a trusted source. Independent witness/pin authentication remains
separate from native file identity.

## Verification gates and requested decision

Root accepted quiescent cooperative writer/trusted namespace, dedicated WAL
directory separate from keys, reader-retained guard/root/pins, same-FD mount ID
and N+P+6 scanner peak plus separately accounted process FDs. Enforce the bound
before opens. Existing non0600 histories are refused by the future native
inspector without chmod; any offline owner-reviewed migration is separate and
must preserve original history. No hostile-sameUID/snapshot/ABA/interruptible-FS
claim is authorized. Native source must freeze as a separate concrete slice.

Prospective creation source now requests0600 through std Unix OpenOptionsExt in
both open_append (initial lock/segment/sidecar) and rotation create_new. Two Unix
Rust tests are authored for actual created modes/rotation/restart and preservation
of existing0644 file modes/dev/inode while appending. Tests are NOT RUN; no fmt,
check, Clippy or native qualification is inherited from the prior33-case proof.
No global umask mutation, chmod of actual history, dependency or graph edit.

- Pure Rust tests cover checked offsets, limits, poison, lifetime/last-reader
  lock retention and unchanged compatibility Report/Work semantics.
- Root-allocated ordinary-user Linux venue records actual kernel/filesystem
  observations,0700/0600 fixture, read-only lock contention and independent
  interleaved readers. Unsupported venue is refusal, never a successful scan.
- Actual local fault cases cover same-size rename, root replacement, truncate/
  regrow/append, sidecar add/remove, membership/FD overflow, symlink/FIFO,
  unsupported filesystem, cancellation and failed final epoch. Deterministic
  mutations test sampled detection, never universal ABA resistance.
- Exact source/registry/config closure, locked/offline check, strict Clippy and
  scoped strict test accounting are required. No workload is allocated here.
- Capacity/FD/RSS measurements, real kernel/device stalls and hard supervisor
  kill/reap remain separate gates. Logical deadlines cannot interrupt a blocked
  filesystem syscall. Docker product e2e and coherent workspace qualification
  remain Root-owned; component tests cannot satisfy those gates.

## Critique reflected

Verdict: needs-work for native qualification and full S1; Root accepted the
specified premises and prospective creation slice, not measured memory/product
completion. New creation source remains unqualified until allocated execution.
No independent review or native qualification is claimed by this local review.

P0: A Reader outliving Source could release the cooperative lock too early.
Local sketch: obtain reader, drop Source, acquire writer lock, continue reading.
Required delta: shared private context owns the guard until last reader drops;
actual lock-lifetime test precedes acceptance.

P0: Held-file fstat alone misses pathname replacement and root displacement.
Local sketch: rename a same-size file/root while old descriptor remains readable.
Required delta: bounded named-membership/root comparison and final publication
barrier, with trusted ancestor/namespace premise. No atomic/hostile-writer claim.

P1: A retained pin per segment exhausts FDs despite a single active cursor.
Required delta: admit exact total/peak descriptor budget and fail without eviction.

P1: Existing Wal ignores unrelated names; a generic producer could accidentally
accept co-located mutable state or reject real deployments without declaring it.
Required delta: explicit proposed dedicated-directory allowlist and Root decision.

P1: Actual writer creation does not currently guarantee the proposed0600 files.
Source inspection: storage/wal.rs::open_append uses create/append/read without
Unix OpenOptionsExt::mode; rotation's create_new also omits a mode. An ordinary
022 umask can therefore produce0644 files inside the private0700 WAL directory.
log/sealed.rs::open_inner checks custody/WAL directories but does not apply
custody::permissions::file to WAL members. Existing successful native component
tests do not establish the proposed exact0600 admission for every produced file.
Local sketch: write/rotate/checkpoint through the actual Wal then inspect all
three created file classes and attempt the new Source. A hand-made0600 source
fixture would hide this product mismatch.
Required delta: Root decision on prospective0600 creation in BOTH writer paths
and explicit existing-history policy. Recommend future creations request0600
without changing process umask; inspection must never chmod or silently repair
existing files. Exact0600 may refuse legitimately existing0644 history and must
be an explicit migration requirement, not an undisclosed compatibility claim.
Any creation fix needs actual source-bound creation/rotation/sidecar tests plus
preservation of existing-file modes. No such fix or test is authored/run here.

Read-only consumer mapping: runtime/src/startup.rs passes separate configured
key_directory and wal_directory into SealedLogOptions; runtime service fixtures
use sibling keys/audit directories; current lab Compose mounts writer-key and
writer-wal separately. This supports considering a dedicated-WAL profile but
does not prove all deployment layouts, filesystem type or permissions.

P1: Quiescent metadata checks and absolute deadlines can be overstated as
snapshot/ABA/hard-time guarantees. Required delta: retain all residuals above;
no healthy owner or production recovery token emerges from this slice.
