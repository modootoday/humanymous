---
id: sot:root/cross-os-runners
title: Cross OS runner qualification and infrastructure research
status: active
created: 2026-11-06T22:37:47Z
updated: 2026-11-07T05:56:42Z
verify: manual
rel:
  depends_on: [sot:root/delivery, sot:root/adapter-ecosystem]
---

# Cross OS runner qualification

Document version: 0.4.0. Research checked 2026-11-07 KST. Accepted execution
design; remote results must be attached separately. This does not certify the
unfinished Rust runtime or change detection scoring.

## Problem and decision

Windows/WSL development does not prevent remote native Linux, Windows and macOS
testing through GitHub Actions. Native runtime, Linux Docker product lab and
privileged deployment qualification are distinct execution contexts. Every
claimed supported tuple needs actual execution, not cross-compilation or a
renamed artifact. WSL/Linux containers cannot prove Windows DACL/named-pipe or
macOS filesystem behavior. [R1, R2, R11]

Use three layers:

1. Standard GitHub-hosted fresh VMs for unprivileged native compilation, unit,
   process, CLI and Node/Bun compatibility tests, without secrets or OIDC.
2. Linux VM Docker lab for isolated Core/Gate/browser/Pass/nginx/attacker scenarios.
   Preserve Docker-only product e2e authority and human/a11y controls.
3. Trusted disposable dedicated native machines for multi-user/remote-client,
   privilege/service/reboot/filesystem/fault-injection cases hosted images cannot
   represent. Their management plane is separate from attack workloads. [R1-R6]

## Runner matrix and support claims

These are available-label candidates observed in official documentation, not
permanent availability promises or a declaration that every tuple is shipped.
Pin an OS-generation label and record the actual image/version at execution.

| Tuple | Hosted candidate | Intended verification |
| --- | --- | --- |
| Linux x64 | ubuntu-24.04 | Native Rust/CLI/UDS and Linux Docker lab |
| Linux arm64 | ubuntu-24.04-arm | Actual ARM execution; ARM product images separately |
| Windows x64 | windows-2025 | Native Rust/CLI/process lock and future pipe/DACL |
| Windows arm64 | windows-11-arm | Actual ARM execution, dependency availability recorded |
| macOS Intel | macos-15-intel | Native Intel Rust/CLI/UDS and Node/Bun |
| macOS arm64 | macos-15 | Native Apple Silicon equivalents |

The current three `*-latest` native jobs do not prove both CPU architectures.
`latest` tracks GitHub's selected image and can migrate independently of vendor
latest. `ubuntu-slim` is not a general VM alternative for privileged Docker labs.
Hosted Windows is elevated/UAC-disabled; it does not prove a normal user's
installation boundary. macOS ARM hosted nested virtualization is unsupported;
do not require Docker Desktop inside those runners. GitHub container jobs,
Docker actions and service-container execution require Linux runners. [R1, R2]

## Required verification contexts

| Surface | Actual evidence required | Current gap |
| --- | --- | --- |
| Rust/native | Stable and declared MSRV tuples, executable target and host architecture | Local and workflow declarations are not remote success |
| Installer | dry-run no writes, digest, repeat/rollback, partial failure, symlink/reparse/private parent, ordinary-user boundaries | CLI provisioning exists; service activation does not |
| WAL/audit | separate-process lock, torn terminal recovery, framed corruption, tamper/HMAC, checkpoint failure, witness rollback | Native Windows tests exist; other OS execution and power-loss proof missing |
| IPC | Unix peer UID/owner/mode and other-user denial; Windows token/PID/DACL/remote rejection | Rust decision service not yet wired |
| Management | actual local peer plus authentication, credential separation, dual-control and nonlocal rejection | Adapter headers are not locality evidence |
| Node/Bun | Express/Hono on each supported tuple, real Rust authority, body/header/target binding, origin once, replay after restart | Current CI Node job is Linux; mock pilot is compatibility only |
| nginx | Supported Linux nginx/njs versions, real authority and compatibility negatives | Current Linux mock UDS slice is not Node protocol/full product |
| Browser | Collector/Pass/keyboard/a11y/human controls in Docker product lab, native browser supplement | WASM/runtime/browser runner incomplete |
| Wargame | Owned target binding, actual egress/direct-origin denial, honest attack/control denominators | Current network-none engine simulation is not live Gate lab |
| Release/Pages | Exact-SHA required matrix and docs review; protected isolated OIDC publish | Registry/provisioning gates still open |

Windows named pipes require explicit restrictive DACL and
PIPE_REJECT_REMOTE_CLIENTS. A controlled second Windows machine is necessary to
claim observed remote-client rejection. APFS case-sensitive/insensitive variants
need separate path cases. nginx Windows is beta with feature limits, not an
implicit supported target. Playwright WebKit is not the installed Safari channel;
automated accessibility checks cannot replace assistive-technology/user testing.
Signing/notarization needs its own protected account/certificate setup. [R7-R10]

Case inventory includes `native-platform-attestation`, `installer-repeat-rollback`,
`installer-private-parent`, `installer-link-reparse-reject`, `installer-partial-failure`,
`ipc-authorized-peer`, `ipc-unauthorized-peer-denied`,
`ipc-admin-credential-separation`, `pipe-reject-remote-clients`,
`pipe-other-user-denied`, `dacl-protected-non-elevated`, `uds-mode-owner`,
`uds-other-user-denied`, `audit-process-lock`, `audit-torn-terminal-only`,
`audit-framed-corruption-closed`, `audit-witness-rollback`,
`audit-checkpoint-failure-blocks-action`, `adapter-rust-authority-binding`,
`adapter-body-header-target-binding`, `adapter-replay-after-restart`,
`adapter-origin-once`, `adapter-reserved-route-collision`, `adapter-stream-policy`,
`lab-egress-denied`, `lab-direct-origin-denied`, `lab-installed-target-binding`,
`human-control-all-allow`, and `a11y-zero-motor-control`. Preserve existing delivery
cases; do not replace unfinished cases with empty success wrappers.

## Dedicated fleet and trust boundaries

Prefer ephemeral/JIT one-job registration plus controller-enforced VM/disk
destruction and external lifecycle logs. Deregistration is not secure erasure or
host reset. Runner labels schedule work; they do not authorize access. Group and
workflow access controls must be verified for the actual account. Public fork
code must not execute on a persistent privileged fleet or access its management
network. Environment approval/secrets masking does not provide isolation. [R3-R5]

Linux ARC is a Kubernetes autoscaling option. Do not claim current official
cross-OS ARC parity from legacy/community Windows recipes. Windows/macOS fleets
need independently validated orchestration and image lifecycle. macOS VMs require
Apple hardware and the applicable version's license review. Linux containers on
Docker Desktop remain Linux guests; Windows Server is not a supported Docker
Desktop install target. Dedicated hardware/VM subscriptions, artifact storage and
maintenance have costs even where self-hosted Actions usage is free. [R3, R6]

Separate dependency acquisition from isolated attack execution. Permit only
required GitHub management/artifact endpoints to the runner control plane; deny
attack workload access to production, credentials, metadata and general egress.
Use actual negative network assertions, not a report's self-declared isolation.
Publish on a different fresh runner with minimum OIDC scope and exact SHA/digest
checks. OIDC removes long-lived credentials, not malicious-job risk. [R4, R5]

## Evidence and readiness contract

