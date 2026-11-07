---
id: plan:root/20261107151758
title: Audit key custody typed production recovery
status: draft
created: 2026-11-07T15:17:58Z
updated: 2026-11-07T15:17:58Z
rel:
  governs: [crates/audit/src/storage]
  depends_on: [plan:root/20261107103624, plan:root/20261107135332, sot:root/structural-development]
---

# Audit key custody typed production recovery

## Goal

Design v0.2, DRAFT/needs-work. Prepare the production recovery contract after
private S1 facade qualification; this is not S2 implementation or authority
approval. L2 owns real storage/FD/recovery inputs; Runtime/Authority/CLI/Lab own
their consumer paths. Existing claims/full goal survive the structural role change.
Latest actual native source proof remains3d325/six tests in evidence144415.
Subsequent8cbc GNU gate qualifies only five pure L1 transition tests, fmt/check/
strict Clippy, evidence155745. Facade/capacity compile there but are not executed;
no native/repair/capacity qualification is inherited from the five-kernel gate.

Replace history-sized production startup/proof materialization with one guarded,
typed, bounded owner. Authenticate all complete input before torn-record repair
or missing-external-checkpoint append. Preserve canonical/signing bytes, report
priority/Scope, unsigned proof APIs, physical checkpoint provenance, independent
witness requirements, fsync-before-success and no business-effect replay.
No frozen scoring change, new dependency/unsafe, ephemeral key bootstrap,
signing-identity rotation, silent unsupported-platform fallback or erasure claim.

## Steps

### 1. Separate file ownership from operation deadlines

Current NativeSource Context Arc owns pins/guard but BORROWS startup Limits and
absolute Control. It cannot be moved directly into a long-lived writer: that
deadline eventually expires and its caller borrow cannot become self-referential
owned key/file state. A boolean "verified" or cloning handles into a new owner
does not resolve the contract.

Propose private NativeFiles owning root/parent/provider/payload pins and sole
guard, with no File/&File/raw-handle getters, public Clone/Deserialize or
into_parts. An OperationSource borrows that owner plus SAME operation policy,
Limits and absolute Control; its readers retain an operation context with an
owner-lifetime borrow. Source drop cannot invalidate a reader; Rust borrows block
mutable writer transitions while read leases survive. Deadline/cancel is per
operation, never reset per segment/read. Permanent owner poison is distinct from
an operation's cancelled state; classify retryable request refusal versus lost
source/uncertain write explicitly before implementation. No stale startup Control
is retained as a perpetual health clock.

Proposed private state transitions:

| State | Owned facts | Allowed transition |
| --- | --- | --- |
| OpeningOwner | Existing custody/key/witness context plus admitted NativeFiles/guard; no successful history | Complete two-pass authentication under one borrowed operation context |
| VerifiedScan | SAME OpeningOwner, private authenticated scalar/frontier/physical provenance and optional exact repair plan | Consume once for final revalidation + selected repair/initialization |
| OperationalOwner | SAME keys/files/guard, fixed frontier, sequence/head, physical checkpoint link/counters, durability/poison state | Bounded append/checkpoint/proof/fold operations |
| AmbiguousReadOwner | SAME authenticated files/context plus immutable preserved-tail/external coverage facts | Read-only inspection/proofs; no normal append/checkpoint/effect/Ready promotion |

Construct VerifiedScan only inside a consuming authenticate(self) operation:
temporarily borrow owned key/file fields, finish checks, then move the entire
unchanged owner into the result. Avoid a self-referential borrowed policy stored
beside moved keys, or transferable summary attached to another file set. No
public constructor, clone/serde, source-path token or caller success flag. The
existing Diagnostic/Report/WitnessCoverage are observations, not these types.

