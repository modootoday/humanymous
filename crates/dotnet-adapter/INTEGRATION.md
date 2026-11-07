# Integration requests

## Current approved continuation

### Hosted ordinary actor handoff contract (prepared, not compiled)

Root assigned dotnet-validation.yml after CI explicitly excluded that literal
file. This workflow covers only .NET on standard Windows/Linux x64 runners;
Python/Apache remain separate owners. Its stable jobs are preflight, windows-x64
and linux-x64, with independent outcomes and concurrency
dotnet-validation-${{ github.ref }}. Push is limited to codex/dotnet-validation
plus manual dispatch. Contents read only; no OIDC, deployment, publication,
automatic artifact upload or remote cache save. Only bounded allowlisted public
facts/hashes may reach logs; private report bundles remain runner-local.

The sole SDK/package controller remains the existing package integration-test
binary, built from crates/dotnet-adapter/tests/package.rs. Its actual ordinary
child invocation is fixed, not caller-supplied executable text:

```text
<package test executable> --ignored --nocapture --exact sdk_package_and_real_host_matrix --test-threads=1
```

The controller packs the candidate, creates a clean consumer, restores/publishes
source/packed JIT and AOT, invokes the separately built hosted.rs executable for
each mode and verifies installed native bytes. It does not build Cargo, create
accounts, impersonate tokens, install SDKs, grant SCM rights or publish packages.
SDK installation and release Rust builds occur as separate hosted prerequisites.

Proposed native-Windows-owned hook: standard-user-dotnet, reusing the existing
ephemeral Users account, token verification, suspended-child/job-kill-on-close,
protected private parent and cleanup path. No adapter-local account/unsafe code.
Its input manifest schema humanymous.dotnet-actor-input/1 contains source_root,
source_sha and hashed absolute paths for controller, host_harness, native_library
and native_provenance. The provenance file uses the strict existing schema above.
The manifest also identifies sdk_root, git_executable and read-only native
toolchain paths/environment. The wrapper verifies source/artifact identity before
launch; an arbitrary executable/argument launcher is not requested. Fixed child
test arguments above are part of the hook, not manifest-supplied shell strings.

Required child inputs are HUMANYMOUS_DOTNET_SOURCE_ROOT,
HUMANYMOUS_DOTNET_NATIVE_LIBRARY, HUMANYMOUS_DOTNET_HOST_HARNESS,
HUMANYMOUS_DOTNET_NATIVE_PROVENANCE and HUMANYMOUS_DOTNET_FIXTURE_PARENT. The
wrapper supplies a new protected owner-only inheritable parent owned by the
actual ordinary child. Scratch under it contains private CLI home, fresh NuGet
caches, build outputs, audit parents and logs. Never grant the actor source or
prebuilt artifact write access. No credentials/runner tokens are copied.

The SDK/toolchain environment allowlist needs SystemRoot/WINDIR/SystemDrive,
COMSPEC, TEMP/TMP, private USERPROFILE/DOTNET_CLI_HOME/NUGET_PACKAGES, DOTNET_ROOT,
the explicit telemetry/certificate/no-global-tools settings documented below,
and a constructed PATH for System32, verified SDK, Git and native compiler/linker.
NativeAOT may require existing ProgramFiles/ProgramFiles(x86), VSINSTALLDIR,
VCINSTALLDIR, VCToolsInstallDir, WindowsSdkDir, WindowsSDKVersion,
WindowsSDKLibVersion, INCLUDE/LIB/LIBPATH; validate bounded existing local paths,
not ambient arbitrary executable strings. Git needs process-local safe.directory
for the one verified read-only checkout owned by the provisioning user; no user/
system Git configuration write. Genuine hosted facts may be forwarded only
after the wrapper verifies its disposable hosted venue and exact GITHUB_SHA;
local profile never forges those facts.

Controller subprocess budgets remain SDK restore600s/publish1200s and one
host harness180s, with bounded private stdout/stderr. Proposed outer Windows job
deadline is30minutes, below a60minute workflow job; timeout is a real failure,
and job-empty/account-deleted/scratch-deleted proof is mandatory. The owner must
confirm the bounded launch API rather than silently using an incompatible limit.