Retain same checkout SHA, documentation review digest, zero required skips/errors
and immutable image identities. Version report schema before adding required
tuple fields; current schema 1 does not validate full runner provenance.
Record expected and observed OS/release/kernel/architecture/target, filesystem,
native/VM/container boundary, IPC transport, privilege/token context, runner
environment/image/version, toolchain/Node/Bun/browser/Docker versions, lockfile
and executable digests, workflow SHA/run/job/attempt URLs, case assertions and
trace digests. Dedicated controller lifecycle evidence is separate from code's
self-report. Redact secrets, raw cookies, private keys and entity bodies.

The aggregator must compare expected required tuple sets with actual verified
success sets, including duplicate/missing tuples and artifact identity mismatch.
Track passed/failed/unavailable/unsupported/not_run distinctly. Required unavailable
or skipped work blocks qualification. Mock, engine, native and Docker-runtime
evidence have different types and cannot substitute for each other.

PR runs cover a bounded critical matrix; expanded nightly and release runs cover
all supported tuples. Do not exclude costly or unavailable required cases from
release aggregation merely to obtain green status. Larger runners, network access
and paid machine provisioning remain explicit account prerequisites. [R12]

## Current remote state and rollout

Read-only gh API snapshot: public `modootoday/humanymous`, default branch main,
Actions enabled; repository-visible self-hosted runners count zero. This does not
prove organization fleet availability. The local Rust reusable workflow is not
listed among the remote default-branch workflow records. Current workflow token
default is write; new probe explicitly overrides permissions to contents read.
Do not change repository-wide settings or publish main as part of diagnosis.

User authorized gh-based setup after the research request. Add a manual six-tuple
diagnostic workflow on codex/rust, push that branch under the Git mutex and run
it against its exact commit. Its scope is current implemented crate tests and
platform observation, not full delivery readiness. Preserve failures and actual
job URLs in evidence. If GitHub requires default-branch registration before a
manual workflow can dispatch, record the limitation and use a branch-scoped
push trigger rather than merging incomplete Rust into main.

## Critique reflected

Blue: one shared authority, no score-policy change. Red: no persistent public-PR
fleet, no target/egress escape. SRE: explicit image lifecycle and unavailable jobs.
DPO: bounded synthetic fixtures and sanitized artifacts. Evaluator: native CPU
execution differs from cross-build, WebKit differs from Safari, process restart
differs from power loss, mocked adapters differ from product integration.
Verdict: design ready for staged setup; full product qualification remains blocked
by the explicitly listed runtime and deployment gaps.

## Sources

- R1: https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- R2: https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/run-jobs-in-a-container
- R3: https://docs.github.com/en/actions/reference/runners/self-hosted-runners
- R4: https://docs.github.com/en/actions/reference/security/secure-use
- R5: https://docs.github.com/en/actions/concepts/security/openid-connect
- R6: https://docs.github.com/en/actions/concepts/runners/actions-runner-controller ; https://github.com/actions/actions-runner-controller ; https://www.apple.com/legal/sla/ ; https://docs.docker.com/desktop/setup/install/windows-install/
- R7: https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights ; https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createnamedpipea
- R8: https://support.apple.com/guide/disk-utility/file-system-formats-dsku19ed921c/mac ; https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- R9: https://nginx.org/en/docs/windows.html
- R10: https://playwright.dev/docs/browsers ; https://playwright.dev/docs/accessibility-testing
- R11: https://learn.microsoft.com/en-us/windows/wsl/compare-versions ; https://docs.docker.com/desktop/features/vmm/
- R12: https://docs.github.com/en/billing/concepts/product-billing/github-actions ; https://docs.github.com/en/actions/reference/limits

Pricing is an observation, not a fixed project constant: standard public hosted
and self-hosted Actions usage are currently free; larger and underlying fleet
resources may be billed. Verify account policy and current pricing before paid
provisioning. No paid fleet was provisioned by this research.

## Standard-user Windows implemented-test execution

On disposable GitHub-hosted Windows only, compile all workspace/all-target test
harnesses as the runner administrator using Cargo JSON compiler-artifact records.
Deduplicate actual test executable paths and retain their package/target identity
and SHA256. Execute every manifest harness under a newly created local account
whose only local group is built-in Users (SID S-1-5-32-545). Never weaken CLI DACL
or token predicates. Fail if provisioning, logon, token verification, enumeration,
execution, cleanup or complete manifest coverage is unavailable.

Use Microsoft New-LocalUser with an ephemeral SecureString password and .NET
ProcessStartInfo with UseShellExecute=false, UserName, local Domain, Password,
LoadUserProfile=true and CreateNoWindow=true. Clear the environment and explicitly
supply only Windows runtime paths and account-owned temporary storage. Verify the
child's actual SID, absence of Administrators membership and token elevation via
GetTokenInformation(TokenElevation); configuration alone is insufficient. Grant
the account workspace read/execute only and a separate protected scratch tree.
Do not copy the runner environment, credentials, Git configuration or toolchain
home into the child. Retain only enumerated harness logs/results; delete the
account in finally. No WinRM, scheduled tasks, UAC/security policy modifications.

List each harness before execution, retain the discovered test count and compare
it with the execution summary. Zero-test binary harnesses remain explicit and
must have a successful zero-test summary; they cannot satisfy positive test
coverage. Required ignored/filtered/failed tests block success. This native probe
is not Docker product e2e or full migration qualification. Local nonadministrator
machines validate parsing/contract only; account provisioning is remote-only.

Microsoft API sources checked 2026-11-07 KST:
- https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.localaccounts/new-localuser
- https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.processstartinfo.username
- https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.processstartinfo.password
- https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.processstartinfo.loaduserprofile
- https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.processstartinfo.createnowindow
- https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createprocesswithlogonw
- https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation

CreateNoWindow is requested but Microsoft documents it is ignored when alternate
credentials are supplied. WindowStyle=Hidden additionally requests hidden startup;
this does not change the required token or introduce an interactive handoff.

Write-denial implementation uses icacls advanced WD/AD/WEA/WA/DE/DC/WDAC/WO
bits for only the newly generated SID. Generic write masks can overlap ordinary
read handles via shared standard rights; preserve READ_CONTROL/SYNCHRONIZE.
The child must prove workspace create-file denial before executing harnesses.
Source: https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/icacls
Account provisioning is never attempted locally. Child process startup output is
retained separately from harness output; environment/credentials are never dumped.

## Observed standard-user completion

Run https://github.com/modootoday/humanymous/actions/runs/42309353245 at exact
source c6a245751e3dd4654157f5482f8ef8757a153707 passed all six required native
tuples. Each Linux/macOS tuple passed 130 tests; each Windows tuple passed 131.
Windows x64 and ARM64 each executed all 37 Cargo JSON manifest harnesses:
36 positive-test harnesses and one successful zero-test harness. All six CLI
security tests passed on both Windows architectures. Ignored/filtered counts
were zero. Actual SID/no Administrators group/TokenElevation=0, workspace write
denial and account/scratch deletion were observed in both Windows artifacts.

The private evidence:root/20261106225537 bundle retains raw per-run/per-tuple
reports and logs. Public evidence:root/20261106223747 retains run/job URLs,
immutable artifact IDs/digests and the exact-set, SHA-checked quantitative
summary. The attached summarizer rejects duplicate/missing tuples, source or
artifact mismatch, ignored/filtered work, incomplete manifest execution and
missing cleanup/token prerequisites. Four earlier failed iterations remain
explicitly failed. The final native run proves only implemented crates at that
SHA; later shared-checkout changes need their own execution. Full runtime,
Docker product e2e and dedicated Windows remote-client/privilege tests remain
unqualified. This probe does not authorize main merge, publication or deployment.

## Rust native runner continuation