Bind the startup transaction explicitly: authenticate(self, &'op Limits,
&'op Control, mode) -> VerifiedScan<'op>; finalize(self) accepts NO replacement
Limits/Control. VerifiedScan owns OpeningOwner while borrowing only the original
external operation bounds, which is not self-reference into moved key fields.
Final revalidation/repair checks that same absolute clock and cancellation source.
Only a later independent OperationalOwner operation may receive a fresh context.
Require meaningful actual-type borrow/argument compile-fail checks and a negative
cancel/expired-original-clock-between-auth-and-finalize test with zero writes;
privacy-only compile failure is not proof of the lifetime invariant.

#### Actual-type qualification matrix (proposed; not executed)

The current OperationBounds planning model is not VerifiedScan and cannot close
these gates. Keep the following checks beside the eventual actual private owner
types, with access sufficient to exercise those types. Do not export custody or
file handles merely to make an external compile-fail fixture reach them.

| Gate | Required positive control | Required negative evidence |
|------|---------------------------|----------------------------|
| Original operation lifetime | Authenticate and consume finalize inside the original Limits/Control scope | Moving VerifiedScan beyond either original borrow fails for that borrow, not unresolved imports, privacy, missing dependencies or unrelated type errors |
| No replacement context | The actual consuming finalize() call type-checks | finalize(fresh_limits, fresh_control) fails against that same method signature; no alternate method accepts replacement bounds |
| Single-use owner | One authenticate/finalize chain type-checks | A second use of the consumed OpeningOwner or VerifiedScan fails as a moved-value error; no Clone/serde/constructor bypass is available |
| Original post-auth cancellation | Unchanged original control completes the admitted transition | Cancel ORIGINAL after authentication, leave a fresh control healthy, then finalize refuses before every mutating native operation |
| Original post-auth expiry | The same transition before its original deadline succeeds | Advance a deterministic private clock seam past ORIGINAL deadline after authentication; finalize returns TimedOut before mutation, without sleep-based timing assumptions |
| Preserved-tail authority | Read-only proof/inspection remains available on AmbiguousReadOwner | Normal append/checkpoint/Ready operations cannot be called on that actual type; destructor/flush does not sign the tail |

Compile-fail evidence retains exact source/toolchain, positive-control success,
negative compiler exit and the relevant diagnostic span/reason. A generic
nonzero exit alone is insufficient; a compiler diagnostic is not an executed
durability test. Use the existing locked graph and approved test mechanism;
adding a new UI-test dependency requires a separate graph decision.

The mutation observer must cover truncate, write/append, create/remove/rename,
file sync and directory sync, not only successful durable receipts. Capture the
original bytes, lengths, identities and membership before authentication and
after refusal, and require zero attempted mutations plus unchanged state. Run
the cancellation/expiry case for each admitted repair/bootstrap action and an
unchanged no-repair handoff. Instrumentation-only evidence is L1; repeat on the
actual supported L2 provider before granting native recovery authority. Clock
injection must remain private, monotonic and shared by authentication/finalize;
it cannot become a caller override of the production absolute deadline.

Owner review across architecture, adversarial, operations, data-handling and
evaluation perspectives: the strongest false green is a privacy-only compile
failure paired with a receipt counter that misses failed write attempts. The
positive controls, reason-specific diagnostics and complete mutation observer
above are required to exclude that false green. This refinement is a draft
verification contract, not independent approval or an implementation grant.

Independent X-owner review of immutable15174100 found no new P0/P1 blocker in
this narrow verification-contract refinement. The reviewer confirmed the
positive controls, diagnostic reasons, original monotonic clock and complete
mutation observer requirements. This accepts the source contract only: actual
VerifiedScan, borrow/clock/provider proof and production/execution grants remain
absent; the reviewer ran no workload.

### 2. Preserve physical versus external candidate provenance

Ordinary startup requires exact trusted Checkpoint equality in PHYSICAL decoded
sidecar membership, including writer/witness signatures. Equality means the
existing Checkpoint value contract, not a new JSON-whitespace/field-order rule;
raw transcript checks independently detect byte changes.

Explicit pinned recovery differs: existing plan_witness_recovery virtually
appends a missing externally authenticated checkpoint before verify_history.
Current S1 metadata rejects a missing physical trusted pin, so simply calling it
unchanged cannot implement this recovery. Do not weaken ordinary pin presence.
Add a separately accepted private recovery scan mode requiring independently
pinned witness/client, distinct writer/witness keys, trusted external head and
explicit TailRecovery; allow_new is forbidden there.

Private provenance distinguishes PhysicalCheckpoint from ExternalCandidate.
Candidate exists only if physical latest is absent/strictly older; exact equal
physical latest requires no append; same-size replacement or external behind
local is refused. Candidate previous-writer-signature/root/Merkle/witness checks
use existing shared rules at the exact actual prefix, then ALL complete records
after it must still authenticate. Resource refusal returns no partial result.
RequireCovered refuses an authenticated complete tail; PreserveAmbiguousTail
retains it without signing, replay or effect acknowledgement.

Root-selected ambiguity policy: a positive preserved tail finalizes ONLY into
AmbiguousReadOwner, never Ready/OperationalOwner. It cannot append records,
generate a normal checkpoint or have flush/shutdown auto-sign the tail. Unsigned
current/prefix proofs and previously physical checkpoint proofs remain available
as read-only observations; they are not witness/effect acknowledgements. A normal
later checkpoint or newer receipt cannot clear ambiguity. No into_writable,
boolean override or automatic "latest head now covers it" transition is provided.
Any future explicit business reconciliation/resolution needs a separate accepted
contract, preserving original uncertainty; it cannot be inferred from storage
signature coverage. This is an explicit S2 compatibility restriction requiring
the L3 startup result to distinguish preserved/read-only from writable recovery.
Exact preplanned terminal-torn repair or persistence of the ALREADY authenticated
external PREFIX candidate is separate from normal record append/checkpoint and
cannot sign/cover the preserved tail or grant readiness.

Keep raw sidecar bytes/digest/counts PHYSICAL. Legacy recovery Report/Scope may
include the virtually appended candidate, matching the old algorithm. Carry
separate physical read counts, candidate checks, actual Work and compatibility
Scope; never report candidate verification as a physical checkpoint read or
persisted membership. WitnessCoverage.local_latest_checkpoint remains the
pre-repair physical head; authenticated_external_head is separate. Candidate
cannot satisfy checkpointed_inclusion before confirmed append durability.

Preflight/fatal-recovery error priority must match existing recovery planning,
not just offline verify. Preserve node/progression/pin framing precedence and
record seq/link/hash/HMAC -> base checkpoints -> witness diagnostics/Scope.
Differential fixtures must cover candidate plus multiple simultaneous errors,
late record/base errors overriding witness failures and resource refusals.

### 3. Repair only from the consumed verified owner

Before any write, poll the same operation Control and revalidate held objects,
named membership/root/provider identity, complete raw transcripts/lengths and
exact independent-head binding under the SAME guard. A changed external pin
requires a fresh full scan; do not transplant an old verified summary.

Current source payloads are read-only. Obtain a private writable capability via
secure relative open and SAME retained-object identity/provider comparison while
both handles are alive. No unrelated File+provider label or raw handle bridge.
Count its temporary descriptor in a reviewed write-phase ledger; the read-only
N+P+6 bound is not automatically the write/rotation bound.

Permitted repair actions only:

- Truncate exactly the observed terminal INCOMPLETE record suffix at its checked
  complete offset, after all complete input authenticates. Never truncate complete
  ambiguous records or a nonterminal torn frame. Match exact observed suffix/
  length/epoch immediately before set_len, then sync the file.
- Append the already authenticated external checkpoint when explicitly planned.
  Revalidate existing sidecar or exact admitted absence; create only with secure
  create_new0600 when absent, append complete framed bytes and sync file/directory.
  No new signature or automatic checkpoint covering an ambiguous tail.

After writes, re-observe resulting objects/membership, prove expected transition
and required native durability before OperationalOwner/receipt. Mutation, short
write, failed file/directory sync, cancellation or uncertain namespace transition
poisons/refuses readiness. Drop closes only owned resources. No automatic effect
retry, ambiguous-operation dedup grant or replayed erasure/kill authorization.

Crash cases must include after truncate/before external append, partial checkpoint
append, after file sync/before directory sync and after rotation create. Existing
checkpoint torn-tail is fatal: do not silently introduce checkpoint-tail repair.
Restart may still need explicit operator recovery; fail-closed is not data loss
or successful recovery. Keep original bytes/receipts for uncertain writes.

### 4. Explicit new-history bootstrap

The read-only source requires an existing writer lock. Fresh production history
needs a separate narrow initializer, not implicit artifact creation by inspection.
Require explicit allow_new, existing provisioned keys, trusted private empty WAL
directory, admitted provider and no external head/checkpoint/history. Bounded
enumeration/opened-root admission precedes exclusive0600 lock create; racing
cooperative creators resolve through create_new and one actual guard, never
unlink/recreate a lock. Existing history with missing lock refuses even if a
caller requests initialization. Unknown/co-located files are not "empty".

Lock control-file creation is the explicitly reviewed synchronization bootstrap;
no history repair or key generation is permitted before complete authentication.

Bootstrap restart compatibility: with allow_new, no external pin, valid private
admitted provider and exactly an existing valid control lock but no history,
acquire that same lock and resume empty initialization. This preserves legacy
control-only-directory acceptance. Revalidate ownership/type/nlink/provider and
perform fresh lock/directory sync; never claim an unobserved earlier sync passed.
An active competing guard refuses; no unlink/recreate. A valid existing empty
initial segment with lock can pass ordinary explicit-empty verification. Missing
lock WITH any history, partial/invalid control artifact, unknown member, failed
fresh sync or disallowed new history refuses. Crash after lock create/before
segment thus resumes only through these explicit fresh checks; crash after a
failed/uncertain sync is not automatically a previously completed initialization.
After verified empty history, consume the owner to create initial empty segment
with0600+file/directory sync and expected membership update. Required independent
witness client remains mandatory; first actual record must obtain its normal
checkpoint/ack boundary before release. Preserve SealedLog preflight order,
especially missing witness before consuming unlock bytes or opening state.

### 5. Append and rotation without history collections

OperationalOwner retains only bounded pins, fixed Frontier, sequence/linear head,
latest PHYSICAL checkpoint/link state and trusted context. No records/checkpoints/
leaves Vec or history-sized proof cache. New appends use the shared canonical/
HMAC emitter, WAL append+sync before state advancement, and existing witness/
durability requirement. A record persisted before later checkpoint failure still
poisons the owner; callers must not retry a business effect blindly.

Retain previous writer signature as exact original TEXT, even after successful
authentication. hex::decode_to_slice accepts valid uppercase/mixed-case signature
text, while checks::checkpoint compares prev_checkpoint_hash to the original
sig string. Re-encoding a decoded64-byte signature as lowercase would change the
next signed message/link and exact physical membership. Valid text has bounded
128-byte length, but its case must not be normalized. Include a valid uppercase
writer-signature/next-link fixture and same-size alternate-text pin rejection in
S2 differential tests; preserve the existing verification algorithm, not new rules.

Rotation securely creates exact next segment0600 under the held directory,
preserves contiguous names, admits additional retained pins before opens and
syncs namespace. Never evict/reopen old payloads to evade FD bounds. Update the
expected source epoch/transcript/state only for proven owned writes; unexpected
changes are refusal. Reconcile native read snapshots with intentionally appended
bytes using a new operation lease, not the old read-only stamp after mutation.
No implicit unsupported/unbounded fallback in the required bounded startup path.

At MAXsegments, admission happens before any required rotation create, record
write, checkpoint write or expected-state advancement. If next append needs a
new segment and N==limit, return resource refusal with no write/new sequence.
Appending within the existing last segment may proceed only if all remaining
record/byte/FD bounds hold. Count write reopen/create, sidecar bootstrap and sync
temporaries in a separate transition ledger; do not reuse N+P+6 as a write proof.

### 6. Bounded proofs preserve existing root semantics

| Existing API | Required preserved behavior |
| --- | --- |
| merkle_root | Current unsigned tip over all authenticated complete records, including explicitly preserved ambiguous tail; no automatic signing |
| inclusion_proof | Inclusion at current tip; use ASCII record-hash TEXT leaves |
| inclusion_proof_at | Any checked prefix, even unsigned; None for sequence0/out-of-range or usize conversion failure |
| consistency_proof | Nonempty strict prefix only: old_size0 or >=current returns None, preserving existing Tree behavior |
| checkpointed_inclusion | Exact PHYSICAL authenticated Checkpoint membership, not same size or virtual candidate; signature/size/root binding |

Reuse existing fixed Plan capacity65 and Frontier/RFC6962 rules. No new proof
algorithm or materialized Tree leaves/HashMap in the bounded owner. Derive ranges,
scan in file order with a bounded range frontier, preserve canonical output order
and at most65 path hashes, one selected leaf and requested prefix/current roots.
Verify generated paths with existing verifiers before returning them.

Proof rescan must authenticate complete source through EOF and final native/
transcript/Control barriers; reading only record_hash fields or a trusted prefix
cannot hide later body/cp corruption. Compute internal range projections during
the shared crypto pass, or specify an equally bounded extra authenticated pass.
No proof/physical head escapes an earlier failed range or late record/base/witness
check. Exact owner/key/pin/source binding is required for every proof operation.
O(history) I/O and request-budget refusal are explicit, not a throughput claim.

### 7. Consumer handoff belongs to L3 owners

SealedLog must ultimately use the bounded typed operational owner in its actual
open/recover/proof path, not call legacy Wal::records/checkpoints or VerifiedHistory
Vec wrappers. Keep legacy materialization only with explicit separate budgets;
it cannot establish bounded recovery. Preserve public result shapes where
possible; root accepts cross-owner API/durability differences before migration.

Current read-only consumer map (paths were clean when inspected; no consumer
implementation/qualification is asserted):

| Current consumer | Actual materialization | Required lower contract / upper decision |
| --- | --- | --- |
| runtime/startup.rs::witnessed_through | checkpoints()?.last() | Constant-size last PHYSICAL witnessed head/through from same owner; preserve Some(0) for configured external witness with empty history, None for writer-only |
| runtime/settings/store.rs::open -> store/journal.rs::read | &audit.records()? -> &[Record] | Staged bounded latest revision/digest/parent fold, retain current node/tenant/config-revision checks and reject effects after witnessed_through |
| runtime/control/state.rs::KillStatus::assess | audit.records()?.iter() | Bounded Effect fold, exact scope/commitment/action/authority/other-target checks and fresh retained anchor; no replay or cached health token |
| authority/startup.rs::open_audit sealed branch | SealedLog open -> DurableAudit | Preserve protected credential/witness/preflight/error handling and append-before-release trait; no fallback to inline on bounded provider refusal |
| authority/startup.rs inline branch | Wal + Log direct startup | Explicit compatibility/bounded-migration decision needed; do not silently label unbounded inline startup as qualified production |

The physical getter must not use a virtual recovery candidate or a caller boolean.
It advances only after required signatures, external acknowledgement and physical
checkpoint persistence under the same owner. Pending repair cannot advertise a
new witnessed-through size. Requested bounded fold/physical-head contracts were
sent to Runtime/Authority owners for their acceptance; no API migration or source
write in those paths is authorized by this read-only map.

L3 owner feedback refines the contract without granting consumer writes:

- Settings stages only {revision,digest,previous}; first revision0 then checked
  contiguous canonical decimal <=MAX_SAFE. Preserve exact node/tenant/config
  revision and outer full ManagementScope checks. Match/parent uses commitment
  to FULL canonical snapshot bytes, not revision alone. Existing filter accepts
  only runtime.settings.commit/version1; other settings-family names are ignored.
  Do not silently tighten that to the kill-family policy.
- Kill scans every runtime.kill.* and refuses unknown names/versions or malformed
  fields, including late errors after a valid covered effect. Covered dominates
  later valid ambiguous kill records but does not stop EOF validation. Retained
  exact scope/action/target/public key/other-target-empty and fresh retainer anchor
  join remain serialized outside the fold, not a reusable healthy lease.
- Empty externally configured/verified history remains Some(0), while writer-only
  remains None. An optional physical head None cannot collapse these distinct
  owner states or invent a virtual covered head. No Config/native snapshot
  finalization/Ready before complete EOF+repair/native/crypto barrier under the
  SAME owner/control and absolute minimum caller/operation deadline.
- Authority preserves DurableAudit: Send with append(&mut self, Record)->
  io::Result<()>. Ok means required record/checkpoint AND required witness coverage
  policy actually completed, not queued/healthy/Diagnostic. Append Err permanently
  poisons Authority and leaves its already reserved nonce burned. Authority
  rechecks time after Ok; expiry still withholds response despite persisted audit.
  Uncertain append/witness/flush never rolls intent back or permits blind retry.
- Keep exact-node/trusted-head/allow-new/checkpoint_every1 and private key/WAL/
  witness preflight before unseal/listen. Missing protected credentials/native
  store/witness never falls back inline. Keep redacted stage errors and owned
  Zeroizing credential transfer; caller's original JSON buffer is not erased by
  magic. The inline Wal/Log branch is explicitly COMPONENT/NONPRODUCTION, not
  bounded production; future finite-limit/migration decision remains separate.

Propose scoped read/fold leases that borrow the owner through complete operation,
stage bounded Runtime settings/kill/Authority ledger state, and publish results
only after authenticated EOF. No escaping cursor that outlives owner/guard and
no effects/Ready while folding intermediate records. L3 owns bounded staging,
latest revision/digest/parent and other-target rejection. Merely borrowing a
callback does not prove consumer memory bounds or side-effect freedom.

Linux-only initial provider is a named slice, not whole-goal completion. Windows/
macOS require their own accepted safe opened-object/provider/writable-handoff
proofs. Actual ServiceHost, independent witness, retained dual-control, remote
sink, protected/native role and Docker product gates remain separate. Titles
do not grant writes to Runtime/Authority/native-security/CLI/Lab paths.

### 8. Qualification order and review gates

1. L0 accept this typed ownership/lifetime, bootstrap, virtual/physical, error/
   Scope and proof contract. Independent critique before S2 source; no approval
   inferred from private S1 diagnostics or these owner-authored design notes.
2. L1 pure transition/proof/range/error tests and compatibility vectors. Prove
   rejected state cannot construct/move/deserialize a healthy owner. Preserve
   unsigned proof functionality, counter overflow and precise resource refusal.
3. L2 actual supported files/FDs, same-object writable reopen, guard lifetime,
   creation modes, mutation/crash/sync failure and restart under finite profile.
   Capacity/stack/allocator/RSS/FD observations stay distinct from logical bounds.
4. L3 bounded actual SealedLog + owner-accepted Runtime/Authority migration, no
   complete-record materialization, startup EOF before configuration/effects.
5. L4 supported real host/credential/service pilots; L5 Docker isolation/product
   and cross-OS/source-bound artifact matrix. No component green implies these.

## Record

### Closed hosted prerequisite and sole case catalog (source only)

Root accepted two owned test-only seams requested by Delivery: the committed
native-owner-validation.json sole13-selector/ignored-intent catalog and a closed
precondition entrypoint reusing the existing private NativeSource checks. The
separate native-owner-core-f9b0cda3.json commits all193 baseline relative paths,
byte lengths and SHA256; all193 were checked against both the frozen archive
bytes and canonical f9 Git blob hashes. CI must use exactf9 checkout plus the
declared seam overlay, not label a later whole candidate as unmodifiedf9.
Only baseline native/tests.rs changes for one module declaration; the prerequisite
module and two JSON contracts are additions. The catalog binds these source
hashes, while its own blob is bound externally by the candidate Git identity.

The precondition is ONE explicitly ignored library-test fixture gate before the
13 semantic cases, not a fourteenth semantic pass. It stays behind the existing
test/Linux/x86_64 boundary, exposing no public API or new graph edge. Non-root
matching real/effective UID and an allocated current-owner0700 ext-family parent
are required. The caller must itself create that NEW private parent and retain
its ownership; never pass existing product Audit directories. The gate creates
one uniquely named child with mkdirat0700 and two create_new0600 synthetic WAL
members, then reuses actual held-object privacy/ext0xef53/statxMNT_ID/openat2,
independent read_at readers and ExclusiveFile contention/release checks.

Root explicitly authorizes fresh disposable fixture/bootstrap WAL writes for
this gate and the planned WAL tests ONLY. Existing product Audit data access,
normal native writes, recovery repair and production Ready remain forbidden.
No chmod, process umask mutation, recursive delete, symlink fallback, privileged
helper or subprocess is added. Cleanup unlinks only the two successfully created
members relative to the held new root and removes only its mkdirat-created empty
child via the held parent; unknown entries are never recursively removed. It
syncs and observes name absence. Parent lifetime/cleanup remains with Delivery.
All OS operations remain cooperative; the separate runner owns the original
deadline/process containment and must classify missing receipts/cleanup as unknown.

One fixed AUDIT_NATIVE_PREREQUISITE_V1 JSON receipt is emitted to STDERR under
--nocapture, preserving named libtest stdout accounting. Status distinguishes
observed/refused/unknown, with actual UID/eUID, fixed check keys, first failure
stage/kind and cleanup state. Only observed/all checks true, non-root equal UID,
cleanup removed/name_absent true/no error, first_failure null AND the exact named
one-pass/zero-fail/zero-ignore gate footer/terminal success admits13. Any other
status, missing receipt, failed cleanup or missing footer refuses them. Gate
observations are not an atomic snapshot or a health lease; every semantic case
still performs its own native checks. The gate receipt carries no business or
production authentication authority.

Direct pinned Windows Rust1.99.1 rustfmt, JSON/catalog/source marker and hash
checks pass. This new gate and seam source have NOT COMPILED or RUN. Their own
candidate identity/review/actual hosted results are required in addition to the
f9 baseline; neither old Windows evidence nor Docker shell preflight qualifies it.

### Docker prerequisite refusal and standard-runner handoff

Root explicitly resumed a15-minute Docker-only window for frozenf9b, no WSL or
pull/install/foreign container changes. The existing image inventory offered no
Rust image identity; pinned Ubuntu62d572b9 under admittedUID1000 had no rustc/
cargo in PATH or checked canonical locations. Stop before filesystem admission,
compiler or tests. The shell probe exited0 but the toolchain prerequisite FAILED.
One new owned2CPU/4GiB/network-none/read-only/cap-drop/no-new-privileges container
was observed terminal, removed and absent afterward. No volume/target acquired;
local window returned. Evidence [[evidence:root/20261107165921]], raw164818.

Delivery received and acknowledged the exact frozen source,193 input manifest,
67 package roster,13 selectors and12 planned compiler-attempt contract plus
actualext-family/statx/openat2/private modes and strict counts. Its existing
standardLinux build-only component/caller contract owns workflow/runner changes;
Root owns push/dispatch. No remote run, current qualification, semantic reuse
from unrelated components or private-bundle upload is inferred. All full-goal
native/repair/witness/consumer/capacity/product gates remain open.

### Actual private owner-type slice (source only)

OpeningOwner now owns the retained NativeFiles, actual non-Clone KeyMaterial,
actual Custody guard and owned node/witness/physical-head policy. Its private
existing constructor loads only an already sealed identity; the create closure
always refuses. It rejects an empty node and writer/witness equality. No caller
success flag, transferable diagnostic, file getter or public factory constructs
the owner. Its policy borrows its own keys transiently during the existing full
two-pass native scan, never as self-referential fields stored beside moved keys.
This existing-physical slice has no allow_new option: its transient policy always
sets allow_new=false and the post-scan gate requires Some(report) with report.ok.
Diagnostic None is not successful cryptographic authentication. Empty bootstrap
remains a separate future contract, not a capability of this constructor.

Consuming authenticate(self, original Limits/Control borrows) executes that scan,
requires successful cryptographic diagnostics, ends all operation readers and
moves the SAME whole owner into actual VerifiedScan. Consuming finalize(self)
accepts no replacement arguments, checks the original Control, repeats complete
authentication with those bounds and compares diagnostics/work before returning
an actual private ReadOnlyOwner. That result retains files, material and custody;
it has no append/checkpoint/Ready/Clone/Deserialize or repair interface. This is
a read-only ordinary physical-pin slice, not OperationalOwner, ambiguous-tail
resolution, virtual-candidate recovery, durable receipt or production S2.

The original Control has a private test-compiled monotonic ManualClock seam;
checked nanosecond advancement can only move forward, including overflow refusal.
The default remains Instant-based. One new all-platform clock unit is authored.
The existing ignored controlled-native case now provisions fixture keys outside
the WAL directory, loads them through actual Custody and exercises actual owner
authentication/finalization, retained WAL/key locks, post-auth ORIGINAL cancel
and expiry while a fresh Control stays healthy, and unchanged bytes, object
stamps and membership for both WAL/key directories. Those assertions are NOT RUN.
The current finalizer contains no writable native calls. Unchanged snapshots
are not a complete mutation-attempt observer or proof for future writable repair;
that stronger gate remains open for every real write/bootstrap/repair action.

Actual-type compile probes live under native/ownership/probes: one positive and
ten negatives, with exact expected error codes and primary-span text in cases.json.
They are intentionally not normal Cargo modules. An explicitly allocated Linux
validation must inject ONE fixture as ownership/contract_probe.rs into a frozen
copy of this actual crate, append a test-only allow(dead_code) contract_probe
module declaration to the copied ownership.rs, then run locked/offline Cargo
test --lib --no-run with JSON diagnostics. No replacement owner definitions,
public visibility changes or dependency edges are permitted. Record the exact
two-file overlay and each fixture hash. All probes import the actual parent
types and methods, so privacy/import failures cannot qualify them.

Require the positive fixture to compile first and again after all negatives.
For each negative require unsuccessful Cargo exit, the manifest error code and
a primary span in contract_probe.rs containing the recorded source text; every
error-level diagnostic must be an expected probe error, with no unrelated crate,
privacy, import or dependency failure. Preserve actual argv/stdout/stderr/exit
and source/tool/config closure for each phase. These are compile checks, not
11 executed Rust tests. Never inject probes into the shared checkout or use the
outer Cargo target recursively. A missing supported venue remains NOT RUN.

Direct pinned Windows Rust1.99.1 rustfmt parses/formats the authored modules and
all11 probe sources; inventory/JSON/expected-span checks passed. No compilation,
Clippy, actual native test, probe or product execution qualifies this source.

Independent X-owner review of1728b3a2 found no P0/P1 blocker for this private
read-only direction, subject to all actual gates. Its P2 empty-history clarity
finding is resolved above by removing allow_new and requiring Some(ok). The
native fixture now authors an explicit positive S1 empty diagnostic(None) versus
negative actual-owner authentication of that SAME empty history, with no writes
or successful owner result. Additional real-owner negatives cover foreign node,
writer/witness alias, a deliberately invalid record HMAC reaching the completed
diagnostic-failure branch, missing sealed identity refusing creation, and a
post-auth file mutation refusing finalization without automatic repair. These
are six additional logical subcases inside the existing ONE ignored controlled
Rust test, not six separately executed tests; all remain NOT RUN. Fixture-only
byte mutations and restoration occur outside the owner's read-only operations.

### Older-source Windows six-case qualification

Exact frozen8cbc passed ordinary-user Windows Rust1.99.1 check/strict Clippy,
changed-module rustfmt and six exact tests: facade preflight/Unsupported plus
five pure transition cases, each1pass/0fail/0ignored/102filtered. Source195,
registry3249files/75packages, selectedtools56, config52absent and eight environment
hashes unchanged; terminal45327 and target returned before closure. Evidence
[[evidence:root/20261107162841]]. This qualifies neither20d573 NativeFiles nor
subsequent original-clock/actual-owner source. Linux/native/capacity/compile-fail,
writable recovery, production and consumer gates stay open.

### Owned native resource prerequisite (source only)

Root accepted the actual private owner-type slice after15174100/88cc4ba9 review,
including a lower resource-lifetime refactor when needed. The previous Context
stored every native pin/guard beside borrowed startup Limits/Control, so moving
that Context into OpeningOwner would retain the startup borrow. NativeFiles now
owns the pins, inventory, original descriptor reservation, poison state and
last-dropped guard WITHOUT operation borrows. Context borrows only current bounds
and retains Arc<NativeFiles>; existing readers retain that same Context.

Consuming NativeSource::into_files refuses while any operation reader remains;
those readers keep the guard on refusal. A subsequent private operation validates
its bounds, descriptor reservation, retained count/byte limits and poison state
before re-opening names for the existing unchanged barrier. This is PRE-auth
resource reuse only, not permission to replace VerifiedScan's original bounds.
No raw handle, mutable File, production factory or recovery authority is exposed.
The extra Arc allocation/reference count belongs in the eventual memory ledger;
no added native descriptors or lowered reservation formula are claimed.

The existing explicitly ignored controlled-native test now also authors a real
NativeFiles positive lifetime control: local opening bounds end before files are
returned, the retained guard blocks a second WAL, a fresh PRE-auth operation runs
the existing two-pass scan, and an outstanding reader refuses resource handoff
while keeping its readable data/guard until the last drop. These new assertions
have NOT RUN. Direct Windows Rust1.99.1 rustfmt and diff checks passed; no compile,
Clippy, native, capacity or product qualification is claimed for this refactor.
Actual OpeningOwner/VerifiedScan types and their gates remain the next step.

### Private L1 transition scoped qualification

The test-compiled scan/recovery_transition.rs now implements pure tail
classification, preserved-tail rejection of every normal append/checkpoint/Ready
intent, checked segment rotation admission, explicit control-only bootstrap
decisions and OperationBounds borrowing the original Limits/Control with sticky
refusal. These are LOCAL planning/precondition results only: no NativeFiles,
VerifiedScan, successful authentication, real write, durable receipt or readiness
capability is constructed. The original unavailable operational factory stays
Unsupported. No S2 production source or consumer mutation is enabled.

Five authored invariant tests cover preserved-tail immutability across all
normal intents, zero/u64MAX/ahead-anchor boundaries, MAXsegments before a new
index, explicit allow-new/control-only/bootstrap negatives and original cancelled
Control refusing even with a fresh replacement/reset. Exact8cbc passed all FIVE
in individually selected GNU1.99.1 harnesses, plus fmt/check/strict Clippy in
[[evidence:root/20261107155745]].177 selected source inputs,3181 registry files,
73 selected Rust tools and44 absent config candidates unchanged; terminal36565,
A returned. No native/capacity/facade behavior was executed in this gate.
Actual VerifiedScan type compile-fail,
post-auth expiry/no-write and actual bootstrap/repair/durability gates remain
distinct requirements. Earlier3d325 proof qualifies none of this new kernel.

Independent reviewer reread the v0.2 contracts and found no further P0/P1 blocker
in those refinements for Root's narrow private L1 model. This is not acceptance
of S2 production/native repair, full memory or effect authority. Required actual
type/method/clock/readonly/bootstrap tests above are not waived.

Owner multi-perspective review, not delegated or independent approval:

- P0: startup-borrowed Control cannot become a long-lived writer deadline.
  Require owned NativeFiles + operation leases and consuming whole-owner typestate.
- P0: virtual recovery candidate cannot acquire physical proof membership.
  Require distinct provenance/counts, ordinary versus explicit recovery modes and
  persist-before-physical promotion; preserve compatibility Scope separately.
- P0: reopening writable path after scan can target a substitute object.
  Require simultaneous held-object/provider comparison under same guard, exact
  pre-write revalidation and poison on uncertain durability.
- P1: bootstrap missing-lock rule and mutation-phase FD peak are not covered by
  read-only qualification. Require separate initializer/write ledger and actual
  negatives before enabling production startup.
- P1: valid prefix/range proof could mask late corruption or silently remove
  unsigned functionality. Require full EOF auth and preserved API root matrix.
- P1: a fold callback can allocate unbounded state or apply effects early.
  Require consumer-owned finite staging/EOF handoff and L3 review, not a bool.

Verdict: needs-work. Production implementation and resource/execution grants are
not authorized by this draft. Frozen scoring/a11y/defense-only constraints stay.

Independent X-owner review of61bb found three unresolved transition contracts;
Root directed the v0.2 requirements above: read-only preserved-tail outcome;
original &'op Control/Limits through finalize; and resumable control-only
bootstrap with fresh validation/sync, plus pre-write MAXsegments admission.
These are draft-contract findings, not observed production bugs. Root accepted
the lowest private L1 transition-source slice after these refinements; no actual
native repair/owner/authority promotion or compile slot follows from that grant.