The existing private success report is dotnet-package-run/package-observation.json
with schema humanymous.dotnet-package-observation/1. A prepared additive
HUMANYMOUS_DOTNET_OBSERVATION_PATH will hold a bounded typed controller outcome
on success or failure so the native wrapper can recover it before cleanup.
The safe public projection schema humanymous.dotnet-component-summary/1 permits
only source/tool/artifact/package hashes, OS/arch/mode, exact counts, required
case IDs, fixed stage names and cleanup booleans. Paths, SIDs, raw diagnostics,
headers, credentials and arbitrary error messages are excluded. This outcome
extension/hosted bootstrap is not yet authored or compiled. The wrapper's actual
child exit and cleanup proof must agree before a passing component projection.
Remaining required contract cases stay NOT_RUN and prevent full qualification;
exit2/incomplete must never be normalized to workflow success.

Ordinary-private cleanup clarification from the native owner: the child runner
must create and protect its own new private root before invoking this controller;
the current admin-plus-actor TEMP inheritance is not a qualifying parent. The
administrator must never repair/take ownership of that private root to read or
remove it. A separate restricted admin-plus-one-actor handoff directory is the
only publication channel before exit. Proposed extra inputs are
HUMANYMOUS_DOTNET_HANDOFF_DIR and HUMANYMOUS_DOTNET_OBSERVATION_PATH beneath it.
The controller will copy only whitelisted regular non-reparse result files and
the last failed stage's bounded stdout/stderr, with a relative-name/length/SHA256
manifest. Proposed limits are32files,256KiB per raw stream,8MiB total and64KiB
per typed outcome; partial copied logs explicitly retain original length/hash
and truncation facts. Feeds, caches, SDK directories, credentials and whole
private trees are never copied. Raw handoff stays private on the runner.

The controller must attempt self-cleanup of its own validated private work root
after publication and report its real outcome. If a timeout/crash prevents this,
the native owner proposes a bounded same-generated-user cleanup child after the
main job is empty, before account deletion; it reuses the same account/token/job
boundary and validates the exact owned nonce path. Admin ACL repair is refused.
Child-self-cleanup or this cleanup-child proof must precede account/scratch
deletion success. This cross-owner cleanup contract is still under review;
the workflow cannot claim actual Windows readiness until accepted and compiled.

Release C returned: standalone clean source2ffa4335df49c111685a764a81f8e4b414685cd4
built the Rust facade DLL in76.032s and hosted Rust test executable in12.616s.
Both actual exits0,261recursive inputs unchanged and source clean afterward.
Artifacts were selected from Cargo JSON and hashed; strict native-provenance.json
and exact paths are private evidence100408/artifacts.json. Public evidence:
evidence:root/20261107100735. No test/SDK compilation/PInvoke/host execution follows
from this --no-run build. Actual source/packed JIT/AOT/GC stages remain NOT_RUN.

### Release build preparation and reproducible commands

Root ran the release facade and hosted Rust harness before the separately
allocated actual managed fixture. No repeat build slot is assigned by this text.
Freeze the owned implementation/configuration before commands; coordinate the
recursive dependency closure through root, snapshot it before/after, and reject
drift. No manifest, source or lock changes during compilation. Preserve the
actual clean checkout HEAD and Cargo.lock hash. A shared dirty workspace HEAD
alone cannot populate the strict clean-source provenance expected by package.rs;
root must provide a committed clean source checkout that contains these files.
Root owns checkout/worktree setup and any common graph edits.

After explicit allocation, use Rust1.99.1/edition2024, CARGO_BUILD_JOBS=1 and
CARGO_INCREMENTAL=0, the assigned CARGO_TARGET_DIR, and the actual host target
x86_64-pc-windows-msvc. Record environment/target rather than guessing artifact
paths. Run these commands from that exact source checkout, serially, checking
each real exit before continuing:

```text
cargo build --locked --release -p humanymous-dotnet-adapter-export --message-format=json-render-diagnostics
cargo test --locked --release -p humanymous-dotnet-adapter --test hosted --no-run --message-format=json-render-diagnostics
```

Capture both stdout JSONL and stderr privately, including elapsed time, compiler
identity and resource admission/exit measurements. Select the actual facade
filename from its compiler-artifact record and the hosted test executable from
its test target compiler-artifact.executable; do not select the newest similarly
named file from a shared target cache. Hash those exact outputs using stock
SHA256. The strict provenance JSON requires source_sha, cargo_lock_sha256,
native_sha256, harness_sha256, rustc, target and native_profile=release; omit the
optional generic_sha256 unless its separate real generic ABI artifact is supplied.
These two builds do not execute any actual host test. The package.rs controller
also needs its own compiled test executable in a later assigned stage; do not
recursively build Cargo from the controller or silently add it to this allocation.

### Later managed fixture prerequisites

