# Native runner integration

Rust1.99.1/edition2024 independently locked tool; root Cargo is unchanged.
Build: `cargo build --locked --manifest-path scripts/ci/windows-native-runner/Cargo.toml`.

From the repository root, invoke the built executable with literal arguments:

```
humanymous-native-runner native --report ABSOLUTE_NEW_REPORT --binaries ABSOLUTE_ROOT/target/debug
humanymous-native-runner standard-user --report ABSOLUTE_NEW_REPORT --binaries ABSOLUTE_ROOT/target/release
```

The report parent must already exist. `native` works on Windows/Linux/macOS with
an actual non-elevated installer token/non-root UID. `standard-user` is Windows
only and refuses before provisioning unless GitHub-hosted and elevated. Local
nonadministrator machines never create accounts. Report paths are not secrets.

`--binaries .../release` builds/tests the release profile; other directories use
debug. Paths and SHA256 must equal actual Cargo JSON product artifacts, so a
prebuilt/stale release binary cannot be relabeled. Prepare compiles and executes
all harnesses for runtime, authority, CLI and native-security only, including the
named actual-process integration assertions. This is not the all-workspace probe
replacement and does not waive the legacy all-harness requirement. The parent
standard-user mode prepares first, then creates a Users-only local account,
uses native CreateProcessWithLogonW suspended launch into a kill-on-close job,
clears the environment to an explicit minimal block, checks actual SID/token
and workspace write denial, and deletes the account/scratch before reporting.
Its privileged execution still requires actual disposable hosted qualification.
The narrowly allowed FFI lives only in this CI tool, with ownership/pointer
invariants documented in actor.rs and cross-os-runners SoT. No shell source.

Schema1, kind=native, current source SHA, reviewed-document digest, source-file
manifest, actual OS/arch/token, Cargo manifests, per-harness SHA/counts/exit and
real CLI snapshot checks are emitted. Source changes during execution fail.
Product algorithms/signing/scoring are not copied: source-bound integration
harnesses execute their existing actual native children and shared APIs.

Exit2 means component execution completed but full delivery is blocked. The
report explicitly misses local-only-admin and documentation-examples, with
documentation_examples_executed=false and full_runtime_qualified=false. Exit1
means infrastructure/assertion/refusal error; unknown ignored/filtered tests fail.
No path returns full-qualified exit0 yet. Raw build stderr/messages use the report
stem with `.build-stderr.log`, `.build-messages.jsonl`, `.tests-stderr.log` and
`.tests-messages.jsonl`; retain privately. Credential-bearing runtime stdout is
parsed in memory and never copied to report/debug output. The document hash is
an identity reference, never a claim that reviewed examples executed.

The completed local Windows debug source snapshot ran10harnesses/51tests,
zeroignored, with required runtime/authority native-process assertions. It remains
blocked and dirty-source scoped. Linux/macOS and hosted account/job execution
are not yet observed; request exact-source Actions through root/CI owner. OS
service activation belongs to CLI; browser, positive raw TLS/H2, actual remote
machine peer denial and Docker product qualification remain separate blockers.

No push/tag/publish/main changes are performed by this lane.

## Signed service continuation and explicit pending test

Prepare now enables CLI's `service-pilot` feature. The exact required privileged
`native_service_install_upgrade_rollback_and_remove` test is not a helper skip:
default native execution preserves its raw ignored count in `skips`, records
the exact `required_not_run` name and remains blocked. No generic ignore exception
is added. Actual signed-plan/native-readiness and ordinary-SCM-denial tests do
execute. New observed local snapshot is12/12harnesses,55passed,1required ignored,
errors0,skips1,exit2. Later source manifests remain separate.

`service --report ABS --binaries ABS_ROOT/target/debug|release` is an opt-in hosted
native manager pilot executing exactly that ignored test with its explicit
disposable-actor flag. Windows requires an ordinary token with externally scoped
SCM creation rights; direct elevation is rejected because product installer
checks remain nonadministrator. Unix requires actual non-root user service
manager support. Missing manager/delegation fails. A successful fixture remains
component-only and does not qualify runtime service readiness or full delivery.
The temporary SCM delegation/restoration bootstrap is implemented in the later
section below, but is not remotely observed. Root accepted its isolated feature
delta; no shared manifest/lock change follows.

