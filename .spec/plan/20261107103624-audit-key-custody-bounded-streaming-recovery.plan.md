---
id: plan:root/20261107103624
title: Audit key custody bounded streaming recovery
status: draft
created: 2026-11-07T10:36:24Z
updated: 2026-11-07T10:36:24Z
rel:
  governs: [crates/audit/src/storage]
  depends_on: [sot:root/audit-key-custody]
---

# Audit key custody bounded streaming recovery

## Goal

Design v0.1, proposed only. Root requested bounded streaming recovery before
remote sink transport. No implementation, dependency change or new validation
workload is authorized by this draft. Current code174e5f21 and110-case Windows
evidence102348 remain frozen. Root and independent security review must accept
the concrete implementation slice and resource profile first.

Read current framing.rs, wal.rs, recovery.rs, witness_recovery.rs,
verification/mod.rs, log/mod.rs, log/proofs.rs, merkle/mod.rs and record canonical
sources. The current decoder bounds each frame but collects every decoded frame;
WAL recovery collects segments/records/checkpoints, verification duplicates
record-hash leaves, and Log retains leaves/checkpoints. Replacing only read_line
does not establish bounded recovery. The full target includes opening the actual
writer and serving proofs without these history-sized in-memory collections.

### Frozen semantics and intentional differences

Keep audit-NNNNNNNNNN.log and checkpoints.jsonl bytes, field/default/null handling,
CRLF/blank-line behavior, canonical record bytes, HMAC and checkpoint signing
bytes unchanged. Merkle leaves are ASCII record-hash TEXT bytes, not decoded
hashes or JSON. Preserve RFC6962 domain separation, linear roots, previous writer
SIGNATURE linkage, strictly increasing checkpoint sizes and exact node identity.
Use independently provisioned HMAC/writer/distinct witness identity and exact
external checkpoint; never derive freshness from the directory or cache.
Passphrase rotation retains writer/HMAC/witness identity; signing-identity
transition remains unsupported and is not silently implemented by key_id.

Every complete record and every complete checkpoint is checked through EOF,
including bytes AFTER a trusted head. A valid prefix is never recovery success
when later complete bytes fail. RequireCovered/PreserveAmbiguousTail keep their
meaning; export, cancellation or retry must not sign/cover/replay an ambiguous
complete tail. Existing metadata/node/progression/trusted-presence checks precede
record errors. Preserve record mismatch order seq/link/hash/HMAC, then all base
checkpoint checks, then witness errors. Offline compatibility report fixtures
must remain byte-equivalent where resource limits do not refuse the input.

Intentional stricter differences: resource exhaustion returns a distinct bounded
failure, never a compatibility pass; counter overflow fails rather than wrapping.
Current Wal::open truncates an incomplete terminal frame before cryptographic
verification. New production open plans that exact truncation but performs it
ONLY after complete input authentication and explicit recovery selection. Invalid
sidecars or earlier complete corruption therefore leave original bytes intact.
No complete record is truncated, adopted as fresh history or automatically retried.

## Steps

### 1. Finite resource policy before allocation

Proposed qualification profile (not yet accepted defaults): frame4MiB including
newline; decoded string budget4MiB/frame;4096 items/array;65536 JSON values/frame;
depth64; trusted node ID256bytes; canonical record output16MiB; checkpoint signing
message16KiB; normalization pending scalars64; accounted working buffers128MiB;
4096 segments;10million records and
10million checkpoints;64GiB aggregate record bytes;4GiB checkpoint bytes;
no derived proof cache in v0.1; monotonic deadline300seconds. All values finite,
validated with checked arithmetic; zero/overflow/impossible profiles refuse.
Trusted installation policy may explicitly choose another validated profile;
input cannot select its own limits. No unbounded fallback or skipped outcome.

Frame length alone does not bound owned serde allocation: many empty/default
objects can amplify Vec element storage. Before owned Record/Checkpoint decode,
a bounded serde visitor preflights depth, decoded strings, element counts and
total JSON values without building Value trees. It then uses EXISTING Record/
Checkpoint decoders; fixed schema arrays and fields receive their preflight
budgets BEFORE owned decoding. Resource refusal is distinct from a wire/report
pass, and never truncates a value to fit. Account parser scratch, decoded fields/
Vec capacities, normalization, signing messages and hash buffers.