The prepared SDK's exact executable, verified official ZIP hash and private
paths are in private evidence094726/readiness.json. Invoke that executable
explicitly. The package controller resolves dotnet through PATH, so prepend
only the private SDK install directory to the supervisor process's PATH; never
change the user/system PATH. Set DOTNET_ROOT to that directory, private
DOTNET_CLI_HOME and NUGET_PACKAGES, DOTNET_CLI_TELEMETRY_OPTOUT=1,
DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1, DOTNET_NOLOGO=1,
DOTNET_GENERATE_ASPNET_CERTIFICATE=false,
DOTNET_ADD_GLOBAL_TOOLS_TO_PATH=false and
DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE=true. DOTNET_CLI_UI_LANGUAGE=en-US
makes future tool diagnostics consistent; preserve earlier raw localized logs.
The SDK working directory remains the checkout's adapters/dotnet so global.json
resolves10.0.403. MSBuild runs one node, no shared compilation or node reuse;
source/packed builds use separate private output roots and fresh package caches.

Windows NativeAOT requires Visual Studio2022 or later with Desktop development
with C++ and its default components, per the official Native AOT prerequisites:
https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/ . Discover and
record the existing native compiler/linker/Windows SDK; prior Rust linking alone
does not prove the full NativeAOT prerequisite set. No administrator installer,
account creation, SCM/service operation or global setup is authorized here.
If a prerequisite is absent, report it and retain that tuple as NOT_RUN.

An actual Windows host fixture requires the shared ordinary-user identity checks
with no enabled administrator identity and an existing verified private parent.
The SDK owner-only installation does not grant another account access. Root and
the native-Windows owner must supply the actual actor and controlled readable
artifact/source layout without relaxing the shared protected-directory checks.
Each source/packed JIT/AOT run creates its own new audit-parent children. Keep
production ApplicationHttp negative evidence separate from synthetic immutable
evidence passed through the real authority/audit engine. No SDK preparation or
release compilation establishes managed GC, PInvoke, Kestrel or package behavior.

The shared create_user_private_directory implementation calls current_identity
and installer_allowed, rejecting elevated or enabled-administrator tokens. It
opens the local non-reparse parent and verifies both ProtectedDirectory and
InheritableDirectory before creating a new child; it then verifies the child's
owner, installs the same protected owner-only inheritable DACL and re-verifies.
It does not repair an existing arbitrary parent or accept ambient TEMP. The
prepared SDK owner-only root is a candidate parent, not established shared-helper
proof; each actual run must use a fresh child layout and retain helper failure.
The hosted runner creates production/synthetic children and refuses existing
children. Never rerun against an already-used audit parent.

For explicit local fixtures set HUMANYMOUS_DOTNET_LOCAL_ALLOCATION to
root-approved-local only after root allocates the actual run, and remove inherited
GITHUB_ACTIONS/RUNNER_ENVIRONMENT from that local child environment. Report
execution_environment=root-allocated-local. Do not spoof standard hosted runner
variables. Manual native DLL placement is permitted only in the source-reference
JIT fixture with its original source/hash recorded; it is never a clean installed
NuGet consumer result. The controller's packed fixture must restore its native
asset from the measured nupkg and compare exact installed bytes.

SDK preparation A returned: official SDK10.0.403Windowsx64 ZIP hash verified and
private installation ready. Only --version/--info ran successfully, reporting
runtime10.0.14; no managed build/host. Evidence094859/private094726. Exact executable
and environment paths are in private readiness.json and supplied directly to root.
Fresh owner-only ACL does not qualify a different ordinary actor; package/host
fixtures still require their separately verified actor/private parent. No global
PATH/admin installer/SCM/account operation was used. Next build needs allocation.

Quiet ended by explicit root instruction. Reclaimed adapter-dotnet; allocated A
validated latest all-target check/Clippy and core12/export1pure, all passed with
zero failures/ignored and259recursive source/configuration inputs unchanged.
Raw evidence094127/public094251. A returned immediately. The new package/support/
fixture source is now type-checked; no managed SDK/scope execution/actual host is
implied. SDK/Kestrel/GC/AOT/package still await their separately assigned stage.

Root6500ec23 closed the approved graph window with38workspace members and the
shared lock, including this export crate. The owned manifests now match that
approved graph. Explicit local A validation passed core12/export1pure and both
all-target check/Clippy;252recursive selected input hashes did not drift. Slot
returned, raw evidence084546/public084728. No managed SDK/scope execution/host
pilot is implied. No further graph changes without root's next window.