`summarize --source ABS_PRIVATE_REPORT --report ABS_PUBLIC_SUMMARY` publishes typed
counts/source commitments only, excluding actual SID, credentials and traces.
It returns0 only for writing this separate summary schema; native/service modes
still cannot produce full-qualified exit0. Source evidence distinguishes validated
worktree SHA256 from canonical Git object SHA256 and checks LF-only own sources.

## Hosted SCM actor contract (implemented; execution not_run)

Root approved only this isolated tool's existing windows-sys Services feature;
no shared Cargo/lock delta. Use `standard-user-service --report ABS --binaries
ABS_ROOT/target/release` on an elevated disposable hosted Windows VM. The parent
creates an ordinary Users-only actor and delegates SCM CONNECT|CREATE(0x3) through
a captured manager DACL; the child runs `service` with actual ordinary SID/token.
It restores/readbacks the exact original DACL before account/scratch deletion.
Concurrent/unknown/inherited SCM ACLs are refused; namespace restriction is CLI
policy, not a falsely claimed OS per-name grant. No local SCM execution allowed.

Negative: append `--actor-case after-grant`. It intentionally exits1/statusfailed
before launch; require actual `standard_user_lifecycle.account_deleted`,
`scratch_deleted` and `scm_delegation.restored` all true to prove cleanup. Absent/
false facts fail. Normal mode additionally observes owned job active-process count0;
SCM-started processes are outside that job and CLI must prove service stop/delete.

An earlier local release diagnostic13harness/62pass/rawrequiredignored1/errors0/blocked2
is source-specific; no hosted service/account or Linux/macOS success is inferred.
New raw logs use explicit reportstem.harness-i-{list,run}-{stdout,stderr}.log and
errors identify failed index/target/exit/discovered. Copy only these expected names
from child scratch; never copy account profiles or arbitrary temp files. The prior
release failure at a different owner/source remains a failure, not overwritten.

Newer source e30e0e9+dirty produced a real authority local_service failure:
index9,exit101,2discovered,1passed/1failed. Its report/logs remain failed. Authority
owner diagnosed reactor cleanup in the fixture and supplied a source repair;
exact-source rerun is required after common lock resolution, not inferred from
the prior62-pass snapshot. The runner now retains compiled manifests and partial
coverage(counts,failed/validated flags,complete=false) even on early failure.

## Whole-workspace component migration (separate qualification)

Use `workspace --report ABS --binaries ABS_ROOT/target/debug|release` on native
ordinary/nonroot actor; Windows elevated hosted parent uses `standard-user-workspace`.
Same account/process/job implementation, Scope switches the single prepare/
manifest/accounting implementation to default Cargo --workspace --all-targets.
No extra service-pilot feature. Every discovered executable is hashed and run.

Schema `humanymous.workspace-probe/1`, kindworkspace, qualificationimplemented-
workspace-component, full_runtime_qualifiedfalse. Only this separate complete
unit/component scope may exit0/statuspassed; it cannot satisfy native schema1.
Generic ignored/filtered/failed/missing work fails. Preserve known audit key_custody
2rawignored helpers only with exact package/target/kind, exact ignored inventory,
required parent successes and one strict outer summary. The owning parents
verify bounded child capture, exact static markers, exit status, stderr and state;
nested child summaries must not escape into the outer result.
Native4package service-feature still returnsblocked2 with1required service test
not_run. Parent source snapshot spans all crate source/manifests and the expected
runner architecture must equal the compiled executable architecture.
Audit captured-helper contract: only `humanymous-audit` integration target
`durable_witness` helper `child_witness_probe` with parent
`witness_process_lifetime_and_interrupted_recovery`, or `key_backups` helper
`child_retire_without_pin_acknowledgement` with parent
`crash_reconcile_preserves_consumed_authorization`. Require exact ignored1,
parent passed, exit0, one summary, passed+1=discovered, no failed/measured/filtered.
Parents must capture/assert actual child status, marker and state/recovery.
Raw ignored helper counts remain separate from passed counts. Old nested child
summaries fail this contract. These add two to existing custody helper count2.