The historical six-tuple harness probe remains component evidence. Replace its
new orchestration with an independently locked Rust1.99.1/edition2024 native tool
under scripts/ci/windows-native-runner. Share bounded subprocess/readiness ownership
through test-support; reuse existing actual runtime/authority integration pilots
rather than copy their signing, replay or audit algorithms. Record exact Cargo
executable manifests/digests, actual token/OS, source files and subprocess counts.
Native startup/refusal/CLI dry-run/state snapshots must be actual processes.
Missing local administration, service activation and reviewed documentation examples
remain explicit required failures; no blocked component report qualifies delivery.
Linux/macOS use the same native command under actual non-root UID; Windows requires
non-elevated actual process token. No localhost product e2e claim follows.

Windows temporary-account orchestration is hosted-only and must be Rust, without
new PS/JS/Python or runtime script text. The current safe winsafe closure lacks
NetUserAdd/CreateProcessWithLogonW. A separate narrowly allowed Win32 FFI module in
the CI-only runner is justified for those official APIs with owned UTF16 buffers,
RAII native handles, ephemeral secret zeroization, suspended launch into a kill-on-
close job, minimal environment, generated-name-only cleanup and exact SID checks.
No production verifier acquires unsafe or account provisioning. Local nonadmin
validation never provisions system accounts. Unavailable execution fails closed.

SCM activation uses separate explicit native-security ServiceRole grants: immutable
executable RX versus state-directory modify/inherit to one exact virtual service
SID. Shared descriptor parsing remains single-owner; ordinary installer/audit/
custody profiles must continue rejecting the additional service principal. This
opt-in activation provisioning never runs from inspection/startup or repairs an
unsafe existing object. Peer pipe descriptors share owner/protected/ACE checks,
while authority independently checks actual pipe transport and server PID.

Microsoft sources checked for the FFI boundary:
https://learn.microsoft.com/en-us/windows/win32/api/lmaccess/nf-lmaccess-netuseradd
https://learn.microsoft.com/en-us/windows/win32/api/lmaccess/nf-lmaccess-netlocalgroupaddmembers
https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createprocesswithlogonw
https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects

## Native runner v0.3 API and ownership agreement

Implemented (Windows local tests) native-security storage APIs:
`grant_service_access(&Path,&str,ServiceRole)->io::Result<()>`,
`revoke_service_access(&Path,&str,ServiceRole)->io::Result<()>`,
`verify_service_access(&File,owner_sid:&str,service_sid:&str,ServiceRole)->io::Result<()>`,
`resolve_service_sid(&str)->io::Result<String>`, and
`verify_windows_pipe_privacy<H:AsRawHandle>(&H,&str)->io::Result<()>`.
ServiceRole is Executable(RX0x1200a9,no inheritance) or
StateDirectory(Modify0x1301bf,object/container inheritance). Virtual service SIDs
require S-1-5-80 plus five canonical u32 subauthorities. Resolver accepts only
humanymous-lab-[a-z0-9-]{1,48}, then performs native NT SERVICE account lookup.
Grant/revoke inspect and set the same non-reparse object handle. Exact grants are
idempotent; arbitrary ACL/owner repair is refused. Owner-only remains full access.
An activation state parent is deliberately not a strict custody directory: the
service must create its own protected owner-private child state. SCM production
registration/dispatcher/signed-release transaction remain CLI owner's contract.
Authority owner accepted shared pipe descriptor verification while retaining
independent actual named-pipe type/server PID verification. Ordinary file and
custody checks are not broadened. Current token exposes observed elevated and
enabled_administrator facts without changing installer_allowed semantics.

Implemented shared test-support API `process::capture_with_stderr(&mut Command,
&[u8],Duration,usize)->io::Result<Captured>` with Captured(status,stdout,stderr).
Existing capture suppresses stderr. Each output is bounded independently;
private stdin budget4096bytes, actual exit/total EOF deadline are mandatory.
Debug prints lengths/status only. Lab owner accepted this interface to retire
its duplicate thread/drain/wait implementation. Caller owns descendant job/group;
the helper owns/kills/reaps only its direct child. No supervisor containment claim.

CI owner accepted the independently locked Rust command contract:
`humanymous-native-runner native --report ABS --binaries ABS/target/debug|release`
on actual non-root Unix or non-elevated Windows; Windows-only `standard-user`
prepares then creates the disposable Users-only actor with native Win32 APIs.
The binary directory selects matching Cargo debug/release build/test artifacts;
path/digest mismatch fails. Target packages are runtime,authority,cli,native-security,
not all workspace. Existing complete workspace probe remains separate legacy
orchestration until its own migration; this tool never labels four packages as all.

Report schema1 retains native delivery identity fields and adds qualification,
source.file manifest/github image tuple, actor native facts, manifest product/test
artifact digests/profile, coverage harness counts/per-test summary hashes/exits,
cases, missing_required_cases, unsupported, documentation_examples_executed and
full_runtime_qualified. Actual named runtime/authority integration assertions are
mandatory; unknown/ignored/filtered/missing/duplicate summaries fail. Native direct
CLI processes prove doctor/dryrun/no mutation and invalid-config/no readiness.
Exit1 means error; exit2 means component assertions completed but mandatory full
cases remain missing; exit0/full qualification is deliberately unavailable.
Missing local-only-admin/documentation-examples and unsupported positive raw
TLS/H2/browser/service/remote-machine qualification remain explicit. The reviewed
example digest is identity only; documentation_examples_executed=false. CI owner
must not waive these missing IDs or rename this component success as release.

Observed local snapshot: native-security9, test-support8 and tool3 unit tests pass;
Windows actual product attempt4 executes10/10harnesses,51tests,0ignored and exits2
blocked. Attempts1/2 compile failure from concurrent incomplete CLI and attempt3
Windows fixture namespace rejection are retained separately in private evidence
root/20261107051539. Later profile selection/source changes need new local source
execution. Hosted account/job and Linux/macOS execution are not observed yet.
No push/tag/publish/production or local system-account provisioning is authorized
for this continuation. Root/CI owner owns exact-source remote orchestration.

## Shared security and process checkpoint delta

Implemented/locally verified after26d9925: service activation grant/revoke accepts
only explicit absolute local fixed-drive paths; native protected-DACL extraction
is shared rather than copied. Actual state-directory Modify/inheritance and
executable RX transition tests both pass and strict custody rejection remains.
Latest native-security count10. Lab owner requested and accepted
`ManagedChild::spawn_duplex(&mut Command)->io::Result<(ManagedChild,ChildStdin,
ChildStdout)>` and `try_wait(&mut self)->io::Result<Option<ExitStatus>>` for a typed
persistent witness protocol. Caller owns frame bounds/deadlines/pipes; shared
guard still owns direct-child kill/reap. Same hidden Windows pipe factory is used
by capture and interactive spawn. Latest test-support count9 includes actual
native duplex echo/EOF/lifetime, concurrent stderr and overflow/timeout refusals.
No common Cargo dependency delta; raw-output Debug remains length-only.

CLI owner recorded83397fd/3d1d003 consumes the shared service API and source-bound
working-file hashes separately. This checkpoint commits those exact shared files
before root remote execution. Actor/SCM hosted qualification is not a dependency
of API compile/unit readiness and remains separately incomplete. Authority/lab
consumers must qualify their actual pipe/witness paths at their own source.

## Disposable SCM delegation implementation