Next allocated actual gate needs SDK10.0.403 from adapters/dotnet working directory, native
facade artifact, JIT/NativeAOT published apphost, a fresh private ordinary-user
audit PARENT, and the exact ignored Rust host selector. The driver creates NEW
production/synthetic children; do not precreate them. Each source/packed JIT/AOT
invocation needs a distinct private parent. The post058fixture delta consumes
the shared initializer and remains compile-pending until its next check.
Windows initializer API is supplied by native-windows; never accept arbitrary
TEMP/admin ownership. Root remains sole dispatch/push and CI owner owns workflow.
Final clean NuGet install, remaining contract cases and Docker product evidence
remain required. No published package or release-qualified tuple exists.

Packed consumer restore must use NuGet.Packed.Config with
HUMANYMOUS_NUGET_FEED pointing to the just-built local feed and NUGET_PACKAGES
pointing to a NEW empty per-run package directory. The exact candidate ID maps
only to that feed; SDK Microsoft/System/runtime packages map only upstream.
Do not allow a same-ID public package or existing global cache to stand in for
the measured local nupkg. Use UsePackedAdapter=true and compare package/native
hashes after restore/install. Pack metadata includes owned candidate NOTICE,
README, project LICENSE and the source-tree audit compatibility notice; full
resolved dependency/license/vulnerability qualification is still required.

The human approved exactly three managed binding/middleware/pilot sources and
reactivated this goal. Root confirmed the exception. The prior blocked handover
below is historical; it is not a current permission requirement. Exact source
paths and the automated inventory gate are in sot:root/dotnet-adapter.

Native ABI owner approved/provided validate_context, buffer_bytes and public
Version::validate. The safe bridge uses these through one Boundary/Registry.
Its namespace100..109 owns .NET request composition and bounded diagnostics only. Generic handle/error/
buffer/version ownership is unchanged. Replay copies precede the final native
expiry/host/route check; no lock spans IPC and no managed address is retained.

Applied root-approved graph delta: core native-abi path dependency,
serde/serde_json/tokio workspace dependencies; hosted test dev-dependencies audit,
signals, native-security and test-support; separate export workspace member with
core/native-abi/tokio dependencies, cdylib+rlib and deny unsafe except the single
unmangled pointer-free attribute. No new external version is needed. Local dotnet
was unavailable at the earlier checkpoint. Actual managed build/GC/Kestrel/
NativeAOT/package work may use root-allocated local Windows/WSL or a standard
hosted job under the updated root policy. Benchmark quiet has ended.

The new ignored sdk_package_and_real_host_matrix test accepts these absolute
supervisor inputs: HUMANYMOUS_DOTNET_SOURCE_ROOT (clean committed checkout),
HUMANYMOUS_DOTNET_FIXTURE_PARENT (existing verified private ordinary-user parent),
HUMANYMOUS_DOTNET_NATIVE_LIBRARY (release DLL/SO),
HUMANYMOUS_DOTNET_HOST_HARNESS (prebuilt hosted.rs test executable) and
HUMANYMOUS_DOTNET_NATIVE_PROVENANCE (strict JSON). The JSON fields are source_sha,
cargo_lock_sha256, native_sha256, harness_sha256, rustc, target, native_profile
(release) and optional generic_sha256. An optional
HUMANYMOUS_DOTNET_GENERIC_LIBRARY must match that separate hash. Every supplied
artifact must be built from the recorded source; checks do not create provenance.
Local execution additionally uses HUMANYMOUS_DOTNET_LOCAL_ALLOCATION=
root-approved-local after root actually allocates execution. This marker is not
permission. No recursive Cargo, GitHub spoofing or workflow dispatch occurs.
The orchestrator passes Rust all-target check/Clippy but has not executed.

The three managed source candidates and XML project/package metadata are authored
but NOT_RUN. A candidate NuGet pack requires an explicit existing native artifact
and RID, and never publishes. Production authority negative outcomes and frozen
synthetic evidence in the real engine/audit for compatibility ALLOW must remain
separate profiles. Do not call these browser/capture qualification.

## Historical pre-approval requests and handover

Root owns Cargo workspace and lock. Register humanymous-dotnet-adapter with
framework-adapter, authority and integrity path dependencies and serde_json dev
dependency. No new third-party dependency is requested.
Workspace/profile/encoder integration is now present; these requests are fulfilled.

Integrity owner: add DotnetAspNetCore provisioned profile, exact ID
humanymous.dotnet, version0.1.0-experimental.1, aspnetcore-normalized only,
rawHeaderOrder=false, peer/connectionId/capture/stream null. Framework owner:
encode selected profile using that mode; preserve existing parser validation.

Native ABI owner: request/payload handles and C exports belong to your crate.
This crate owns safe body/lifecycle behavior and accepts no pointers. No unsafe
or foreign host glue exception is approved. Actual Kestrel/NativeAOT package is
blocked on an authorized binding mechanism, not replaced by a mock.