Publication hardening (new source, hosted not_run): passed workspace summaries
reject missing/null/nonunsigned counts, nonzero/absent exits, inconsistent required/
executed/row/discovered denominators, unclassified ignored and overflow. Zero-test
harnesses remain in the denominator. Actor-backed passed summaries additionally
require attempted child exit0 and observed successful cleanup.

Sanitized summaries retain typed `standard_user_lifecycle`, `scm_delegation` and
`service_execution` facts without SID, SDDL or paths. Missing stays null; false
cleanup stays false. For `standard-user-service --actor-case after-grant`, injection
precedes child launch: expect exit1/statusfailed, child_launch_attempted=false,
child_exit_code/job_empty_observed/job_kill_on_close=null. Account and scratch
deletion plus SCM restored=true remain required. Positive returns2/statusblocked
when its service component passes but other native requirements remain missing;
it requires actual child exit2, job empty and cleanup/restore evidence. These two
cases must use separate report paths and be consumed as separate evidence.

Initialization failure after option parsing preserves a new failed report only
when no report exists. Source and harness denominators remain null/unknown, never
zero-pass success; existing raw reports are not overwritten. Public projection
keeps only typed initialization facts and an optional OS error number.

Build observations are separate from execution denominators. Each existing Cargo
prepare command records compiler-artifact event counts (dependencies included),
fresh/rebuilt/unknown and cargo_child_elapsed_ms. Parent input fingerprints include
recorded source files, root/runner locks, declared toolchain, OS/architecture/ABI,
profile/features and hashed flag/override inputs. Actual compiler version and all
Cargo configuration inputs are not proven; input_key_complete/cache_reuse_proven
remain false. No prebuild/discovery command is removed. Raw Cargo JSON and a build
observation sidecar remain private; --timings HTML is private and retrieval is
not claimed. Public summaries whitelist labels/counters/digests, never raw flags or
paths. These new observations and initialization fallback await pure/hosted checks.

Actual runtime service hook (new source, not yet hosted): native prepare enables
both humanymous-cli/service-pilot and humanymous-cli/runtime-service-pilot. Service
mode executes generic service_process plus runtime_service_process exact ignored
native_runtime_service_lifecycle_and_missing_credential_fail_closed. The latter
requires Windows; other-target source reports unsupported work explicitly. Default
native retains both required-not-run entries. These are product pilots, not helpers.

Runtime controller inputs to the selected test are nonsecret
HUMANYMOUS_SERVICE_PILOT_RUN_ID/HUMANYMOUS_SERVICE_PILOT_SOURCE_SHA from the verified
snapshot. The test authenticates its signed bootstrap and independently checks
SCM PID/status/nonce/executable/receipt. It emits one leading-newline JSON record:
schema humanymous.runtime-service-pilot/1, case native-runtime-service-lifecycle-and-
missing-credential, exact source/run and product_qualified=false. Nine facts in
service/receipt.rs must all be true; duplicate/missing/false/unbound records fail.
Output counts remain selected1 with discovered-1 filtered and exit0. Summary adds
runtime_service_execution beside the unchanged generic service_execution field;
raw PID/path/error data is not projected. Both logs are copied before actor cleanup.


The runner continuation checkpoint distinguishes execution_complete from successful
coverage. Completed child stdout with invalid strict accounting records an indexed
failure and permits later harness receipts; identity, enumeration, spawn, capture,
containment or persistence failures stop collection. Native named-case receipts are
retained before aggregate accounting rejection. Ignored Node product tests remain
required, never helpers. Only the exact shared subprocess helper with all six
parent tests can be accounted separately. Coverage sidecars are written after each
completed child; parent recovery when the final report is absent is not wired yet.

The runtime service receipt now requires thirteen true facts, including actual
upgrade, rollback, replay floor and complete readiness manifest verification. The
older nine-fact receipt cannot qualify this contract. Actual hosted execution is
not_run for this source. Root-assigned D validation passed 21 pure tests, zero
failed/ignored and one excluded filesystem/ACL fixture, plus all-target check and
Clippy with warnings denied. Forty source/manifest/lock/toolchain files had zero
final-run drift. Initial visibility and Clippy failures were corrected and retained
in private logs. These checks do not qualify process, directory, WinCred or SCM
behavior. Shared ordinary-user directory dependency compilation was independently
observed by the .NET owner; actual initializer execution remains not_run.


