---
id: sot:root/cross-os-runners
title: Cross OS runner qualification and infrastructure research
status: active
created: 2026-11-06T22:37:47Z
updated: 2026-11-06T23:09:29Z
verify: manual
rel:
  depends_on: [sot:root/delivery, sot:root/adapter-ecosystem]
---

# Cross OS runner qualification

Document version: 0.2.0. Research checked 2026-11-07 KST. Accepted execution
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