Root accepted the independently locked tool's Win32_System_Services feature on
existing windows-sys0.61.2; root Cargo/lock remain untouched. Implemented but NOT
hosted-qualified: `standard-user-service --report ABS --binaries ABS/target/release`
requires an elevated disposable GitHub-hosted parent, creates a Users-only account,
retains the SCM DACL through the actual manager handle, adds only CONNECT|CREATE
rights(0x3) for that generated SID, launches ordinary-token `service` in the owned
job, restores/readbacks the exact original DACL, then deletes account/scratch.
Direct elevated service execution is rejected: installer checks stay nonadmin.
The OS grant applies to the SCM manager, not a name prefix; actual CLI signed
policy restricts generated service names. Unknown/inherited manager DACL and
concurrent descriptor changes fail without overwriting unfamiliar permissions.
No UAC/security disablement. A5KiB snapshot is not assumed; the native descriptor
query enforces Microsoft's8KiB maximum. SCM service processes are manager-owned,
not falsely claimed as descendants of the runner's job.

`--actor-case after-grant` deliberately fails after actual SCM mutation but before
child launch. Expected exit1/statusfailed; hosted negative validation must observe
account_deleted=true,scratch_deleted=true,scm_delegation.restored=true. This case
is not a passed whole native gate. Missing/false restoration is failure. Normal
launch records actual job-empty accounting after explicit termination as well as
kill-on-close configuration; timeout/unknown process status never qualifies.
On execution/restore/delete errors, lifecycle error/report facts remain failed;
raw per-harness list/run stdout/stderr are copied by explicit names only. The
runner never dumps credentials or environment.

`service` directly runs the exact opt-in CLI native service lifecycle test only
on a disposable ordinary/delegated Windows or non-root Unix manager actor. CLI
service-pilot feature is explicit. The default native component preserves its
single required ignored service test as rawignored/skips1/blocked, not an accepted
helper ignore. Unknown ignores and filtered work remain errors. Case sets and
missing full native IDs are unchanged. A diagnostic fixture is not a runtime
ServiceHost integration.

References: https://learn.microsoft.com/en-us/windows/win32/services/service-security-and-access-rights
https://learn.microsoft.com/en-us/windows/win32/api/winsvc/nf-winsvc-queryserviceobjectsecurity
https://learn.microsoft.com/en-us/windows/win32/api/winsvc/nf-winsvc-setserviceobjectsecurity

Later actual local release snapshot atc1641c7+dirtymanifest executes13/13harnesses,
62passed,1requiredignored/errors0,exit2blocked. Earlier CI owner summaryinvalid
failure is preserved at its different source; no retrospective parser-cause claim
is made. New failure diagnostics identify index/target/exit/discovered and preserve
private list/run stdout/stderr. Public summary is generated by Rust `summarize`,
whitelists typed counts/commitments, strips SID/credentials/raw traces and emits
LF bytes. Own source records validated worktree and canonical Git SHA256 separately.
Local account/service execution modes actually refused before provisioning; no
local system-account, SCM grant or service registration was attempted.

## Release diagnostic source delta and partial failure contract

A later source e30e0e9+dirty native release run failed atindex9 targetlocal_service,
exit101,2discovered/1passed/1failed. Preserve it separately from earlierc1641c7
62-pass/blocked snapshot. Authority owner diagnosed actual fixture reactor cleanup
and supplied the repair without weakeningcap2 or403 expectations; a new locked
native run is required. Root owns sharedlock churn. The runner records compiled
manifest before execution and partial coverage before propagating any harness
error. Partial coverage includes complete=false,required/executed counts,actual
failed/passed/ignored/filtered and validated flag. An unrun remainder is not zero-
test success. Public summaries retain partial/failed distinctions while stripping
credentials,SIDs,paths and raw traces. Rawstdout/stderr stays private.

New hosted SCM source and features remain separately unqualified. Localtool5unit
checks only descriptor composition/guard/parser/redaction, not account/SCM API
mutation. A controller must execute standard-user-service and after-grant failure
cleanup at the exact committed source before any native service result is claimed.

WASM compatibility correction: test-support::process is native-only through
cfg(not(target_arch="wasm32")); pure JSON contract assertions remain WASM-usable.
Root observed drop_non_drop from unsupported wasm ChildStdin. Restricting the OS
module fixes the boundary instead of relaxing any lint. Scoped wasm32 all-target
Clippy and native9tests passed; root owns full integrity/collector rerun.

## Workspace actor migration contract v0.4

CI owner requested/accepted one shared Rust actor/prepare/accounting implementation
for default-feature whole-workspace tests, separate from native product pilots.
Implemented CLI: `workspace --report ABS --binaries ABS_ROOT/target/debug|release`
on actual ordinary Windows/nonroot Unix, and Windows hosted elevated parent
`standard-user-workspace` with identical arguments. It executes locked Cargo
--workspace --all-targets discovery with no extra CLI service-pilot feature, hashes
and executes every discovered test executable, distinguishes zero-test harnesses
and counts exact passed/failed/ignored/filtered. Parent uses the same Users-only
account/minimal environment/suspended job/cleanup code as native and service modes.
There is no second actor, compiler manifest parser or summary parser.

Workspace report schema is `humanymous.workspace-probe/1`,kind=workspace,
scope=workspace-default-features,qualification=implemented-workspace-component,
full_runtime_qualified=false. Complete required unit/component execution may
return0/statuspassed; error/unknown/ordinary ignored/filtered work returns1/failed.
This separate schema can never satisfy schema1 native product delivery. Native
four-package service-feature profile remains exit2/blocked with required service
not_run. Native/server/browser/Docker qualification is not implied by workspace0.
The source snapshot obtains actual workspace member directories from locked Cargo
metadata, covers their Rust/manifests/lock inputs plus this isolated runner and
root toolchain/manifest/lock, and checks expected runner
architecture against the actual compiled native executable architecture.

Only audit package#humanymous-audit@,targetkey_custody,kind[test] may classify the
existing two named ignored subprocess entrypoints. Exact rawignored names are
child_lock_probe/child_interrupted_writer. Parent cases competing_process_cannot_
load_or_create and process_exit_releases_lock_and_rotation_pending_requires_recovery
must each actually pass; child_lock_probe actual nested pass must appear. Exactly
two summaries are required: nested1passed/0failed/0ignored/0measured/discovered-1
filtered; finaldiscovered-2passed/0failed/2ignored/0measured/0filtered. Preserve raw
ignored_subprocess_helpers=2 and required parent names. Missing/duplicate/extra
ignored/summary/parent proof fails, never a broad ignore allowance. Extra required
service tests do not inherit this classification. Unit negative cases cover these
boundaries. Actual workspace/hosted6tuple execution remains required before retiring
the old Python/PS orchestration; local execution is currently being measured.