Parent recovery is now implemented for an absent final report. Before deleting the
scratch directory, the actor preserves the bounded private coverage sidecar and
strictly checks its denominator, sequential row indices and executable/package/
target identities against the owned manifest. Known counts and null unknowns are
retained; recovered aggregate coverage and execution completion are always false,
with fatal interruption recorded. Recovery errors propagate after explicit SCM
restoration. The earlier unwired-recovery limitation is superseded for this source.
Actual timeout/cleanup/SCM execution remains not_run. Root-assigned D validation
passed23 pure tests, zero failed/ignored, one excluded actual filesystem/ACL test,
all-target check and Clippy. Forty-one closure files had zero before/after drift.

Earlier hosted8132117 failed on all three OS native actor jobs at host_process:
Windows15/89 harnesses55passed; Linux/macOS15/88 harnesses51passed. Each observed
zero failed Rust tests, four raw ignored tests, one owner error and incomplete
coverage. Those ignored cases are one verified audit helper and three required
Node product tests; no helper exemption applies to the latter. Separate MSRV
workspace execution continues independently, so this actor observation does not
mark every later row globally not_run. Owner raw reports remain not retrieved.


Pending source-only accounting follow-up during the root benchmark quiet window:
Linux authority's ignored privileged parent is explicitly required_not_run in the
privileged-docker scope; its child probe is declared but contributes zero verified
helper exemptions. Public projection retains whitelisted native assertion names,
boolean verification, row indices and indexed failure codes. It drops arbitrary
private diagnostic strings. Summary accounting uses checked cardinality addition,
requires exact libtest labels and a finite nonnegative elapsed field, and avoids
summing untrusted failure counts. Five new pure tests are authored but NOT_RUN;
format/check/Clippy validation is pending the next root allocation. No compilation
or runtime process was started for these additions. The validated ae5891d7 source
remains the committed benchmark candidate. User authorization now permits actual
local comparison; no account/SCM provisioning profile or provenance bypass is
inferred from that authorization, and no such local run has occurred here.


Pending paired custody capture contract: audit owner is replacing both unbounded
inherited-stdio child launches with bounded capture, exact static child markers,
exit/stderr checks and the original state assertions. The runner accepts only the
exact two ignored helper names, both actual parent successes and one outer final
summary. An old leaked nested summary is rejected. This changes evidence framing,
not the required product assertions or ignored-test policy. Historical813 evidence
keeps its old contract. Updated source/tests are NOT_RUN during the root benchmark
quiet window; no compiler or runtime validation was started for this follow-up.


Pending runner source is paired with audit helper-only checkpoint de33864b:
60-second/64-KiB bounded child capture, exact newline-framed flushed effect
markers, original and stronger custody state assertions, unchanged two ignored
helper names. Runner rejects leaked markers and nested summaries and requires
both parent successes plus one outer summary. Formatting is now permitted and
applied; new pure28/check/Clippy remain NOT_RUN under the benchmark allocation
hold. Historical813 results do not qualify this contract. The goal remains
blocked on validation allocation and actual SCM lifecycle evidence.


Shared Docker affinity source follows rust-migration's Local Docker engine identity
and cleanup canon. test-support::docker is a pure, dependency-free comparison seam:
DesktopObservation/ServerIdentity, verify_desktop and verify_sdk, sanitized proof
projection and static error variants. Desktop context, canonical named-pipe endpoint,
Linux OS and bounded nonempty private metadata are mandatory. Observed server ID
and version must exactly match the actual caller-observed Desktop baseline. SDK
host is explicitly allocated and must match the actual configured host; actual SDK
server identity must also match. An allocated Unix proxy is a distinct endpoint,
not renamed as the Desktop named pipe. No IDs/versions are hardcoded as universal
facts, and equality is not authenticated provenance. Wrappers retain I/O and private
raw evidence. Lab accepted the typed contract. Four new pure tests are authored,
not executed; formatting/diff checks only, no Docker/compilation/provider execution.