CI owner: schedule scoped units/fmt/Clippy and future actual Windows/Linux host
pilot only on standard free GitHub runners. Root dispatches. Qualification must
bind source and artifact hashes, runtime/OS/arch/link mode and preserve ALLOW,
CHALLENGE,DENY,error, failed, ignored, unsupported and origin-effect counts.
The exact21case/fourtuple candidate contract is adapters/dotnet/pilot-contract.json;
it is runnable=false until approved actual managed binding exists. Contact and
application mutation counters are separate. PInvoke/GC/NativeAOT are not covered
by a Rust-only test driver or HTTP greeting fixture.

## Exact unresolved mechanism decision

Minimum exception proposal, NOT approval: allow reviewed native export attributes
and any necessary pointer-copying facade only in native-abi's owned boundary;
keep this crate forbid(unsafe_code). Allow one reviewed managed LibraryImport/
SafeHandle binding source, one ASP.NET InvokeAsync middleware source and one
actual host pilot source under adapters/dotnet, with no scoring, crypto,
policy, credential minting or capture authority in managed code. NativeAOT must
use the same fixed ABI and release artifact as JIT, not a separate detector.
Root must obtain the explicit rule exception before any of those foreign files
or native exports are authored. Pooled byte memory is copied during each call;
no managed pointer survives an await and no reverse managed callback is required.
Current partial root approval in native-abi is no_mangle on its pointer-free
by-value Command->Response export only. It does not approve managed sources,
unsafe blocks, raw pointers or dynamic-loader code in this adapter.

Rust-only alternative: independently qualify the existing Rust Gate in front
of an untouched upstream .NET HTTP origin fixture. That proves only the HTTP
origin deployment tuple and cannot fulfill the named PInvoke/NativeAOT/GC
middleware scope. It must not replace this goal or populate its qualified tuples.

Root checkpoint validation: c23b504 has7pure tests and scoped library Clippy pass,
directly observed in completed command exec-b6805700-3564-4e01-bd4e-b2a091e9c933.
New DecisionAttempt cancellation-on-drop source has8unit cases and is NOT_RUN;
request the next scoped root unit/lint slot without manifest/dependency changes.

Public docs delta for docs owner: .NET currently has safe Rust lifecycle source;
no installed middleware, managed GC, PInvoke/NativeAOT, NuGet or qualified tuple.
No full-product protection claim follows. Reference dotnet-adapter SoT.

## Handover: full objective blocked, not complete

Mission remains actual ASP.NET/Kestrel PInvoke/NativeAOT lifecycle, cancellation,
body replay, GC ownership and package qualification. Follow AGENTS.md,
.agents/lessons/HARD-WON.md and the owned dotnet-adapter SoT/plan first.

Done: c23b504 safe request lifecycle, exact binary body and shared authority
ownership; root7pure/libraryClippy green directly inspected (public evidence
073832/private073833). 3b13f01 adds decision-future-drop RAII cancellation,
eighth pure regression and21case real-host qualification contract. Latest eight
cases have NOT_RUN status and remain queued with root. Owned paths are clean.

Blocker across three goal turns: authored managed binding/middleware/pilot remain
unapproved and absent. The narrow native-abi export exception is insufficient.
No actual .NET/GC/NativeAOT/package/Docker result exists; qualified tuples stay
empty. Do not infer completion or replace this scope with an HTTP greeting.

Next: root obtains/records an approved host mechanism; resume this chat's goal,
list claims and reclaim adapter-dotnet normally; inspect latest root validation
of source637f259b7f3cb274a4387dd98026859577ca63689acb3ebefefefd85840b6c7f;
integrate ABI via its owner, then run the tracked host contract on free hosted
runners. Source/manifest/feature changes need root graph-window coordination.

Verify (only on root allocation): cargo test --locked --offline
-p humanymous-dotnet-adapter --lib -- --test-threads=1; cargo clippy --locked
--offline -p humanymous-dotnet-adapter --all-targets -- -D warnings. Direct fmt:
rustfmt --edition2024 --check crates/dotnet-adapter/src/lib.rs. Owned Specfold
check is allowed without Cargo. Actual processes/IPC/OS/Docker remain hosted.

Out of scope: shared Cargo/lock/toolchain, native ABI/shared engine/CI edits,
docs/**, score changes, publication, paid infrastructure and local actual host
execution. Root chat owns orchestration; residual details are in this Codex chat
and canonical source/evidence, not Claude memory. Work-lane release note links
this handover; no force release or staged-path manipulation is needed.