Source-bound local observations: f3c8ea2072783141f38e96f95864acc2bb1a963f
plus recorded dirty inputs completed release native14/14 harnesses,66passed,
0errors,1required service ignored/skips1,exit2/blocked. It does not qualify SCM.
Workspace initial compile failed for missing CMake PATH. Existing Rust cef-pilot
toolchain discovery supplied a process-local PATH without disabling sandboxing;
retry discovered73 harnesses and executed10, then failed exact unknown ignored
child_witness_probe in audit durable_witness. Its nested summary is preserved.
Audit owner is preparing captured subprocess output/assertions for durable_witness
and key_backups. The accepted next contract is limited to audit kind[test]:
durable_witness helper child_witness_probe with parent
witness_process_lifetime_and_interrupted_recovery; key_backups helper
child_retire_without_pin_acknowledgement with parent
crash_reconcile_preserves_consumed_authorization. Each requires exactly one raw
ignored entry with that exact name, its exact parent actually passed, exit0,
one summary with passed+1=discovered and failed/measured/filtered0. Parent captures
child stdout/stderr with shared capture_with_stderr(deadline60s,budget64KiB),
checks actual exit0/stderr empty/static flushed marker and signed-state/recovery
effects. Child summaries cannot leak into parent output. Old nested-summary sources
therefore still fail. Raw ignored helpers total4 across these three audit targets
and never add to passed tests; no arbitrary ignored case inherits the exception.
Runner negative tests cover absent/wrong parent/helper, extra summary, nonzero exit
and missing test. Canonical audit implementation and actual rerun remain pending.
Raw reports
and all list/run/build logs remain private; sanitized failure/component summaries
are registered in evidence:root/20261106223747. Passed workspace summaries require
complete coverage, every row validated, zero failed/errors/skips, required count
equal executed rows and raw ignored equal explicitly classified helper counts.
Public rows retain helper/required-parent names; account identities are excluded.
On rejected multiple-summary output, subsequent runner versions preserve the
observed final summary counts with validated=false rather than dropping counts.
Summaries from older raw reports with unreadable counters explicitly expose
counts_complete=false and unknown_count_harnesses; aggregate observed zeros do
not imply that unmeasured counts were zero. Actual workspace-member retry at
16c9e89 plus recorded dirty inputs still rejected the old durable_witness nested
summary after73 discovered/10 executed harnesses. Seven runner tests and Clippy
passed after this accounting correction. Native91f93c5 plus dirty inputs completed
14harness67passed/errors0/requiredignored1/blocked2, preserved separately.

Root memory-investigation instruction on 2026-11-07 deferred all new heavy
validation. Already-running workspace-captured-helper compile was identified by
owned process chain and reported to root; it was not killed or restarted.
No new heavy runs may start until root reallocates capacity. This scheduling
constraint does not alter qualification status or the remaining hosted gates.
The existing workspace-captured-helper run subsequently exited1 on its own;
owned processes all ended. It discovered73/executed14 harnesses. Audit
durable_witness now passed12 plus rawignoredhelper1/validatedtrue. The next
node-adapter host_process target passed2 but ignored3 explicit actual Node/Bun
production pilots, so required workspace coverage failed. These are required
product cases, not subprocess entrypoints, and receive no helper exception.
key_backups was not reached in this execution. Raw partial-run artifacts are
registered privately. Root and CI received the exact failure and no-running-
process report. Full local Specfold check also failed on other private analysis
paths/unregistered inputs(17938errors/1warning); --no-local remained0errors/1warning.

## Active Windows goal: strict publication and lifecycle evidence

Root relayed explicit user authorization for an active Windows completion goal.
get_goal returned none; create_goal set active without a token budget. Completion
requires owned implementation/commits, pure unit checks, actual hosted pilots,
strict accounting and Specfold evidence. Thread role: Windows native runner.
Policy canon remains program-roadmap/delivery; local actual process,
SCM/account mutation, duplicate dispatch and publication remain unauthorized.

Pure evidence/accounting validates passed workspace publication: typed unsigned
counts, positive required denominator equal executed/row count, complete coverage,
every exit0/validatedtrue/failed0/filtered0, checked passed+ignored=discovered,
per-row ignored=classified helpers and nonempty aggregate passed. Missing/null/
string/negative/overflow counts fail closed; zero-test harnesses remain in the
denominator. Raw reports are not rewritten. Producer flags alone are insufficient.

The evidence/lifecycle whitelist retains typed cleanup booleans, child exit,
job-empty/delegation request/known actor case, SCM rights/single-generated-SID/
restored/original-descriptor digest and service counts. Actual SID/credentials/
raw SDDL/free-text errors/paths are excluded. Missing remains null, false remains
false. Projection does not alter failed/blocked status or prove actual execution.

CI owner received separate hosted standard-user-service positive and after-grant
negative contracts: positive actual service1passed still componentblocked2;
negative intentionally failed1 before child launch. Both require actual account/
scratch deletion and original SCM restoration. Positive additionally requires
actual job-empty. Corrected earlier description: after-grant precedes launch,
so child_launch_attempted=false and child exit/job-empty/job-kill observation
remain null. Raw and sanitized reports retain this attempt fact. Actor-backed
workspace passed publication requires attempted child/exit0/normal case and all
actual cleanup facts true. CLI owner confirms the
existing service pilot is a signed fixture; actual RuntimeServiceHost pilot name
awaits CLI/runtime source work. Remote42348518353/63f69ae is a distinct source.
Root allocated the single local Cargo slot: six new pure evidence tests passed,
zero failed/ignored and seven outside-scope filtered, jobs1/incremental0. Direct
rustfmt and scoped all-target Clippy with warnings denied passed. Slot returned.
Exact source-byte hashes at HEAD2b2d3ccf+dirty are recorded in the pure-unit summary;
raw console output exists in tool results, not a separately retrieved raw log.
Actual hosted lifecycle/projection verification and private raw retrieval: not_run.

Root assigned native-windows lane and sole native-security OS credential provider
ownership. CLI selects typed references, runtime consumes protected bytes and audit
custody retains key sealing responsibilities. Missing/locked/unsupported must fail
closed, no UI or plaintext argv/env/file fallback. This ownership acceptance is not
an implemented or verified OS secret provider; other-OS gaps stay explicit.

## Protected service input draft (dependency activation pending)

CLI and this owner agreed API native_security::secret::{ServiceSecretReference::
new(service_name,credential_id),read_service_secret(&reference)} returning
io::Result<Zeroizing<Vec<u8>>>. References are nonsecret owned-service selectors,
not paths/argv/environment values. Existing humanymous-lab service namespace is
retained; id is1..64 lowercase ASCII/digit/hyphen. Debug redacts selector values.
Reader returns only nonempty <=4096 bytes; CLI additionally verifies UTF8 before
injecting existing zeroized runtime audit passphrase into an otherwise secret-free
signed config. No creation, fallback, UI, logging or custody reinitialization.

Draft Windows implementation requires resolve_service_sid(service)==actual token
user SID before accessing an explicit native store. Exact target is
humanymous/service/{service}/{id}; existing target metadata and Local persistence
are checked before and after secret retrieval. These separate reads are not an
atomic anti-ABA snapshot. Returned bytes are immediately zeroized-owned; existing
sealed custody authenticates its keys. Backend errors are replaced by a static
unavailable error. Missing/locked/store/principal errors fail closed. Virtual
service credential-store availability is not yet demonstrated; no installer
credential store may substitute. Other OS readers return Unsupported.

Proposed safe dependency graph is keyring-core1.0.1 and Windows-only
windows-native-keyring-store1.1.0 without default features plus existing workspace
zeroize. Root owns graph/lock activation. Draft secret files remain unexported and
uncompiled until that coordination, with no manifest or existing lib change.
Read-only provider has no provisioning endpoint; hosted fixture provision/cleanup
in the actual service principal remains required, not_run. Caller may not implement
a duplicate OS reader or pass plaintext through argv/env/file to evade this gate.