Use a shared sink-based canonical encoder in BOTH compatibility and streaming
paths, not a copied hash algorithm. Vec-returning compatibility wrappers collect
that same encoder; bounded hashing emits directly to the existing digest through
a byte-limited sink and never builds the complete canonical Vec, parts Vec or
joined array strings. Keep original raw rule/verdict versus normalized scalar/
signal-ID behavior, ordering helper and fixed confidence formatting. Normalize
through a shared Go stream-safe Unicode15 iterator with bounded pending state;
no unbounded safe-string-then-collect chain remains in the bounded path. Golden
Unicode/canonical vectors must prove byte identity before using it for authority.

Checkpoint signing bytes use the same frozen JSON ordering/escaping with a
bounded message buffer checked while encoding, before crypto allocation. The
16KiB cap and256-byte trusted node bound refuse outside-profile inputs rather
than changing signatures or diagnostic priority for admitted inputs. Existing
public-key/signature validation remains strict, with fixed-size hex decode where
possible; no unchecked arbitrary signature-sized allocation is accepted.

The allocation ledger must include two bounded input frames/cursors, decoder
scratch, both owned record/checkpoint strings, conservatively grown array
capacities, bounded sort references, normalization state, checkpoint message,
frontier/path descriptors and custody/witness context retained during scanning.
Record currently has only triggered_rules Vec<String> and top_signals Vec<Signal>;
4096-element limits bound their per-element overhead independently of string bytes.
No JSON Value tree, Vec<all records/checkpoints/leaves/paths>, canonical output
Vec, signature-sized input buffer or per-record retained projection is allowed.
A source-derived capacity calculation plus instrumented worst-case allocation
test must prove the profile before acceptance;128MiB is not a measured process
RSS promise. Trusted external witness implementations must declare bounded
buffers too; a trait object alone does not establish a whole-process bound.

Cancellation is checked before/after bounded frames, directory entries, crypto
work and repair steps. It stops future work and grants nothing. Synchronous OS
file I/O cannot promise a hard interrupt deadline; an owned process supervisor
supplies the external watchdog, with no foreign process termination.

### 2. Inspection under one nonextractable writer lock

Proposed WalInspection owns the existing native single-writer lock and trusted
directory configuration. It opens sources without symlink/reparse following and
reuses owner/privacy/regular-file checks. Enumerate matching segment names with
finite counters, validate exact10-digit indices and contiguity, then stream
expected names1..N; do not collect a Vec of paths or records. Checkpoints use an
independent bounded cursor. Keep at most one segment and one sidecar cursor open.
Check source identity/length before and after every pass and rescan membership
before accepting the result. Cooperative locking/trusted-directory ownership
remains a premise: this does not claim protection from a malicious same-owner
or privileged writer, nor an immutable distributed snapshot.

Inspection performs no authoritative truncate, rename, checkpoint append or
append-capable owner publication. V0.1 needs no derived staging/cache files;
acquiring the existing persistent lock inode is distinct from history repair.
Empty storage is accepted only by the existing explicit first-install rule;
existing sealed keys never regenerate. Invalid/oversized/nonterminal torn frames,
segment gaps, sidecar truncation and changed source identities refuse.

### 3. Complete streaming verification with compatibility priority

Phase A scans all bounded frames for parse/node/metadata/progression and exact
trusted-head presence, retaining scalar counters and first errors only. It also
records the last incomplete record offset as a PLAN, never mutating the source.
Phase B streams record authentication and Merkle frontier construction. Use at
most64 hash slots plus one current record and previous linear hash. A second
checkpoint cursor joins strictly increasing tree sizes to exact prefix linear
and Merkle roots. Keep first base-checkpoint and first witness failures separately;
finish all record checks and prioritize base failure before witness failure,
matching frozen verification behavior. Checked counters prevent overflow.

Factor reusable record/checkpoint validation primitives so offline and streaming
paths share canonical/HMAC/signature/error rules. Do not reimplement field rules
or call the old Vec-returning verifier inside the bounded path. Phase A and B
must both reach EOF and agree on source identity/length/count commitments.

Only then construct a private-constructor, noncloneable/nondeserializable
VerifiedScan carrying lock ownership, exact source commitments, sequence/linear
head, frontier, latest checkpoint and authenticated recovery coverage. It is not
an effect grant or distributed freshness lease. No public per-frame callback
receives authority during the unverified scan. Any trusted consumer projection
must finish verification before releasing effects or a healthy readiness state.