Primary API basis: [explicit native Windows store](https://docs.rs/windows-native-keyring-store/latest/windows_native_keyring_store/),
[store build](https://docs.rs/windows-native-keyring-store/latest/windows_native_keyring_store/store/struct.Store.html),
and [keyring Entry](https://docs.rs/keyring-core/latest/keyring_core/struct.Entry.html).
Pinned dependency activation and actual hosted tests must substantiate the draft.

Root approved the pinned graph and sole-provider scope. Native-security manifest
now declares workspace zeroize and Windows-only keyring-core1.0.1 plus
windows-native-keyring-store1.1.0 with default features disabled; lib exports
secret. The owned service-name check is shared by selector and SID resolution.
No authored unsafe or shared root Cargo/lock change. Root must resolve common
and isolated-runner locks; its current Cargo slot belongs elsewhere. Thus new
provider compile/pure tests and actual service principal read remain not_run.

Hosted observation is separate: run42348518353/attempt1/source63f69ae job
128331815997 windows-2025 built workspace successfully then failed workspace
actor step. The job log has only the wrapper's generic owner execution/summary
failure. Specific harness and denominator are unknown; positive/negative service
steps were not reached. CI owner holds raw native-windows-job.log in private
evidence20261107064713 (SHA676b6a7d7695163f6e299b354b34a53596aa5ae58951251e3c744a3d04859443).
No runner summary was recovered. Do not substitute historical local Node ignored
failures as this run's cause. CI followupa0ceac3 emits sanitized formatter output;
its execution is not qualified by the old run. Public job observation is attached.

## Follow-up source closure and verification queue

Root approved credential-pilot=[] and existing serde.workspace for test-support.
The generic duplicate-decoded-key visitor moved with attacker-owner consent from
671c439 into test-support::json::decode_strict_json(bytes,max_bytes), with typed
TooLarge/Invalid, byte bound before parse, serde nesting limit, finite numbers,
trailing-input refusal and no payload in errors. Attacker removed its private
visitor in052c1da; lab consumes the same API. Schemas, replay and semantic checks
remain consumer-owned. New shared pure2tests and graph verification are not_run.

Service private children now have two explicit entry points: installer-owned
delegated StateDirectory parent with signed installer SID, and already service-
owned protected/inheritable parent. Both use one create-new/same-handle hardening
body, actual service TokenUserSID, no existing-child repair or unknown cleanup.
Parent-owner/path-replacement and trusted-ancestor premises remain explicit.
Service fixtures clean their own newly created children after custody/WAL drop;
production persistent state is not deleted on normal stop. New pure1test and
actual OS directory initialization remain unverified.

Credential-pilot fixture API accepts DisposableServiceContext::new(service,run,
source_sha40or64lowerhex) and ServiceSecretFixture::create(ref,zeroizedbytes,context),
finish(self)->Result. Context is caller-validated nonsecret metadata, not signed
or hosted attestation. CLI authenticates its signed plan; external run/source/
actor evidence is required. No SCM CI-environment inheritance is assumed. Actual
service SID and unique pilot-32lowerhex target are mandatory. Existing/unknown/
changed metadata or secret refuses mutation. WinCred create/delete is not atomic
or CAS against same-principal writers; exclusive disposable context is a premise.
Explicit finish observes cleanup and Drop is best effort only. Default production
reader remains readonly; other OS fixture creation is Unsupported. Root's prior
pure3 report does not qualify this feature/factory extension. New pure1featuretest
and actual service provisioning/read/cleanup remain not_run.

Runner early initialization errors now preserve failed reports with unknown/null
source/required/executed denominators, without overwriting existing reports.
Only typed facts/optional OS error number are public. Pure1fallback test pending.

CI-requested build observations count compiler-artifact events separately from
harnesses, including dependency events and fresh/rebuilt/unknown; each existing
Cargo child has its own elapsed milliseconds. Source/locks/declared toolchain/
OS-arch-ABI/profile/features/hashed flags and overrides form an observed input
fingerprint. Actual compiler version and all configuration inputs are not proven:
input_key_complete=false/cache_reuse_proven=false. No prebuild is removed. Existing
commands request private --timings HTML; raw JSON/sidecar stay private and HTML
retrieval is not claimed. Public output permits counters/known labels/digests only.
New pure2metrics tests and actual hosted fresh/timing observation remain not_run.

Subsequent root-assigned A validation at canonical shared sourcef44b32b passed:
native-security credential-pilot secret4 (11filtered), private-child component1
(14filtered), test-support strictJSON2 (9filtered), all0failed/0ignored. Filtered
counts are per invocation, not additive unique-test coverage. Feature-inclusive
all-target native-security/test-support Clippy denied warnings and passed. No
actual OS secret/ACL/filesystem/process test ran. Before starting, resource guard
observed34.15GiB physical free/83.1% commit; jobs1/incremental0. Slot returned.
Post-execution17 owned Rust/manifest files exactly matched f44b32b Git blobs;
SHA256/individual blob comparisons and counts are attached in
shared-native-f44b32b-pure-summary.json. This comparison is scoped post-execution,
not an atomic snapshot of all concurrent project edits. Four raw test/Clippy logs
are registered in private evidence20261107051539. Actual hosted fixture/CrossOS
verification and standalone runner fallback/metrics validation remain not_run.

New authoritative root update: sourcea26d1c3f08b365ca93545c6a51343095868edb45 was
pushed to codex/rust; run42354873001 terminated at workflow startup with0jobs and
0tests. Nested policy actions:read exceeded caller actions:none. CI owner fixes
the caller permission contract; root alone pushes/dispatches the next source.
This is not old63f69ae's workspace failure, nor execution evidence for local dirty
runner hooks. Native/shared sourcef44b32b is included but actual lifecycle remains
not_run. The separate startup observation is attached; no execution report was
generated when no job started. Future Go/.NET/Apache profiles stay unqualified.

CLI actual runtime service contract is now canonical7f73604251e983addbb1ef97283959cd6045c6e1:
integration target runtime_service_process, exact ignored test
native_runtime_service_lifecycle_and_missing_credential_fail_closed. Runner source
adds this as a distinct required case alongside generic service_process, enabling
both approved service-pilot/runtime-service-pilot features. Default native does not
run these privileged cases; ignored required cases are not helpers. The runtime
case is Windows-implemented; other-target source explicitly reports unsupported
required work. The generic pilot's qualification is not promoted to actual runtime.

Selected execution verifies exact package/target/kind, executable digest, enumerated
test inventory, one selected pass, zero failed/ignored/measured and filtered count
equal discovered-1. Runtime final JSON is one strict64KiB line bound to exact source
and run ID with nine required independently observed facts: missing credential
fail-closed, actual principal, readiness, graceful stop, stopped host exit, credential
and state cleanup, receipt binding and SCM registration removal. Missing/duplicate/
wrong-source/run/false facts fail. Raw service PID remains private. The parent CLI
test independently correlates native SCM PID/status, nonce/executable and service
receipt; metadata alone is not process provenance. Nonsecret controller run/source
environment is input only to the ordinary test, not assumed inherited by SCM.
Two case-specific execution fields and logs are preserved and whitelisted; overall
component remains blocked2 for other missing native requirements. New catalog1 and
receipt2 pure tests plus runner evidence7/metrics2 await the assigned slot.

Browser owner requested Captured.pid. Shared process helper now captures actual
direct-child ID before wait and omits it from Debug. A new hosted-only process test
binds the child self-reported PID and distinguishes the parent. This is not caller
authorization, descendant containment or protection from PID reuse after exit.
No actual process test ran locally. This change is afterf44b32b and is not covered
by its shared pure validation. Existing no-op subprocess entry point accounting
was raised to root for explicit helper-contract review; no product-ignore exception
is introduced while that review is pending.

Fixed source8132117e7bf1445939d7dee9066fcf45ccf0915b run42355168729 is in progress;
observed policy success and Windows job128351926153 queued. This supersedesa26d
startup failure only for subsequent execution, not historical results. Root alone
pushes; no new candidate is pushed during the run. Local dirty runner hooks/PID
changes are outside this source and remain unqualified by it.


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


Retrieved hosted8132117 MSRV job128359709010 completed successfully. Its decoded
job log proves Linux authority native_process7pass/0failed/2ignored/0filtered and
runtime native_process1pass/0failed/0ignored/0filtered, including the four named
normal authority assertions and the runtime native binary session assertion. The
ignored Linux privileged parent remains required_not_run; its ignored probe is
not a verified helper exemption. This separate execution supersedes any inference
that all rows after the failed workspace actor remained globally not_run. It does
not qualify Windows, SCM, credentials or later-source PID/directory/recovery code.
The private202088-byte UTF-8 decoded transcript SHA256 is
fdd0b9b0bd84be19c16d9608e28bd6c3b5d4356d95e7ab11a14f40e15906a6e2;
HTTP raw byte identity and uncollected owner reports are not claimed.


Direct Windows813 job128351926153 decoded-log recovery now proves standard-user
actor cleanup independently of the failed product coverage: account_deleted,
scratch_deleted, job_empty_observed and job_kill_on_close are true; child launch
was attempted and exited1. SCM delegation was false. The actor failed at the Node
host_process row (5discovered/2passed/3ignored/0failed/exit0/unvalidated), after
15of89harnesses and55passes. This is actual actor cleanup evidence, not SCM or
complete workspace success. Original private owner report remains not retrieved;
its reported SHA256 is285a2bc17f2ea90f07d91ed8dddc09f1d748a2f65d23fc43e08387cd676304d6.
The146775-byte decoded UTF-8 job transcript SHA256 is
23a92e18a39d7e5d19fe03b27600ebb38e199ac474de277947778ae7fa5776b5.
A complete sanitized owner summary is semantically extracted and separately
attached; its canonical JSON is not asserted to be original owner-report bytes.
This supersedes the earlier missing Windows log/hash observation only.


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


Root resumed the goal and assigned C (target/validation-agents, jobs1/incremental0)
for pure validation only. Runner28pass/0failed/0ignored/1filtered and shared Docker
comparison5pass/0failed/0ignored/12filtered passed. The excluded runner test performs
actual filesystem/ACL work. Initial public API missing-documentation warnings were
fixed; a Clippy manual_unwrap_or failure was corrected using closed-code filtering
followed by unwrap_or, preserving private-string exclusion. Runner28 was rerun.
Final runner and shared support all-target Clippy with warnings denied passed;
the earlier explicit runner check passed before that syntax correction. Forty-three
source/manifest/lock/toolchain files had zero final-before/after drift. C returned,
no active process handles remain. No actual Docker, account, subprocess fixture,
ACL, SCM or interruption execution occurred in this slot. This supersedes the
pending pure/check status only, not missing actual qualification. CLI complete
readiness source2679089e remains a separate unexecuted actual service candidate.


Actual ordinary-user local Windows audit evidence is now independently adopted
from evidence:root/20261107095831 (audit sourcec09d6897,111file closure0drift).
Three targets pass44 tests: control_operations21/0ignored, durable_witness13/1ignored,
key_custody10/2ignored. All failed/measured/filtered counts are zero. All three raw
log hashes were independently checked. The actual custody log contains both named
parent successes, exactly one outer final summary, and no leaked child markers;
this confirms the paired current capture framing on Windows. Three raw ignored
helpers remain excluded from passed counts. This is local ordinary-user native
qualification for these audit targets only, not hosted/full workspace/SCM/provider/
actor provisioning or actual sidecar interruption qualification. Raw logs remain
with the owning audit evidence bundle; the adoption record binds their hashes.


Pending mutation admission correction: disposable_host now observes ACT with
var_os and rejects any presence, including empty, false, zero and non-Unicode
values. GITHUB_ACTIONS and RUNNER_ENVIRONMENT still require exact accepted
labels. Both actor account provisioning and service mode call this guard before
mutation. This is fail-closed local-emulator admission, not cryptographic hosted
attestation; external actual-run evidence remains mandatory. One pure regression
covers malformed labels and non-Unicode ACT without mutating environment. Source
is formatted but NOT_RUN; next runner pure selection29, no SCM/account execution.

Paired audit WAL checkpoint d7045fee makes process_lock_probe explicitly ignored,
panics when unprovisioned, and is exercised by bounded actual parent capture.
Runner recognizes only the exact audit library target/helper/parent and single
outer summary, rejecting leaked humanymous-wal-lock-refused markers. Future full
audit raw ignored count5 is separate from old/local narrower results. In hosted
813 MSRV, audit library raw54 includes one no-op helper (53 substantive); full
audit raw101 outer passes include that same no-op (100 accepted). Parity cases5
are included, nested custody child summary is not added. Original raw logs and
original reported totals remain immutable. Existing local three-target44/raw3
adoption is unchanged. This historical qualification correction is not a fresh
execution pass of the changed WAL code.


Root-assigned C validation of ee7ff0f1 runner source completed:29pure passes,
zero failed/ignored and one excluded actual filesystem/ACL fixture. All-target
check and Clippy with warnings denied passed. Forty-three source/manifest/lock/
toolchain files had zero before/after drift. Exact selectors and raw hashes are
attached; the working Git HEAD was not captured, so this is a component source
checkpoint plus exact closure claim, not an atomic full-workspace HEAD claim.
Admission observed33.15GiBfree/80.68%commit and verified exclusive target/
validation-agents-native; jobs1/incremental0. C returned. No actual actor, account,
SCM or process-native profile was run. ACT denial and WAL helper tests are now
pure-qualified; actual changed WAL parent and service lifecycle remain distinct
native/hosted gates. No .NET workload source changed during this validation.


## Draft .NET ordinary-user actor workload v0.1

Status: draft contract, not implemented or qualified. Root requested a narrow
Windows workload hook reusing the existing account/token/job boundary; no SCM,
new account implementation, administrator fallback or manifest change is included.
The exact controller contract is crates/dotnet-adapter/INTEGRATION.md, Hosted
ordinary actor handoff contract. .NET owns controller and dotnet-validation.yml;
this owner owns actor dispatch, private environment, containment and safe summary.

- Proposed parent mode standard-user-dotnet and fixed child mode dotnet-package
  use the existing actor identity verification and suspended kill-on-close job.
  The child runs only the digest-pinned package test executable with --ignored
  --nocapture --exact sdk_package_and_real_host_matrix --test-threads=1. No generic
  executable/argument runner, recursive Cargo or native default harness build.
- Typed bounded strict input schema humanymous.dotnet-actor-input/1 binds the
  clean source root/SHA, hashed controller/host harness/native library/provenance,
  SDK root, Git executable and explicit read-only compiler/linker paths. Unknown,
  duplicate, relative, missing, nonregular or changed inputs fail before launch.
  Root/Cargo JSON build provenance and genuine hosted run/source evidence remain
  separate; a local hash or environment label cannot create that provenance.
- Parent validates source and artifacts before and after execution. The generated
  Users actor has read/execute source/artifacts and verified source write denial.
  Genuine hosted admission rejects ACT before any account mutation. The child
  validates its actual SID and non-elevated token; no local-profile promotion.
- Environment is constructed, never inherited wholesale: SystemRoot/WINDIR/
  SystemDrive/COMSPEC, private TEMP/TMP/USERPROFILE/CLI home/NuGet caches,
  DOTNET_ROOT, explicit SDK settings, and verified SDK/Git/MSVC/Windows SDK search
  paths. Only named bounded path-list/toolchain values are accepted; credentials,
  runner tokens and arbitrary command strings are excluded. SDK installs and
  Rust release builds are prerequisites, not ordinary-child responsibilities.
- Git ownership compatibility is process-local for the exact verified checkout;
  no global safe.directory, user/system config writes or ambient credentials.
  Optional index writes are disabled. .NET HumanymousBuildRoot redirects all
  intermediate/output/cache files outside source into the private fixture tree.
- Proposed profile deadline is30minutes inside a60minute workflow. Existing SDK
  restore600s/publish1200s and host180s bounds are ceilings, never extensions of
  remaining outer time. Parent timeout/nonzero/missing output remains failure;
  killed work cannot manufacture complete counts or successful cleanup.
- Controller outcome schema humanymous.dotnet-component-summary/1 is safe typed
  metadata only. Proposed OBSERVATION_PATH publication is owner work, not assumed
  to exist. A complete package unit footer is not full contract qualification;
  required case IDs/modes, executed/failed/ignored/unknown/not_run and native
  artifact identities stay explicit. Incomplete exit2 never becomes workflow0.

### Required private handoff and cleanup decisions

Current actor TEMP inherits restricted administrator+actor grants, whereas the
shared ordinary private-directory factory requires a protected owner-only parent.
The child must create/privatize its own parent before controller use. The elevated
parent must not take ownership or repair ACLs to inspect private nested output.
Selected bounded regular-file outcomes/raw logs must be published by the child
into the restricted handoff root with relative allowlisted names/length/hash
manifest; never copy whole package/cache/build trees into public evidence.

Normal child exit must publish known results and clean its private work first.
A timed-out child cannot self-clean. Proposed fallback is a fixed bounded cleanup
mode launched under the SAME generated user, after main-job containment/empty
observation and before account deletion, reusing the original account/token/job
code and only the exact generated scratch identity. No arbitrary target, command,
ACL repair, administrator deletion substitute or SCM authority is granted. Root
and native/.NET owners must review this ordering before implementation; current
account-delete-before-admin-scratch-remove is insufficient for private leftovers.
Cleanup failure/unknown remains explicit and prevents a passing actor summary.

LOGON_WITH_PROFILE is currently used. Account deletion and scratch deletion do
not prove Windows profile directory or registry profile removal. USERPROFILE
redirection is not proof of OS profile cleanup. Profile lifecycle is an explicit
unresolved cleanup gate/residual; historical account/scratch/job facts are not
rewritten as stronger profile-erasure evidence.

### Verification and critique gates

Pure: exact mode/controller/argument inventory; strict schema and fixed environment
allowlist; path-list bounds; source/hash mismatch; 30minute deadline selection;
unknown and incomplete result accounting; regular handoff paths; no secret/raw
identifier publication. Actual hosted Windows: generated Users identity, source
write denial, SDK/Rust artifact pinning, all source/packed JIT/AOT declared modes,
independent native host evidence, main and cleanup job emptiness, private tree/
account/scratch/profile outcomes and bounded timeout. No local account/SCM test
is authorized by this draft. Docker product qualification remains separate.

Blue: reuse one actor boundary and shared private-directory verification. Red:
reject arbitrary commands, ambient environments, writable SDK search paths,
ACT-spoofed hosted claims and stale outcomes. SRE: complete containment before
same-user recovery and account deletion; keep bounded partial failures. Privacy:
selected private handoff only, no admin ACL repair or token copying. Evaluator:
package controller success is not every host contract case or full product pass.
Verdict needs-work until outcome publication, cleanup/profile contract and typed
input/environment details are agreed. No implementation starts from a placeholder.

## Protected CI actor context carrier source checkpoint

The narrow native-security metadata publisher/inspector is implemented, not a
trusted launch/product authority. The owned prebuilt plan specifies bounded
create-new context/binding files, actual Admin publisher/reserved ordinary actor,
exact administrative-owner/actor-read DACL, retained handles and explicit failure
cleanup. No local current-user fallback or implicit account/SCM/SDK operation.
Context and startup credentials remain separate.

Exact producer-handle/provider association remains Unsupported. Fixed-drive
checks do not prove trusted ancestors or a nonreparse tree; locator/hash/JSON/GHA
labels cannot mint Ready or a trusted execution profile. Source6312cc4f/export
44abcf67 passes native check/strict Clippy, nonce/SDDL pure1 and runner check only.
Actual publisher/reader/Job/profile/HostIO remain unexecuted. Initial E0308 is
preserved; actor-context-carrier-summary.json binds raw/source/closure evidence.

## Shared binding and retained capture source checkpoint

The shared parent_context codec owns the closed nine-field
humanymous.parent-context-binding/1 schema: schema, run_nonce, actor_sid,
source_sha, context_sha256, verified_manifest_sha256, provenance_sha256,
original_budget and preflight_environment, in that declaration order. Exact SID
and four original budget fields must equal the decoded parent context; the
github-hosted enum value is metadata only. Invocation source_sha is distinct
from managed/native/harness compiled lineages. Binding uses compact typed JSON
plus LF; provenance commitment uses typed compact declaration-order JSON without
LF, never arbitrary map ordering. All decoded claims remain unverified; codec
success cannot bypass native producer identity or enable Command/Ready.

Root requested a coherent source-only checkpoint before allocating validation.
Binding tests3 and new CaptureAttempt pure tests4 are authored NOT_RUN. Existing
parent-context evidence does not qualify these changed bytes. CaptureAttempt
retains the actual direct Child and every started IO JoinHandle, returns the
same owner on partial/unsettled failures, and preserves sticky errors and original
absolute run/cleanup thresholds. Public poll()/settle_joined() sample real time
internally after observations/joins; no caller observation-time argument exists.

Poll performs no blocking wait/join/write/kill. Start/kill/final joins can block
in the OS and require independent caller/host supervision. Only actual joined
workers permit the private settled result; direct child status, pipe EOF and
descendant settlement remain distinct. No timeout detaches workers or renews a
budget; abandonment is Unknown and Drop is not cleanup proof. Existing legacy
ManagedChild/capture behavior remains present and is not newly qualified by this
additive API. Full retained contract and outstanding actual negative fixtures are
specified in [[plan:root/20261107140500]].

The separate isolated bridge source/graph checkpoint is Root commit
9d0c0db985c4b345931abb4f03d548b9157577ef, with the explicit unsafe exception in
rust-migration. [[plan:root/20261107140021]] records accepted parent assertion and
W/T/R design plus the exact bridge contract. Its three pure tests, native calls,
caller custody wiring and actual Windows tests are still NOT_RUN. Producer
identity stays Unsupported until the complete native caller and evidence qualify;
no account/Job/SCM/apphost execution follows from either source checkpoint.

The later exact b06fddae+querye792 source-only receipt now observes parent-context7,
retained-capture4 and bridge3 pure passes, both all-target check/strict Clippy/fmt,
with captured closure zero drift. See binding-capture-bridge-pure-summary.json
in the cross-OS evidence bundle. Initial E0432 and corrected retry-before-renewal
ordering are explicit and Root accepted the observed limited receipt. This
supersedes NOT_RUN above ONLY for these exact pure/check bytes; actual child IO,
worker joins, native queries/creation, producer custody and hosted tests remain
NOT_RUN/Unsupported. A subsequently identified create.rs prevalidation allocation
gap is not covered by these passing tests and needs a separate source checkpoint.

The next immutable173008f0 gate observes common-owner7a1 capture4/direct-prefix1,
bounded-create3a0 bridge4 pure and external Rustdoc2 passes, plus both package
all-target check/strict Clippy/fmt. Selected closure has zero differences; the
registered common-owner-pure-summary.json preserves exact counts and the disk-
versus-RAM admission clarification. This qualifies ONLY the new L1/static bytes:
FIVE actual capture parents, direct-pipe consumer0/74, actual provider queries,
ACL-at-create/WTR/producer association and privileged/hosted gates remain unrun.
No source-only status or pure success upgrades native producer authority.