### 4. Bounded operational writer and proof access

Log cannot retain Vec<leaf> or Vec<checkpoint> in the bounded production path.
Keep frontier/counters/latest checkpoint; older checkpoint/record access uses
bounded authenticated cursors. Concrete v0.1 proposes bounded authenticated
rescans for proofs, preserving current functionality without a new persistent
cache format/trust boundary. Derive at most65 sibling/subproof ranges from
checked sizes/indices using existing RFC6962 split rules. Scan actual record-hash
TEXT in sequence, compute required range commitments with bounded frontiers and
retain only ordered path hashes. Sort disjoint sibling ranges by file position,
compute one range frontier at a time and restore canonical proof order using
the bounded range descriptors; keep a separate full-history verifier frontier.
Authenticate through the exact requested source
snapshot before returning any proof. Check outputs against expected signed roots
with existing proof verifiers before publication or witness use. Corruption,
resource/cancel failure returns no partial proof or checkpoint. No dependencies.

This costs O(history) I/O per proof and may refuse on the trusted scan budget;
it is not a throughput claim. Required witness consistency-proof generation
obeys the same budget and poisons/refuses rather than bypassing the witness.
A disposable on-disk index is a later optimization requiring its own format,
checked addressing and crash/authentication design. Neither path may silently
retain all leaves. The target remains actual bounded writer startup/proofs and
consumer restoration, not merely a standalone streaming verifier.

Record append remains WAL write+sync before success; no derived artifact replaces
authoritative WAL acknowledgement. Required
external witness and its actual durability gate remain mandatory. No signing
identity rotation, checkpoint rewrite, sequence reset or auto-grant replay.

Legacy records()/checkpoints() Vec materialization is explicitly outside the
bounded path. Introduce bounded cursor/fold APIs and migrate actual runtime and
authority consumers, including settings/kill restoration and proof callers.
Keep legacy helpers only with explicit materialization budgets; no production
open/proof/restore path may call them and still claim bounded recovery. Exact
cross-owner API names and consumer migrations require owner acceptance.

### 5. Repair, cancellation and restart

VerifiedScan may be consumed once for exact terminal incomplete-byte truncation
and/or appending the already authenticated missing EXTERNAL checkpoint. Recheck
source commitments under the retained lock immediately before writes. The
existing sync/poison/platform durability boundary applies. A repair failure,
uncertain rename/flush or cancelled transition grants no owner/readiness receipt;
preserve complete data and report unavailable with explicit recovery needed.
No new checkpoint signatures or business-effect replay are part of repair.

Cancel/drop before verification releases owned scan resources and the lock;
source bytes remain unchanged. Resume means a NEW full scan
under a fresh lock and independently authenticated current pins. No serialized
partial-prefix token skips input. Cancel after a possible repair mutation leaves
explicit uncertain state and requires fresh authenticated recovery, not rollback
or automatic continuation. Do not promise power-loss atomicity on Windows.

### 6. Required validation and acceptance boundaries

- Pure frontier roots/proofs versus existing Tree and immutable Go oracle vectors
  over empty/singleton/odd/power-of-two/large prefix sets; exact text-leaf encoding.
- Offline-versus-streaming Report/Scope and priority equivalence for every current
  fixture, null/default/Unicode/duplicate-field and adversarial multi-failure case.
- Generated many-segment/many-record/many-checkpoint inputs with measured owned
  allocation/capacity bounds independent of history length, plus RSS observations
  clearly distinguished from the accounted-buffer contract.
- Truncation/corruption before/at/after trusted head; terminal incomplete versus
  complete bad frame; sidecar failure after a repairable record tail; foreign key/
  node/root/signature, missing/older external head, complete ambiguous tail.
- Actual ordinary-user file fault/cancel/change/restart and both repair boundaries;
  no owner/effect callback before EOF or after resource/fault/cancel refusal.
- Proof range/address/count overflow, source change between passes and budget/
  cancellation during required witness proof generation; every emitted path
  locally verified before required witness RPC. No cache-only boot.
- Full current audit package and paired exact helper accounting, remaining OS/
  protected profiles, then actual consumer ServiceHost and Docker-only product
  outage/no-origin-effect/permission/restart proof. Unit success is not product.

### 7. Review questions blocking implementation

1. Accept the proposed finite profile and conservative predecode allocation proof?
2. Accept bounded authenticated proof rescans for v0.1, with explicit O(history)
   I/O/cancellation limits and no new persistent cache/format?
3. Accept the intentional verify-before-torn-repair tightening and cross-owner
   cursor/projection APIs? No compatibility report priority may drift silently.
4. Confirm independently pinned checkpoint acquisition and same-owner threat
   premises; a source lock/cache digest is not remote freshness or OS isolation.

### 8. Concrete implementation sequence, still design-only

S1 proposes private FrameCursor/JSON budget preflight, shared byte-limited canonical
encoder/normalizer, incremental record/checkpoint validation and Merkle frontier/
range primitives. Native read-only
WalInspection streams source handles under the existing lock. It emits only a
bounded diagnostic summary; no append-capable owner, business-effect callback
or publicly reusable trusted-state token exists. Pure vectors/differential
fixtures and actual bounded-input allocation/fault tests qualify these primitives.
S1 alone does not satisfy bounded production recovery.

S2 proposes additive SealedLog::open_streamed and recover_streamed entrypoints
with trusted validated ScanLimits and cancellation state. The composition root
keeps Config/key custody, WAL lock, witness and internal VerifiedScan within one
nonextractable construction lifetime. VerifiedScan is module-private, not a
public assertion consumers can transplant to another Config/key/source. After
both full passes, the same context performs exact selected repairs and publishes
the operational Log with frontier/latest checkpoint and bounded proof rescans.
No authority is reconstructed from a public receipt/boolean or different keys.
Old open/recover paths are explicitly legacy until consumer migration completes.

S3 migrates runtime/authority startup, settings/kill restoration and proof/export
callers to bounded cursor/projection APIs under their owners. Projections accumulate
only their declared bounded state and release no effects until successful finish.
No Vec-returning compatibility call may remain inside a claimed bounded path.
Operator/admin paths and actual Docker product faults complete the same target;
they are not inferred from S1/S2 component results. API details and resource policy
are accepted jointly with those owners before S2/S3 implementation.

Runtime owner reviewed the design and supplied concrete S3 requirements, without
accepting an API or changing source. Replace startup witnessed_through's
checkpoints().last Vec with a bounded latest authenticated head/coverage accessor.
SettingsStore journal restoration folds only bounded latest revision/digest/
parent state while preserving exact node/tenant/policy and contiguous revision
checks; a later bad settings event or ambiguous tail still refuses. KillStatus
folds scalar effect state but scans THROUGH EOF and rejects unknown runtime.kill.*
events. It never publishes early Clear/Killed merely after finding one record.

Cursor/projection completion must bind the SAME retained context/source epoch,
not just equal path/node/head text. Use module-private checked context identity
and generation, no caller-selected boolean or deserializable completion receipt.
Hold the source borrow/lock through projection finish and validate identity,
generation and full-source completion before restore/finalize/Ready. Source
mutation invalidates unfinished/stale projections; a projection from another
context cannot be transplanted. Cancellation, late corruption, unverified tail
or repair uncertainty yields no Config/apply/Ready. Preserve Pending and
unwitnessed-intent semantics; these observations never reconstruct a grant.

The proposed300-second scan profile composes with the caller's existing absolute
deadline: runtime's30-second observer/startup allowance uses the MINIMUM, never
a reset/extension. Every pass, repair and proof rescan shares that deadline and
cancellation state. Async caller integration must avoid blocking its stop/watchdog
control path; OS-read interruption still needs the owned external supervisor.
Bounded projections and O(history) rescan costs are not throughput or independent
deployment evidence. Full consumer acceptance remains a separate design gate.

Self-critique: P1 allocation amplification is addressed by predecode limits plus
required measured/conservative bounds, not trusted to frame length alone. P1
premature repair is addressed by inspection/no-write before full authentication.
P1 cached/partial-prefix authority is addressed by module-private context and
full-scan retry, not a serialized resume token. P1 hidden full-history retention
is addressed through operational writer/proof/consumer migration, not only decoder
refactoring. Residual same-owner modification and O(history) proof I/O are explicit.
Independent critique may add blocking findings; none is waived by this draft.

## Record

Design only. No source, dependency, scoring or runtime behavior change; no new
validation process. Root and independent native/security review requested.
