---
id: sot:root/delivery
title: Rust delivery and Pages contracts
status: active
created: 2026-11-06T20:27:03Z
updated: 2026-11-07T05:20:00Z
verify:
  - cargo run --locked --manifest-path scripts/ci/delivery/Cargo.toml -- policy
  - cargo test --locked --manifest-path scripts/ci/delivery/Cargo.toml
  - cargo test --locked --manifest-path scripts/ci/docs-review/Cargo.toml
rel:
  governs: [docs/**, .github/workflows/**, scripts/ci/**, scripts/migration/**, build/rust/**]
  depends_on: [sot:root/rust-migration]
---

## Collector component validation delta c3389b6

collector-component.yml adds read-only, SHA-pinned Actions validation of actual
Rust WASM tests in the network-none, non-root Docker Firefox component lab.
It uploads image metadata and test output by source SHA and publishes no package
or image. This independent component check does not replace any existing native,
browser or Docker full-delivery gate. Remote run42327415870 was started; its
conclusion subsequently succeeded with3actual browser tests; the downloaded
source-bound artifacts are recorded with the program evidence. Full-runtime qualification and
the final delivery runner requirements remain unchanged.

## Rust-only delivery follow-up contract v2

### Later user authorization: local versus hosted measurement

The user subsequently authorized actual local Windows/WSL build, test and
runtime timing/resource comparison against GitHub hosted execution. This
supersedes the earlier blanket hosted-only restriction; historical hosted
results and their source commitments remain unchanged. Root owns the isolated
committed-source comparison and the measured execution-policy decision.

During the explicitly declared benchmark quiet window, do not start new Cargo,
native/runtime, Docker or model commands. Existing owned commands finish and
return their allocated slots; never terminate another owner's processes.
Coding/documentation may continue under existing lane ownership. A local
comparison result does not yet exist and no new execution policy is inferred
from the authorization alone. Common Cargo/locks/toolchain remain root-owned.

The subsequent benchmark instruction selects Windows act.exe and the verified
Docker Desktop Linux engine: Windows docker.exe with explicit desktop-linux
context, and ACT's explicit outer DOCKER_HOST
npipe:////./pipe/dockerDesktopLinuxEngine. Preflight records endpoint, server ID,
version and OSType. The separate WSL-native engine is not a fallback; WSL-native
Rust measurements are a separate dataset. Local parity records the canonical
provider/host and outer-observed engine ID. In-container metadata is not an engine
or VM attestation; hosted Linux runner identity remains distinct.

The isolated local-parity.yml benchmark uses a separate branch trigger and
concurrency group, one standard Ubuntu job and the same pinned act24 image in
both venues. Requested and checked limits are four CPUs, 2 GiB RAM, no additional
swap, jobs one/incremental zero. Missing/unbounded cgroup limits or GNU time
metrics fail explicitly. The Rust action bootstraps 1.99.1; the dependency-free
Rust driver prepares GNU time identically through native apt-get commands and
records setup durations/package version separately. It materializes exact public
8132117 workload source without replacing the new control tool, then measures
collector 26, workspace all-target build and native process one, optionally
repeating in the same job. Source/code/lock hashes, exits, counts, CPU/RSS and
context remain explicit; neither cache coldness, hardware equivalence nor
execution policy is inferred. No artifact upload, publication, OIDC or product
qualification occurs. Source preparation alone is not a measured comparison.

### Remote lab design delta, 2026-11-07 KST (not executed)

Initial root resource triage authorized design only. The later explicit user
request authorizes applying free standard public-repository hosted validation;
root integrates and performs the necessary branch push/Actions execution.
No new local build, paid resource creation or deployment. Project heavy execution has one
coordinated slot. Priority remains runtime/collector source integration before
whole-workspace, wargame or render retries. Container owner governs provider
security equivalence in container-test-environments; this section governs CI
transport, source/evidence and cost selection. No duplicate lab implementation.
Detailed provider boundaries live in
[container-test-environments SoT](container-test-environments.sot.md), section
"Remote execution design delta (2026-11-07, not_run)".
That owner records Containers DinD/host-network incompatibility, independent
witness durability gaps, Cloudflare reTLS L1/L2 provenance, outbound controls and
Rust/cf lifecycle not_run. Provider compute examples exclude plan, Worker/DO,
logs and egress and do not qualify a whole-lab replacement or a spending cap.

Prefer an existing GitHub-hosted Ubuntu VM for main product e2e investigation.
Reuse delivery `pilot docker`/`pilot browser` and owning Rust lab runner/Compose;
do not introduce a shell/Python/JS coordinator or run product e2e on host loopback.
Cloudflare Containers remains a supplementary actual-Cloudflare API candidate;
its UID/namespace/egress/sandbox/witness and Rust control-plane compatibility
remain unverified, not equivalent to the existing Compose lab.

Official facts checked 2026-11-07 (not performance or account-capacity proof):

| Candidate | Cost and documented capacity | Decision boundary |
| --- | --- | --- |
| GitHub standard public Ubuntu24.04 x64 VM | Public standard compute free;4CPU/16GBRAM/14GBSSD | Main candidate, actual build RAM/disk/runtime not_run |
| Cloudflare Containers | Workers Paid $5/month; included25GiB-hours memory,375vCPU-minutes,200GB-hours disk; overage $0.0000025/GiB-second memory,$0.000020/active-vCPU-second,$0.00000007/GB-second disk | Additional Workers/DO/logs/egress charges; no account or resource change authorized |

Repository visibility was read-only checked PUBLIC with `gh repo view`; no
private-repository or larger-runner free-compute assumption. Artifact/cache
storage allowances and charges are separate. Cloudflare memory/disk billing is
provisioned capacity while awake; CPU billing is active use, so a bursty test
needs measured usage before a dollar forecast. No cost estimate is a budget cap.

Official references: [runner specs](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
[billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions),
[Ubuntu24.04 software](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md),
[job limits](https://docs.github.com/en/actions/reference/limits),
[artifact access/retention](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/download-workflow-artifacts),
[Containers pricing](https://developers.cloudflare.com/containers/platform/pricing/),
[Containers limits](https://developers.cloudflare.com/containers/platform/limits/).

Future execution contract, pending root reassignment and source freeze:

1. Use explicit ubuntu-24.04 VM, not moving latest/slim, one heavy job at a time.
   Preserve existing required checks; a new scoped remote pilot cannot replace
   full-release gates. Current browser/docker jobs are parallel latest-label
   jobs, so the proposed serialized remote design is not implemented yet.
2. Root commits the common lock and every producer prerequisite. Remote checkout
   must be an available immutable commit with exact workflow SHA; local dirty
   bytes are not silently represented by HEAD. Push remains separately forbidden
   until authorized. Do not upload arbitrary local private trees to make it run.
3. Capture OS/runner-image/toolchain/Docker/Compose versions, available RAM/disk,
   source/lock/docs-review hashes and immutable owner snapshot before execution.
   Pin actions to commits; mutable VM image versions are recorded, not described
   as immutable. Assess compilation and image footprint before choosing capacity.
4. Keep current Docker security policy, image digest/source proof, internal
   topology and independent witness unchanged. Network access for dependency
   acquisition is outside the measured isolated runtime. Privileged Docker host
   access is not a successful nonroot container or process-actor assertion.
5. Reuse fresh evidence bundle per attempt. Exit2/component-only stays blocked;
   exit1/assertion or infrastructure failure stays failure; missing/ignored full
   cases cannot turn green. Record cleanup attempt and verify owned objects;
   cancellation/timeout means interrupted unless cleanup/evidence actually exists.
6. Public-repository logs/artifacts are not a private raw evidence store. Upload
   only allowlisted sanitized summaries, hashes and required nonsecret metadata.
   Current target/delivery-evidence summaries qualify for review; raw private
   transcripts, actor identities, cookies, credentials and private spec stay out.
   Restricted durable raw-evidence transport remains an unresolved prerequisite
   for complete remote auditability; never claim a hash alone preserves raw data.
7. Bind result to repository/workflow/run/attempt/source and artifact IDs/digests.
   observation/2 collects attempt-specific logs and source/time-window artifacts;
   subsequent runs cannot replace failed-source evidence. Set explicit short
   artifact retention when implemented; default is90days, read-access downloads.

All proposed remote resource, sandbox-equivalence, runtime/collector connection,
storage headroom, source transport and raw-evidence retention checks are not_run.
Only official-document inspection and existing-source review were performed.

Implementation delta after free-hosted authorization: existing validation jobs
are serialized policy→docker→browser→native→msrv→quality→images→packages→node→
advisories→documentation→pages; native/image matrices have max-parallel1. Native
standard runner labels are explicit Ubuntu24.04/macOS15/Windows2025. No optional
profile removes required gates. A predecessor failure leaves later jobs not_run
and ready rejects skipped dependencies. CI entrypoint keeps same-ref cancellation.
CEF/collector/cross-OS component pushes now target main, preserving manual/PR
entrypoints while preventing duplicate codex/rust push builds. They cancel
superseded same-ref runs; cross-OS matrix runs serially.

Regular branch validation writes no action cache or artifacts by default. Pages
bundler cache is disabled. Tag-only validated release archives retain their
existing publication purpose, with1day retention; Pages deploy artifacts are
main-only/1day. Component artifacts require explicit CI_COMPONENT_ARTIFACTS=true,
with1day retention and review before enabling; raw private bundles are never
uploaded. Pilot prints the existing allowlisted public summary to job log and
GITHUB_STEP_SUMMARY, including on full-product rejection. Root retrieves exact
run/attempt logs privately. Native/raw reports left only on an ephemeral hosted
VM are not durable retained evidence; this remains a limitation, not success.

Local checks for this delta are actionlint and diff whitespace only. Rust summary
emission is formatted but tests/build are deliberately deferred to hosted CI.
No remote run has yet been initiated by this owner. First root dispatch after
source closure is `gh workflow run ci.yml --ref codex/rust`; an authorized branch
push already triggers ci, so do not dispatch a duplicate. Root must record the
resulting exact run ID/SHA/attempt and preserve all failed/not_run outcomes.

Consultation correction supersedes the fully serial draft above: after policy,
two heavy lanes execute independently. LaneA native→msrv→quality→documentation→
pages; laneB docker→browser→images→packages→node→advisories. Each matrix has
max-parallel1. Lane continuation jobs directly need policy and their predecessor,
with the only permitted scheduling condition
`${{ !cancelled() && needs.policy.result == 'success' }}`. This gathers independent
results after a preceding failure without running unvalidated policy or ignoring
failures. Rust workflow policy accepts only that exact condition with a direct
policy dependency; no continue-on-error or conditional pilot steps are allowed.
Ready keeps its exact complete needs set and fails on every failure/skip/cancel.
Do not use one shared concurrency group for every job: pending replacement could
discard required work. Same-ref CI workflow cancellation remains in ci.yml.
This bounds heavy validation to two hosted jobs, separate from the local resource
gate. Interactive human/AT/SCM/CEF and physical durability claims still require
their own actual authority, not a hosted-runner label. No required gate removed.

Final cross-owner policy confirmed by root after all13 owners responded:
[program-roadmap](program-roadmap.sot.md) owns the single project policy; this
document owns its delivery implementation contract. Local work is coding,
formatting, scoped Clippy and resource-bounded pure units, with at most one local
Cargo task after resource review. Real process/port/IPC/ACL/SCM/browser/CEF/Docker
and cross-OS execution belongs on public standard hosted runners. Test file names
alone do not establish purity. Human/AT/IME, physical devices and organizationally
independent witnesses still need direct evidence in a suitable environment.
The two-lane limit is workflow-local, not a global mutex for manual workflows.
Do not start duplicate dispatches. Until an impact classifier is implemented,
do not remove required gates using path filters. Free hosted compute does not
include LLM/API inference or imply artifact/cache storage is free. Missing private
raw recovery remains explicit; old successful evidence never qualifies later
unexecuted source. Root fixes source closure, pushes the Rust branch and observes
the first hosted run; owners do not resume local runtime/research workloads.

Pinned-action storage audit correction: trivy-action ed142fd defaults cachetrue
and passes it to setup-trivy3fb12ec; explicitly set cachefalse for both scans.
setup-buildx bb05f3f defaults cache-binarytrue; disable both instances. build-push
53b7df9 defaults automatic build-record artifact upload; workflow environment
DOCKER_BUILD_RECORD_UPLOAD=false disables it. setup-node2499707 automatic package
manager cache is explicitly false; setup-bun0c5077e binary cache disabled with
no-cachetrue. Toolchain683b588 emits a cache key but has no cache action. Findings
were inspected via official repositories' exact pinned action metadata/README,
not inferred from top-level upload steps. Pages Ruby bundler cachefalse and
main-only Pages upload remain: free Rust branch gets no Pages artifact, existing
main deployment still needs its checked artifact. This is a policy against paid
compute/cache/artifact use by this branch run, not a guarantee of zero total
personal/organization charges or zero third-party inference costs. Hosted
confirmation of no cache/artifact writes remains pending the first actual run.

First hosted run42347732666/attempt1/source
bfc9f62335ae41906996c6b01b0ae9e7993feeed completed failure. Delivery22 tests and
workflow policy passed; action-pins failed on authenticated public aquasecurity
commit lookup with organization IP allow-list HTTP403. The required lanes stayed
not_run and ready rejected their skipped outcomes. No successful runtime claim.
Public artifact API reported0; cache inventory contained3 older entries with
creation/access before this run, not evidence of new cache writes. This policy
failure did not exercise Trivy/Buildx/Bun, so their disabled storage behavior is
still not_run. Root retains the original run/job logs privately.

Follow-up pin contract: use unauthenticated public GitHub commit REST and exact
SHA-addressed public raw action metadata, no repository installation token.
GitHub documents public Get-a-commit as authentication-optional:
https://docs.github.com/en/rest/commits/commits#get-a-commit . This is a public
resource least-privilege path, not authenticated fallback or an upstream IP-rule
change. Exact40SHA identity and runs entrypoint checks remain mandatory; HTTP403,
quota/network failures, invalid JSON, missing/mismatched SHA all fail closed.
Keep unique-action deduplication, report lookup index/total on failure, no retry
loop or paid fixed-IP service. Shared-host unauthenticated quota is a stability
limitation. Local read-only direct public commit+raw metadata lookup succeeded;
the corrected hosted source still requires its own actual run.

### Workflow identity convention

Explicit obsolete-cleanup delta: frozen-go-oracle.yml no longer invokes legacy
agent adapter synchronization, vendor mirror diff or shell layout verification.
Current Rust validation uses pinned Rust1.99.1 canonical agent-maintenance check
and compile targets. The historical oracle no longer runs current Rust targets
after its checkout was pinned to the immutable pre-migration baseline.
The unrelated frozen attack assertion test remains; legacy Node agent/hook tests
are no longer maintenance authority. No vendor trees are regenerated. Root owns
verified mirror deletion and canonical rule changes; Go/oracle product paths,
settings/hooks and immutable history are not deleted by this delivery change.

NuGet preparation candidate for the subsequent C/C++/C# adapter scope:
catalog publisher_candidates.nuget records research-only/release.yml and proposed
environmentnuget, with environment_created/job_present/verified/package_readyfalse
and package_idnull. It is neither a provisioned GitHub environment nor a fifth
ready registry. Actual four registry prerequisites remain separate. Typed policy
rejects a provisioned/verified/publishing claim while this candidate state holds.
No NuGet job/action/token permission is added; root decides environment setup.

[Official NuGet trusted publishing](https://learn.microsoft.com/en-us/nuget/nuget-org/trusted-publishing)
binds account/active-organization policy ownership, repository owner/name,
workflow filename and optional environment. Package glob/scopes distinguish
new-package versus new-version authority. Exchange yields a one-hour API key;
some policies await identity activation. Our proposed protected nuget environment
and exact account/package scope remain unconfigured/unverified; no exchange or
upload test is authorized. Any future login action must be pinned/reviewed.

C/C++ headers/native artifacts and CMake/pkg-config/vcpkg/Conan distribution are
separate ABI/package contracts, not one language-wide OIDC publisher. .NET package
identity does not prove a C# host adapter works. Adapter owners must establish
actual Rust native ABI, provenance/generated glue, framework execution and archive
contents before a package gate can qualify them. No duplicate core or fictitious
language support is introduced by this CI candidate.

Lab diagnostic and product browser consumer source: single owned
test/lab/runner/safe-diagnostics.json defines the vocabulary/limits. Present
malformed diagnostics add an execution error; only reconstructed typed rows are
public, never raw detail. Legacy absent diagnostics staynull. `pilot browser`
selects --collector; `pilot browser-product` selects only --browser-product,
uses a fresh bundle and recognizes the dedicated image digest. Full browser
requirements remain identical/strict. New required-job DAG wiring awaits root
coordination; actual product browser execution remainsnot_run for this source.

Go adapter delivery distinction (subsequent explicit user scope): a future Go
framework adapter is a separate module responsibility, not maintenance of frozen
Go product/scoring source. [Official module publishing](https://go.dev/doc/modules/publishing)
uses VCS semantic-version tags and discovery by Go tooling; published versions
must not be rewritten. [Module reference](https://go.dev/ref/mod) defines proxy/
checksum verification, module-path major versions and subdirectory tag prefixes.
Adapter owner must first finalize module path, generated/allowed glue provenance,
actual framework/native qualification and source closure. No module name or tag
is declared ready here. Module publication has no RubyGems/PyPI-style OIDC upload
job: do not create a Go environment, registry prerequisite or fake trusted
publisher. Optional OIDC artifact attestation/OCI delivery is a distinct future
contract. No tag, proxy indexing, module release, token exchange or deployment
was performed/authorized. Existing frozen Go shipping guards remain intact.

`scripts/ci/workflows.json` (humanymous.workflow-catalog/1) is the single machine
catalog for exact filenames, UI names, stable job keys, concurrency and uploaded
artifact identities. Category/title display syntax is `CI / ...`, `Docs / ...`,
`Release / ...`, `Legacy / ...`. Technical filenames/job keys are stable APIs,
not presentation labels. ci.yml/release.yml/pages.yml, reusable references,
registry release.yml trust and OIDC certificate identity stay unchanged. Current
kebab-case component filenames also stay stable: a ci-* rename offers little UI
benefit while migrating policy, paths, docs and external run consumers. Revisit
only as an explicit coordinated migration, not an incidental cosmetic change.

Catalog validator checks exact inventory, unique filename/display identity,
exact job sets, concurrency and upload names. Existing permission/required-gate,
release/tag and pin verification still run separately. Naming registration is
not authorization. The UI is normalized without changing job keys, artifact
prefixes/download contracts or resource-targeted concurrency cancellation.

New artifact contracts must include source SHA, run ID and run attempt plus
matrix tuple where needed, with all consumers/catalog updated atomically. Current
legacy SHA-only and digest/attack names remain explicitly grandfathered stable
contracts, not attempt-unique uploads; this is unresolved migration debt. Free
branch upload is off, so no new run-attempt safety claim derives from renaming.
observation/2 exact ID/source/attempt/time checks remain necessary for retrieval.

Static naming verification:9policy units pass (16unrelated filtered), live local
policy command and actionlint pass. No local runtime or hosted qualification is
implied. First failed run keeps its old display names at immutable bfc9f623;
new display names and catalog require their own next hosted source result.

### Product browser image source (not_run)

Product browser image source delta (later than63f69ae, not_run):
`build/rust/browser-product.Dockerfile` and its strict context allowlist reuse
the pinned compiler/geckodriver/WBG tool versions from the collector lab.
The WASM compiler enables collector runtime-pilot; the independent Alpine stage
builds the static runtime-pilot Rust driver. Final image copies exactly one
collector WASM artifact, reviewed webdriver JSON and the driver, runs UID65532
and enters --browser-driver. Wildcard ambiguity/missing output fails Docker COPY.
The original collector component image and denominator remain unchanged.
Owner Compose supplies namespace-isolated --browser-gate and owned nonsecret
endpoint/DNS arguments; witness credentials remain on private RPC channels.
Its browser/HTTP/witness assertions are owned by the lab producer.
All compilation/image size/14GB headroom/browser/sandbox/runtime connection
checks remain not_run. No local Docker build or browser execution was performed.

### Artifact-free diagnostics

Candidate preflight regression and correction: the original substring check
`contains("go build")` also matched `cargo build`. Actual static candidate CLI
exposed this after47a6720; it was not an image-build/runtime failure. Shared
docker_source token detection now separates cargo from go/go.exe/absolute Go
commands, handles intervening flags and ignores full-line comments. Original
Go compiler/base-image refusal and legacy source paths remain mandatory.
Allocated isolated target validation passed38pureunits/strictClippy plus actual
staticcandidate/policy commands; no image/runtime execution. Preserve the earlier
failed preflight rather than relabeling it as successful or weakening Go guard.

Approved hosted connection delta after the original63f69ae run: laneB now runs
docker→browser(component)→browser-product→nginx→rust-images(candidate)→images
(legacyshippingguard)→packages→node→advisories. Every new job uses standard
Ubuntu24.04, direct policy-success dependency, bounded timeout and no artifact/
cache writes. LaneA remains independent; all new jobs join strict ready/catalog.
Separate fresh VMs avoid retaining both component/product browser build layers
within one14GB runner. No unsupported step is treated as passed/skipped success.

Nginx consumer uses the owner's single adapters/nginx/pilot-contract.json.
Fixed Rust Docker argv supplies source revision/working manifest and a reserved
unique project/image. Capture build/run privately, verify image identity before/
after and own cleanup even on failure, reject source drift. Process exit0 plus
exact unique35scenario groups, source/lock/module/engine identities and zero
failed/ignored/not_run qualify only this component. A failed process with no
report retains unknown counts, never fabricated35passes or zeroerrors. Scenario
groups are not Rust harness tests or a human/full-product claim. No contract
string is executed, no generic container/resource cleanup is used.

Runtime image candidate has its own Dockerfile/strict context and typed inventory
at build/rust/image-candidates.json. It builds the actual humanymous-runtime
binary with pinned Rust/musl compiler, scratch/nonroot, private inherited pipes
and no baked secrets/state/default listener. Owner confirms main has no external
witness broker injection and requires supplied private custody/WAL state.
role runtime-private-pipes is not core/gate parity; all qualification flagsfalse,
qualified_platforms[] remain. A separate build/Trivy candidate job produces
future actual build/CVE evidence; original Go source guard and shipping mapping
stay unchanged. No publish/push image/tag/token exchange is enabled.

Root allocated the local Cargo slot:37pure delivery unit tests and strict scoped
Clippy passed with jobs1/incremental0, staticpolicy/actionlint passed; slot was
returned immediately. No new local actual process/container/browser ran. All
new hosted connections/candidate builds/cleanup behavior remainnot_run until a
new root-controlled source closure/run. Current63f69ae evidence is separate.

Component summary adds scoped assertion total/passed/failed/other and required-
unavailable count from the owning report, preserving existing schema/status/
qualification fields and explicit full-product flag. Missing case arrays yield
null rather than inventedzero. Assertions are not Rust test counts; never print
case detail, actor identity or raw process streams. This corrects the second
hosted source's missing scoped denominator without guessing old lab counts.

Artifact-off diagnostics correction: workspace probe emits only the owning
formatter's sanitized accounting to stdout/GITHUB_STEP_SUMMARY before evaluating
failure, using the same summary writer as pilot. Never print raw owner reports
or actor transcripts. This addresses source63f69ae's Ubuntu native failure whose
public log had no harness detail; the original exact failure remains undiagnosed.
Common-lock coherence failure initially prevented local checks; after root repair
27pureunits/strictClippy passed. Actual hosted emission remains not_run for this
later source and is not inferred from the earlier policy success.

### Product OIDC environment preparation (no deployment)

RubyGems/PyPI preparation extension, explicitly requested after crates/npm:
registry-prerequisites adds rubygems/pypi with exact same-repository release.yml
identity and product environment. Package names remainnull pending adapter owner
metadata; package_ready/publisher_configured/verified/direct_publish_allowed all
remainfalse. Catalog planned_publishers is distinct from actual job environments:
both statepreparation-only/job_presentfalse. Rust policy checks exact bindings and
rejects premature readiness/publisher assertions or a registry publish job.
No new job, action, id-token permission, long-lived secret or token request added.
Readiness includes all4registries, so pending products remain shipping blockers.

Official procedure checked2026-11-07: [RubyGems trusted publishing](https://guides.rubygems.org/trusted-publishing/)
supports existing-gem owner registration and account pending publisher for a new
gem, using owner/repository/workflow and selected environment. A successful first
push converts pending to normal; it was not performed. Same-repository workflow
identity is retained, no cross-repository reusable publisher assumption.
[PyPI existing publisher](https://docs.pypi.org/trusted-publishers/adding-a-publisher/)
binds owner/repository/workflow filename and environment; our contract requires
the pypi environment although the service makes it optional.
[PyPI pending publisher](https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/)
can create a project on first authorized upload but does not reserve its name.
External account ownership, final package names, built native distributions,
publisher registration/evidence and token exchange all remain unverified/not_run.
GitHub environment preparation by root is not external registry registration.

New preparation contract29pureunits, strictClippy, staticpolicy/actionlint pass;
no actual package or token publication test. Adapter/package source is owned by
the adapter team, not implemented through extra authored Ruby/Python CI scripts.

The subsequent explicit user instruction separates registry products:
release.yml crates job uses environment `crates`; npm job uses `npm`.
Registry prerequisites and trust validator require the exact repository,
release.yml filename and product environment (`crates_io`→`crates`, `npm`→`npm`).
Catalog records all job environment names and rejects drift, including the
github-pages deployment name and existing image/GitHub-release `release` names.
Existing job OIDC permission scopes and tag-only release triggers are unchanged.
No tag, release, npm/crates publish or actual OIDC issuance test is authorized.

Root reports actual GitHub npm/crates environment creation/readback matching
modootoday/specfold: required reviewer myso-kr/1237913, prevent-self-reviewfalse,
admin-bypasstrue, custom branch policies with only tagv*, deployments0; the full
normalized parity receipt is in root private evidence. This provisioned GitHub
policy is distinct from external registry ownership/bootstrap/trusted-publisher
proof. Both verified flags and npm direct_publish_allowed remainfalse.

Exact product-binding negative tests, shared-release environment rejection,
catalog consistency and all27delivery pure units pass; strict scoped Clippy,
static policy and actionlint pass. No actual token or publication was attempted.

Accepted for implementation by the explicit parallel-completion request on
2026-11-07 KST. New authored implementation/tests use Rust1.99.1/edition2024.
The existing upstream action/Jekyll/theme/generated glue and frozen legacy
oracles are separate provenance; they do not authorize new authored script
implementations or passing script source through Rust subprocess arguments.

The contextual package `scripts/ci/delivery`/humanymous-delivery owns workflow
policy, source commitments, package graph/surfaces, registry preflight, evidence
registration, same-run observation and integration of owning pilot runners.
Its root membership/common Cargo lock delta is owned by root. A temporary local
workspace/lock is pre-integration evidence only, not a second committed authority.
Legacy delivery.py/check_action_pins.py/test_delivery.py are replaced after Rust
boundary/parity checks; Windows actor migration belongs to the Windows owner.

### Data and execution contracts

Actions observation schema `humanymous.actions-observation/2` binds the exact
run ID, source SHA and positive attempt number. Each attempt has its own
`attempt-N` directory. Check the terminal marker before changing terminal
metadata; fetch logs with the explicit attempt. Artifact capture uses exact
artifact IDs and requires matching run/source and creation within the attempt's
UTC start/end window. Incomplete inventory pagination is an error. Preserve
older observation/1 evidence without rewriting it as a later attempt's result.

Source snapshot schema `humanymous.delivery-source/1`: source_sha, dirty,
manifest_sha256, files[{path,sha256,bytes}] or explicit deleted state. Hash actual
working bytes without newline normalization, reject symlinks/path traversal,
and compare before/after execution. A clean HEAD alone is not a byte commitment.
Authored Rust/workflow files must be LF; compare Git stored bytes separately at
checkpoint rather than substituting a clean-filtered blob for executed bytes.

Pilot aggregate schema1 has kind native/browser/docker, source_sha, source,
documentation_review_sha256, status passed/blocked/failed, qualification
full-product/component-only, errors/skips integers, cases[{id,status,evidence,
detail}], components[], source_changed_during_execution, isolation, runtime,
images and private_evidence. Mandatory case IDs remain those in Rust
pilot::required and scripts/ci/INTEGRATION.md. An error, unknown exit, missing
report/case/evidence, changed source, stale doc hash or component-only result
fails full acceptance. Image-backed full evidence needs immutable sha256 IDs.
Blocked is neither a test skip nor a product success. Exit1 rejects full gate;
owning scoped runner exit2 is retained as blocked, never rewritten to zero.
Partial reports retain the owning skip counter; an absent counter is null,
never manufactured zero. Full acceptance still requires integer zero.

Accepted owner interfaces (implementation/actual qualification remains separate):

- Windows/native owner: Rust scripts/ci/windows-native-runner, `native --report
  ABS --binaries ABS` on Windows/Linux/macOS. Windows standard-user lifecycle and
  all-harness actor mode are separate. Actual OS token/UID and subprocess
  artifact hashes/counts belong to that owner; elevated actor is rejected.
  Hosted Windows pilot selects the owner's guarded standard-user mode; local
  validation never provisions an account. The four-package native mode is not
  an all-workspace actor replacement. A shared-owner workspace/probe extension
  is requested; until delivered, the old all-harness Python/PS probe is explicit
  legacy debt and no full Rust-only CI completion is claimed.
  The owner subsequently implemented workspace/standard-user-workspace with
  schema humanymous.workspace-probe/1, default-feature all-target discovery,
  exact native tuple and strict two known subprocess helpers. The delivery
  consumer now calls it through `probe OS ARCH` and uses the owner's privacy
  formatter; generic Python/PS probe calls are removed from both workflows.
  Producer summary compatibility and exact-source native/hosted execution are
  pending. Existing prototype files remain read-only until their parity evidence.
- Compose owner: Rust test/lab/runner, `run --evidence EXISTING_PRIVATE_BUNDLE
  --require-full [--collector]`. lab-report.json schema humanymous.product-lab/2,
  immutable source-manifest.json and resolved topology/image/inspect/transcript
  files. Bounded runtime origin/audit and Firefox component results do not
  qualify full Gate, TLS/H2, Pass, human/keyboard/AT, witness or provider cases.
- Pages owner: humanymous-pages `validate --source docs --destination ABS
  --baseurl VALUE`, apex plus /humanymous. Ruby/Jekyll/theme checker remain
  pinned upstream programs; no authored shell/Python validator is required.
- Runtime owner: actual binary humanymous-runtime, private inherited stdin JSON
  and sealed custody. humanymous-server/humanymous-gate are not delivered aliases.
- Node owner: humanymous-node-package build/inspect generates only upstream
  loaders/types through pinned @napi-rs/cli. Legacy JS test success cannot qualify
  native npm publication; actual Node/Bun/authority Docker receipts plus native
  artifact/source/provenance commitments are required. Exact final receipt
  schema remains an integration dependency, not an invented passing fixture.

### Registry and supply-chain boundary

Publication stays tag-only, protected-environment and job-scoped OIDC. All
publish=false manifests stay unchanged. Exact tag/source/version/main ancestry,
complete original responsibility inventory, CI-visible parity evidence, clean
shipping surface and fresh docs remain mandatory. Trust metadata must bind
repository/workflow/environment plus an existing canonical evidence path and
its SHA256. npm direct publish permission must be explicit. External ownership,
initial registration and controlled live OIDC remain unverified; no token fallback.

Validated Cargo archive schema humanymous.crate-archives/1 binds source SHA,
working manifest, Cargo lock, reviewed-doc digest and each name/version/filename/
archive SHA256. Release downloads that same-run artifact. An existing version
may resume only after exact registry crate/version/checksum matches the validated
archive bytes; 404 means not published, other network/auth/status errors fail.
Before a new upload normal Cargo package verification reproduces the archive and
must match its prior digest. No --no-verify, forced overwrite or blind skip is
introduced. Live account/partial-publication testing is still unverified.

Node receipt schema humanymous.node-native-qualification/1 at
target/node-native/qualification.json was accepted with the Node owner. It binds
source/package version/manifest/provenance, real Node/Bun host counts and actual
authority artifact, production-application-http verdict denominators and origin
effects, source-bound authority Docker image IDs and exact advertised qualified
tuples. Synthetic-evidence ALLOW cannot qualify production browser evidence.
Validated Node archive proof humanymous.node-archive/1 binds its receipt and tgz
SHA; publish downloads and verifies that archive instead of rebuilding it.
Current native receipts/qualified tuples are absent; no placeholder was authored.

Image-publication schema humanymous.image-publication/1 requires downloaded
current-run defender digests, version/latest identity equality, linux/amd64 and
linux/arm64, exact source labels/Git provenance, nonempty per-platform SPDX and
cosign verification for this repository's exact release.yml tag identity plus
GitHub OIDC issuer. Git context is pinned to the exact commit. Image signatures,
SBOM and provenance are retained; registry publication tests remain unperformed.

Target CI pilot artifacts are sanitized summaries: no raw actor SID, credential,
cookie/body or inspect/trace payload is copied from private owning reports.
Their private report SHA binds the full execution evidence retained in .spec.local.

Cargo audit reports preserve database revision/time, lockfile digest, findings
and maintenance warnings for the complete maintained lock closure. Image scans,
Ruby and npm remain distinct. Root Trivy findings in frozen Go are retained as
baseline provenance, not a Go maintenance backlog or a clean Rust closure claim.
No ignored advisory, missing scan, stale success or skipped required e2e waives
final publication. Rust Docker workspace context must include all maintained
member manifests/sources while excluding private evidence, credentials, caches
and compiled targets; private lab images never become public attacker releases.

### Same-handle observation

Run42339951733 at f32042af9136560b41c1765f565c2e04fe373a22 completed failure.
humanymous.actions-observation/1 preserves run_id/source_sha/terminal/status/
conclusion and every job's outcome. Terminal logs/artifacts are from that exact
handle. Four of16jobs passed (policy,Pages,Node,MSRV); native3 reached actual
build/tests/release build and then rejected absent runner. Other gate failures
are preserved. Later source/component success cannot replace this record.

# Rust delivery and Pages contracts

## Evidence byte portability

Normalize authored public text attachments to LF before Specfold attachment.
Bind index SHA/length to the actual Git bytes, not an uncommitted Windows CRLF
view. Before integration, compare each tracked public evidence file's unfiltered
working blob to the staged Git blob and run the public Specfold checker on those
same bytes. Binary attachments retain their bytes and disclosed scanner limits.
Historical CRLF evidence must retain its original private receipt; rebind only
the index when the normalized candidate equals the existing Git payload exactly.
Keep validated-worktree source hashes distinct from canonical-Git source hashes.
Never rewrite old source/runtime claims as though they were tested after normalization.

## Authored workflow and collector target

Workflow YAML is the single authored configuration; retire the unused Python
workflow generator rather than retaining two sources of workflow behavior.
Existing permission, pin and required-job checks remain mandatory. New executable
tooling is Rust; the remaining legacy delivery validators still require migration.
The browser job builds the actual humanymous-collector package with an explicit
wasm32-unknown-unknown target. Its subsequent full browser pilot remains required;
collector component tests or a successful WASM build cannot replace that gate.
Historical toolchain evidence naming the retired generator describes old source.

Pages pins ruby/setup-ruby b3aaceaef7e0ee231b499e7b5f49c03fc1d32199 and Ruby
3.3.13. The old action's builder inventory stops at 3.3.10 and cannot provision
the previously requested 3.3.12. The new immutable inventory includes 3.3.13,
the current Ruby3.3 security-maintenance patch listed by the official download
page when checked on 2026-11-07. Preserve frozen Gemfile.lock, Bundler2.5.22,
advisory checks, theme checks and apex/subpath builds. This compatibility update
does not establish a successful Pages build, deployment or vulnerability-free
Ruby runtime; exact-source Actions execution remains required.

Sources: [pinned builder inventory](https://raw.githubusercontent.com/ruby/setup-ruby/b3aaceaef7e0ee231b499e7b5f49c03fc1d32199/ruby-builder-versions.json),
[action entrypoint](https://raw.githubusercontent.com/ruby/setup-ruby/b3aaceaef7e0ee231b499e7b5f49c03fc1d32199/action.yml),
[Ruby supported downloads](https://www.ruby-lang.org/en/downloads/).

## Native CEF component integration

Full validation's Windows native leg uses the same complete standard-user probe
as the implemented-crate matrix, after the shared Rust CEF toolchain setup.
Compile the locked all-target workspace under the runner, enumerate every Cargo
test executable, then execute and verify every harness under the isolated
non-admin token with read-only workspace. Upload raw accounting even on failure.
Do not run CLI install tests as administrator or allow ordinary ignored tests.
The Unix native legs retain ordinary locked workspace tests. Native process
and release pilots still follow these checks and remain mandatory; the scoped
probe report never becomes a full native-delivery qualification. Reusing the
existing legacy probe adds no new Python/PowerShell implementation; replacing
its orchestration with Rust remains a tracked migration responsibility.

The f6d2893 full Windows native job uploaded an actual source-bound standard-user
report:51required/executed harnesses,214passed,raw2ignored custody helpers,
workspace write denied and temporary account/scratch cleanup confirmed. The job
then completed the release binary build and failed at the absent required native
pilot runner. These artifacts establish actor/harness/build repair only, not full
native pilot or whole CI success. Preserve run42335890776 failure and its logs.

The later cb5dce7 native run42332170496 failed Linux x64 on custody reacquisition
while its other5tuples and browser component passed. Preserve that failure; a
successful retry is not its replacement. A controlled duplicate-description
regression now reproduces the lifetime defect and the new custody/WAL common
guard plus Unix Clippy repair are locally validated. Exact patched-source native
CI is still required. No release gate or standard-user condition is relaxed.

The actual ac560ff source subsequently passed all6native tuples and the Windows
CEF build/provisioned rejection component. Downloaded source-bound receipts are
registered in evidence:root/20261106235519/cef-remote-results.json. Full delivery
run42329288896 remains failed with separately recorded product and wiring defects;
these component passes do not waive any release gate.

The collector component now also executes passive behavior/lifecycle tests and
unchanged existing JS reducer oracles from a test-only Docker image. Workflow
path coverage includes those oracle files. Release WASM excludes cfg(test) oracle
imports. This exception is migration evidence, not a new foreign-language product
implementation; final oracle retirement remains mandatory.

The later advanced collector delta adds unchanged advanced.js as a third
test-only legacy oracle. Actual Firefox30 includes comparison of the shared
Advanced report against that original function on the owned browser; it does
not prove parity across all APIs/OSes or integrate a qualified session. Production
WASM excludes every cfg(test) oracle import. The component lab512-task limit is
supported by retained256-limit pids.events/crash evidence, not sandbox disablement.

The root workspace now registers the Windows bootstrap and desktop CEF packages.
scripts/ci/cef-pilot is an independently locked Rust CI tool: build actual DLLs,
verify the exact pinned upstream manifest source, provision only generated Cargo
artifacts and execute the mandatory native-pilot-tests rejection case explicitly.
The default unit harness does not ignore this case; the provisioned case belongs
to a separate required component gate with exact one-pass/zero-ignore accounting.
No general ignore exception is added to the standard-user verifier.

cef-native-component.yml runs this Windows x64 component gate with read-only
permissions and pinned actions; it neither publishes nor replaces full native,
GUI/sandbox/IME/AX or Docker product qualification. Other native tuples remain
unqualified for the CEF product UI even if their platform stub compiles.

## Rule

This is the accepted delivery design, not a claim that the workflows, packages
or documentation rewrite are implemented. Replace manual verification with real
workflow/document-contract checks as those components land. The full Rust
migration remains the release prerequisite.

### One validation authority

A reusable unprivileged validation workflow is called by PR/main CI and the
tag release workflow. Every release runs it on its exact tag commit; a historical
green workflow or a skipped Docker job is insufficient. Its required jobs are:

- Linux, macOS and Windows locked workspace builds/tests and native executable
  startup/configuration pilots; use fail-fast false and unique artifact names.
- MSRV compatibility plus stable formatting and warnings-denied Clippy.
- The browser WASM build and real browser integration against Rust.
- Linux-only isolated Docker attack, Gate, Pass, swarm and deployment pilots,
  using Rust images. Preserve topology and human-baseline assertions.
- Fresh dependency/advisory scans, package-content/metadata checks, documentation
  review freshness and theme source/built-site validation.

Validation has contents read only. Permissions are granted separately to crates
OIDC publication, GHCR/signing, GitHub Release creation and Pages deployment.
Publication concurrency never cancels an in-flight release. PRs never publish.
The defense-only bots image is never a public package/image release target.

### crates.io publication

Use strict SemVer tag/version equality, main ancestry, reviewed changelog/docs,
validated package archives and the Cargo dependency graph before any upload.
Publish registry dependencies before dependents, including versioned dev
dependencies needed to execute packaged tests. Never ship repository-relative
test paths or unpublished required dependencies.

Publishable manifests require complete metadata, registry-compatible dependency
versions beside development paths, licenses/readmes and explicit package content.
Current publish=false declarations intentionally prevent premature publication.

Authenticate through the official crates.io OIDC action immediately before
publication, pinned to a verified commit. The reviewed v1.0.6 action commit is
3268eb413133e2960fa4ae0e963ca76380c54cd9; do not assume an example's moving v1
tag exists. Keep short-lived credentials confined to the publishing step and
allow the action's revocation cleanup. A long sequence reauthenticates between
dependency waves. No long-lived token fallback belongs in the release workflow.

The Trusted Publisher binds repository owner/name, exact workflow filename and
the protected `crates` environment. It is configured per crate after ownership
and initial registration. Current crates.io requires an initial non-OIDC upload
before that registration; this is an external account bootstrap prerequisite,
not a capability to fake in YAML. Do not perform it or claim live OIDC success
from packaging dry runs. User-owned account setup is recorded as unverified until
exercised; the subsequent automated release path must use OIDC.

On rerun, distinguish network/auth failures from a verified already-published
version. Check matching validated artifact identity before skipping an immutable
version. Registry publication is not an atomic multi-crate transaction; preserve
honest partial-failure records and resume safely without overwriting history.

Sources: [crates.io Trusted Publishing](https://github.com/rust-lang/crates.io/blob/main/svelte/src/routes/docs/trusted-publishing/%2Bpage.svelte),
[official authentication action](https://github.com/rust-lang/crates-io-auth-action).

### Authored Pages source

The site uses remote_theme, pinned to reviewed v1.5.0 commit
ba2b23f0e1639f92ccefbb882f43092bfe5b16b7. Keep title/repository, the existing
humanymous.net domain/CNAME, empty baseurl, GFM and explicit page front matter.
Default reader pages use the theme doc layout. Remove the overriding legacy
default layout, old generated prose/LLM twins and the docsgen build dependency.
Human/agent authors edit Markdown directly; CI validates rather than generating
documentation content. No scripted replacement prose or stale Go examples.

Use a locked, vulnerability-checked GitHub Pages-compatible Jekyll stack with
jekyll-remote-theme. Run the checker from the same theme version against source
and built HTML, including links/navigation, then inspect light/dark/mobile
renderings. Optional twins are omitted unless their source is authored and
maintained without the prohibited documentation generation flow.

Build and deployment are separate. The protected github-pages deployment needs
the checked site artifact from the same commit and receives only its required
Pages/OIDC permissions. Configure the repository's Pages source for Actions.
Do not publish a preview of an incomplete Rust replacement to the public site.

Sources: [theme install](https://github.com/modootoday/jekyll-theme-modootoday/blob/v1.5.0/docs/guides/install.md),
[Pages workflow requirements](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).

### Design token ownership

Port the baseline docs palette, typography, radius and measure through the
supported docs/_sass/modootoday/_site.scss bridge, using only its three override
maps. Do not shadow theme internals or copy the old component stylesheet. Map
semantic roles explicitly, preserving the brand palette; verify actual text
pairings before accepting a palette mapping.

| Baseline role | Theme role | Light | Dark |
| --- | --- | --- | --- |
| bg | background | #eef1f6 | #0a0e14 |
| text | foreground, card-foreground | #14202e | #e4ecf5 |
| panel | card | #fff | #111722 |
| panel-2 | muted, code-background | #f6f8fb | #0d131c |
| muted | muted-foreground | #566579 | #808fa3 |
| border | border | #d9e0ea | #202a38 |
| raise | accent | #fff | #16202c |
| accent | primary, brand, ring | #0e9c8a | #35d0ba |
| accent-ink | primary-foreground | #fff | #04120f |
| allow | tip, status-available | #1f9d43 | #3fb950 |
| challenge | warning, status-partial | #b3811f | #d9a441 |
| deny | caution | #d63a52 | #f0556a |
| info | note | #2b7fe0 | #4a9eff |

Map system font stacks to font-sans/font-mono, 820px content width to measure,
and 10px/14px corners to radius/radius-lg. The link/text-on-primary assignments
require contrast verification; adjust semantic use or a documented contrast
variant rather than silently assuming the old pairings are accessible.
Retire unused faint/border-soft/dim/spacing variants, or keep them as documented
product-owned properties only where a real site component still needs them.
Source: [supported token bridge](https://github.com/modootoday/jekyll-theme-modootoday/blob/v1.5.0/docs/guides/customize.md).

### Release documentation freshness

A read-only Rust verification command checks authored review evidence in .spec:
each shipped command/API/configuration surface and capability has a reader page,
governed source hashes, review notes and executable example/pilot coverage.
Changed governed source requires an explicit renewed documentation review; the
gate never repairs its own evidence or merely accepts any Markdown edit.
Unchanged prose may be retained after a concrete review confirms it still holds.
Run examples/config parsing against Rust and require the same-commit CI pilot
jobs. Missing owners, stale digests, broken navigation, stale Go instructions or
an unexecuted required check block crates/images/releases.

## Why

The user explicitly requested cross-OS builds, OIDC crates.io automation,
release-time documentation review, remote_theme-based Pages and token migration.
The design panel found current path-only/publish=false manifests, broad existing
release permissions, old layout overrides and registry bootstrap prerequisites.
Those findings are requirements above, not hidden implementation assumptions.
The adversarial review additionally forbids stale-SHA authorization, token
fallbacks, vacuous documentation gates and treating registry errors as success.

## Verification

### Workflow implementation boundary (2026-11-07 KST)

The reusable `.github/workflows/rust-validation.yml` now owns mandatory native
Linux/macOS/Windows, user-approved pinned 1.99.1 MSRV/toolchain format/Clippy, actual WASM/browser,
Linux Docker lab, defender image scans, packages, Node/Bun, advisory, authored
documentation review and Pages jobs. The aggregate explicitly rejects any
skipped/failed/cancelled required job. `ci.yml`, `release.yml` and Pages deployment
call this workflow at their own checkout commit. No historical run lookup grants
publication. Frozen Go CI remains a manual oracle and its original release YAML
is inert provenance under scripts/ci, never a shipping path.

`scripts/ci/README.md` fixes the integration interfaces and execution-report
requirements. Runtime reports bind exact source SHA and authored documentation
review SHA-256, require all named cases (including executable documentation
examples), and reject skips/errors. Docker evidence additionally requires Rust,
Docker isolation and immutable image identities. Runner code review and actual
execution remain required; a JSON report alone is not behavioral proof.

The isolated Rust tool at scripts/ci/docs-review is read-only. Its schema-1
docs/reviews.json interface requires exact SHA-256 source inventory, authored
review notes/pages, and fixed command/API/configuration/capability coverage with
named pilot cases. It cannot repair evidence, drop source paths through a
manifest exclusion, or accept a Markdown-only edit as source re-review. Pages
build validates both baseurls through the Pages owner's pinned-theme checker.

Release publication uses protected release environments and job-scoped OIDC.
The official crates authentication action uses the reviewed v1.0.6 commit above.
Dependency-first waves include versioned dev dependencies and reauthenticate
before each wave (maximum eight reviewed waves; a deeper graph blocks until
the workflow is extended). Registry errors and existing immutable versions fail;
there is no blind rerun skip or token fallback. Partial reruns require artifact
identity review; automated identity-aware resume is not implemented. npm uses
Node 24/npm 11.16.0 trusted publishing with provenance. New npm trust settings
must explicitly permit direct `npm publish` (the current default grants staged
publication only), bind release.yml/release, and establish package ownership.

Current registry prerequisites are explicitly unverified in
scripts/ci/registry-prerequisites.json. All library manifests remain publish=false;
Rust runtime binaries, WASM/browser integration, isolated full Rust lab, native
startup pilots are not delivered by workflow files. The Pages owner has authored
the initial source/capability review manifest; concurrent Rust source changes
correctly make individual digests stale until an explicit renewed review.
The canonical source/test inventory must mark every responsibility
verified_runtime_parity with existing Rust paths and CI-visible parity_evidence,
or retired_with_review with an authored disposition and the same evidence.
Pending/unit-only rows and tracked Go source/manifests block final cutover.
Their absence fails validation/readiness instead of skipping. Native release
archives and retained defender GHCR SBOM/provenance/signing machinery
are wired but not publication-tested. Cross-OS Actions execution and live OIDC
must still be demonstrated after these prerequisites land. No publish occurred.

Official interface checks: [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/),
[Cargo package](https://doc.rust-lang.org/cargo/commands/cargo-package.html),
[reusable workflows](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows).

Design evidence is the inspected repository plus primary sources above. Code
and live account setup are pending. Implementation acceptance requires
cross-OS/pilot evidence, negative tests for the read-only freshness verifier,
locked source/site checks and a controlled OIDC release demonstration after
the product migration and account bootstrap prerequisites are met.


## Native probe subprocess-helper accounting

The audit key_custody harness has two explicitly ignored subprocess entry points,
not omitted product tests. Native Windows coverage must retain the exact Cargo
executable hash and distinguish required parent tests from these helpers. Accept
only the registered child_lock_probe and child_interrupted_writer in the audit
package/test target, with both matching parent tests passed, the lock helper's
actual nested successful summary and no unknown ignored/filtered tests. Report
ignored_helpers separately and preserve raw ignored_tests; never relabel them as
zero or count the nested child summary as an additional parent test. Any unknown
helper, missing parent/child proof, malformed extra summary or ordinary ignored
unit fails the gate. All other harnesses retain strict one-summary/zero-ignore
accounting. This does not skip tests or qualify full product runtime.

## Bounded public archive transport and measurement

Run 42354873001 at a26d1c3f08b365ca93545c6a51343095868edb45 is a distinct
terminal startup_failure with zero jobs and zero artifacts. GitHub rejected the
caller because its implicit actions:none ceiling could not grant the reusable
policy job actions:read. The CI and release validate call jobs must explicitly
grant exactly contents:read and actions:read; workflow globals stay contents:read
and no write/OIDC permission is added. Static policy checks this caller ceiling
as well as the callee permission. The failed source must never be rerun or
reported as executed product coverage; a corrected new source needs a new run.

Terminal evidence capture distinguishes this exact zero-job startup failure
from execution failure. Only completed/startup_failure with zero jobs and zero
artifacts may record execution_log_status=not-created-workflow-startup and a
null log hash. Other terminal runs require execution jobs and captured logs;
missing logs never become a successful or complete execution claim. A startup
failure that unexpectedly has jobs/artifacts is rejected as inconsistent.

Public summaries use one compact JSON log record and flush stdout before
subsequent failure diagnostics. The GitHub step summary retains pretty JSON.
This removes the multiline boundary that split the observed receipt in merged
job logs. Remote transport atomicity is not assumed, and hosted verification of
the new emission remains required. No additional report fields are exposed and
the original private raw-byte digest remains separate.

The next native lane runs the maintenance process/storage/package component
tests independently after a successful workspace build, and Windows additionally
runs the existing CEF provisioning coordinator. Failure of either does not hide
the subsequent whole-workspace actor's required coverage accounting. The actor
still fails for missing Node runtime groups; these are never classified helpers.
The maintenance component denominator is eleven actual owner tests, not its
pure unit inventory. Its hosted execution remains pending for this new source.

CEF provisioning creates distinct fresh positive/cancelled bundles outside the
safe report directory, copies only pinned runtime inputs and invokes the desktop
native-pilot-tests library target. The browser owner's pilot-contract.json is
the sole name/count/schema authority; both required cases and exact two-test
summary must pass. Source hashes recursively include native/shared/scheme.rs,
browser source, test-support source and their manifests/toolchain/locks. A failed
desktop stage preserves an already measured bootstrap result without inventing
unknown desktop counts. Public artifacts contain only report.json, never runtime
bundles, profiles, native-report.json or captured process output. Full browser,
IME, accessibility and descendant cleanup qualification remain false/unknown.

The existing native-workspace artifact now retains only the owning formatter's
sanitized report.json for one day on main after a successful build, including
failed actor execution. Free-branch automatic uploads are disabled. Raw
actor/private reports are not included. Browser-product
build context permits only the additional exact Go configuration and contract
JSON files required by the lab owner's new compile-time runtime inputs; this is
source preparation, not execution or qualification of a Go profile.

The next Linux x64 hosted native job explicitly invokes capture's five ignored
hosted_transport cases and the native ABI generated-header conformance example.
Each invocation runs independently after the workspace build, preserves failure
and source/raw-log hashes before returning nonzero, and never runs locally.
Capture requires all five registered actual names, one exact five-pass summary,
zero ignored/filtered tests and an unchanged source. This is original TLS/H2
producer component coverage, not Runtime request binding or product acceptance.

Header generation installs locked cbindgen 0.29.4 into a fresh private tool root
and isolated build target outside the evidence bundle, records its observed
version/binary/header hashes and invokes the owning Rust header_conformance
example unchanged. The owner verifies C11/C++11 syntax and layouts; delivery
does not copy that parser or author C/C++ source. The thirteen expected compiler
evidence files remain private with public hashes and bounded command exits.
No header existence or pure unit count proves dynamic ABI/managed/allocator host
support. GNU timeout bounds these literal upstream executable invocations;
only clean exact-source standard hosted Linux x64 may execute them. Actual
capture/compiler runs remain required independently of source implementation.

Maintenance's independent command is checked through three complete harness
summaries with reviewed counts five/two/four, total eleven, zero failures,
ignored, measured and filtered tests. Exit zero without this denominator is
incomplete. Its source-bound public receipt contains only counts and log hashes.

Runtime independently discovers and executes its all-features library and
native_process targets before the generic actor. The owner-reviewed library
denominator is provisionally 45 on Unix and 42 on Windows, plus one explicitly
named native process case. Exact-source discovery must match those counts and
contain unique actual names; a mismatch fails rather than updating the contract
from an unknown green result. Each execution requires one complete zero-ignore,
zero-filter summary. Both harnesses are attempted independently even when one
fails. Missing discovery or failed execution retains unknown passed counts.
These are Runtime process/library component fixtures, not browser, human,
collector attestation, production ALLOW or full-product qualification.

Actual run 42355168729 at 8132117 reports a successful runtime-candidate build
and Trivy action, but the scanner reports zero language-specific files and an
all-dash target table whose legend means not scanned, not clean. This source
does not establish image CVE coverage. A green scanner exit cannot replace a
nonempty source/image-bound dependency inventory and its vulnerability result.
The next candidate must reject missing inventory explicitly and retain the raw
scanner report hash; shipping-image CVE qualification remains unproven until
lock/SBOM/source identity and actual scan coverage are measured. The legacy
core/gate source guard and full-product block remain in force.

The image coverage gate prepares a clean exact-source manifest before building
and binds it into the candidate image label. Trivy 0.70.0 JSON, all packages and
all severities are required; explicit scanner failure remains failure. The
verified local image ID and scanned image metadata must agree with both source
labels. Only a nonempty rustbinary inventory for the shipped runtime entrypoint
is observed binary coverage. Cargo.lock-only results and empty result tables
remain not_scanned. Suppressed/modified findings, malformed records and absent
reports fail closed. The safe summary exposes counts, hashes and explicit
unknown completeness, never raw vulnerability descriptions or package payloads.
Observed-inventory cleanliness is not proof of complete dependency inventory,
shipping equivalence or whole-product readiness. Source references are Trivy's
v0.70.0 Rust coverage documentation and JSON report/types declarations.

Root checkpoint a26d1c3 resolves the common 36-member Cargo graph/lock and the
standalone scripts/ci/windows-native-runner/Cargo.lock against the shared native
APIs. The runner lock is under this delivery governing scope but remains an
explicit root-owned integration input. Locked metadata resolution proves graph
consistency only; strict native, advisory and hosted execution remain required.
The root staged check's D1 change-note warning is retained in the execution
record; this governing note documents that resolution rather than claiming a
new build or retroactively changing its check output.

The later explicitly authorized cache pilot is an exception to the earlier
no-cache default, limited to checksum-verified public crates.io `.crate` archives.
It never restores compiled targets, Cargo source/index trees, Git dependencies,
private evidence, credentials, or trust decisions. The exact OS/architecture,
Rust 1.99.1 and committed root lock identify the transport key. Pull requests
restore only; only trusted repository pushes on main or codex/rust may save.
Read-only live repository quota, usage and retention checks must verify at most
10 GB configured capacity, seven days retention and at most 1 GiB selected
archives before a save. Unknown state falls back to uncached validation. No
quota change, remote compiler cache, gate waiver or measured cache hit is implied.

Cargo command observations use monotonic wall time and allowlisted command
kinds, success/exit accounting and hashed inputs. Raw arguments and timing HTML
remain private. Resolved features, effective configuration/linker, native ABI
and isolated dependency identities remain explicit unknown dimensions;
reuse_key_complete and linked_artifact_reuse_qualified remain false. Measurements
do not justify deleting builds until equivalent actual execution is established.

The sole lab public-receipts contract governs the CI projection. The current
required owned-deployment wargame ledger has exactly 26 ordered unique indexes.
CI validates source and raw/manifest digests, exact field allowlists, terminal
statuses, expectation nullability and independently recomputed totals/scoped
status. It preserves a valid receipt when surrounding full-product gates fail.
Missing receipts remain unknown; malformed present receipts fail the gate.
No raw body or actor identity enters the public projection, and scoped evidence
never becomes full-product qualification.

Independent Runtime and maintenance profiles consume their owning JSON
qualification inventories. Exact-source discovery must equal the complete
reviewed name set and execution must report the same nonzero count with no
failed, ignored, measured or filtered tests. All required targets are attempted
independently. No copied fixed count is the qualification authority. Contract
hashes accompany the bounded public counts; local parser tests do not establish
actual process, filesystem or hosted outcomes.

The Node host profile consumes the owning host-pilot contract and runs its
actual Node/Bun fixture independently of package qualification. Each container
has an invocation-specific name and ownership label. Cleanup removes only an
inspected exact image and invocation label, then requires a successful empty
exact-name inventory. Component evidence cannot create a native package trust
receipt or full-product qualification.

The bounded Docs / Validate entrypoint permits only the dedicated
codex/pages-validation push branch and workflow_dispatch. It calls the existing
build workflow at the event's immutable github.sha with contents-read permission,
without deployment or OIDC authority. Ruby 3.3.13, frozen Gemfile.lock, one bundle
job and Rust 1.99.1 are explicit inputs. Audit, apex and subpath attempts remain
independent after installation; their failures cannot be hidden by artifacts.
Before/after provenance binds clean docs and Rust builder/delivery input bytes,
docs Git tree, configuration and lock hashes. Trusted step outcomes are emitted
without parsing inherited mixed subprocess stdout; unknown exits remain null.
Only successful corresponding generated sites with unchanged source on main
are eligible for one-day uploads; free branch uploads are disabled. Site archive
usability additionally requires the owned packaging/roundtrip check. Safe
source/outcome JSON remains in allowlisted job logs after build failures. Browser,
assistive-technology and whole-product qualification remain false.

The dedicated caller first runs the exact-source delivery unit suite,
all-target Clippy with warnings denied, and workflow authorization policy on
standard hosted Ubuntu 24.04. All preflight commands are independently attempted
after toolchain setup; any failure blocks the dependent Pages build. This
hosted preflight may establish new-helper validation when local capacity is
unavailable, but authoring and static checks alone never establish its success.

The dedicated Go component caller is isolated to codex/go-validation pushes
and workflow_dispatch with read-only permissions. Exact-source Ubuntu preflight
runs Go owner units, all lab owner units including shared DNS, delivery receipt
units, selected all-target Clippy and workflow policy before actual execution.
CI invokes only the owning lab's --go-product --require-go-component profile in a fresh
private bundle. Its sole go-public-receipts.json vocabulary governs the eight
ordered case IDs, assertion knowledge, safe errors, totals and public fields.
CI independently hashes the original private Go ledger and source manifest,
checks this run's metadata and recomputes public accounting without redoing
HTTP semantics. Unattempted cases are known zero assertions; attempted but
unmeasured cases retain null. Only the safe projection and hashes enter job logs;
automatic branch uploads remain disabled. The owning cleanup case must be
observed and the lab's typed go-component requirement must be satisfied with
exit zero. The lab owns all bounded success predicates. Its existing
--require-full mode remains a mandatory separate final gate with nonzero exit
until full product readiness; exit two is never accepted as component success.
Component qualification and environment metadata are not authenticated provenance.

The existing CEF runner consumes protocol_preflight from the owning desktop
contract before any native pilot process: first compile the native-feature
library test binary without execution, then run the exact synthetic selector
and reviewed names. Its parser-fixture counts and actual filtered totals stay
separate from bootstrap rejection and the two actual desktop process cases.
Unknown process counts remain null. The runner directly rejects any ACT
presence even when hosted environment labels are emulated. Such environment
admission is not remote attestation, and synthetic fixtures cannot qualify GUI,
keyboard, IME, assistive technology or descendant process cleanup.

Public Pages archives are produced only by the owning Rust packager, whose
allowlist includes .well-known/security.txt and excludes source/private/special
paths. Each base variant must pass package and a second fresh archive-check
before its safe manifest/archive hashes are observed. Preparation creates fresh
dedicated output parents; restores use distinct fresh destinations. The main
serving artifact contains the verified artifact.tar itself, avoiding the old
upload action's hidden-file filtering. Main additionally downloads that exact
artifact and runs the owner archive-check against its original manifest before
the build may release deployment readiness. Final source/transfer outcome is
written before the terminal outcomes artifact. Branch uploads remain disabled.
Observed archive integrity is distinct from browser/AT and full-product proof.

The dedicated browser-product diagnostic caller is restricted to
codex/browser-product-validation pushes and manual dispatch on Ubuntu 24.04.
Exact-source owner and delivery units, Unix runtime actor compilation, strict
selected Clippy and workflow policy precede its separate product job. It invokes
the existing delivery pilot browser-product path, including the owning lab's
--browser-product --require-full requirement. Owner exits one and two remain
failures; no diagnostic exception or component success contract is introduced.
Public output reuses the existing source/image hashes, assertion accounting and
sole-allowlist startup stage projection. Raw process output stays private.
Read-only permissions, pinned actions, a bounded job deadline and disabled
automatic artifacts/cache writes apply. This lane does not establish human,
assistive-technology, deployment or full-product readiness from partial results.

The runtime image candidate embeds the actual linked Rust dependency inventory
using cargo-auditable 0.7.7 in its compiler stage. The tool installs with its
packaged lock into /opt/cargo-auditable, and only the build command extends PATH.
runtime-build-tool.json records the original registry archive, packaged lock
hash, upstream source revision and all 47 registry archive checksums in that
lock, including development/other-platform packages. This is the locked tool
closure, not a claim that every package is linked into the tool or runtime.
The metadata encoder records package names, versions, dependency relationships
and source categories while redacting source URLs and filesystem paths. License
metadata and a snapshot-bound advisory check remain separate tool provenance;
they do not establish a current runtime image scan. The runtime source/lock,
scratch image, nonroot UID, entrypoint and source labels remain unchanged.
Installation, auditable build and Trivy binary inventory require actual evidence
before any qualification is asserted. Existing shipping guards stay fail closed.

The dedicated native component caller accepts only capture and header, with
capture as the dedicated branch push default. Manual selection is a closed
choice passed through a quoted environment variable into the Rust
native-component selector, which rejects all other targets before resources.
The selector reuses the existing component owner invocations and receipt checks.
Read-only exact-source Ubuntu preflight runs delivery units, selected owner
all-target checks/Clippy and workflow policy before a bounded component job.
Capture must execute all five hosted transport cases without ignored/filtered
counts; header uses pinned cbindgen and the owning C11/C++11 layout verifier.
Neither profile qualifies dynamic ABI, product e2e or browser/AT behavior.
Automatic uploads/cache writes, publication and OIDC are absent.

The standalone CEF runner requests Cargo JSON for its native build and emits
a closed safe summary on terminal success or failure. Only Rust E-number codes,
reviewed lint labels and primary file/line locations matching its canonical
repository source inventory may leave the private log. Rendered diagnostics,
source snippets, messages and arbitrary paths are excluded. Diagnostic reads
are bounded to eight MiB, records to 256 KiB and output to 64 diagnostics with
four locations each. Malformed, oversized, omitted, missing and truncated data
remain explicit; unsupported build-script/linker failures retain unknown cause.
The observed private prefix and terminal private report are hashed. A full log
hash is absent when the read is truncated. Raw files remain private; this read
bound does not impose a new bound on temporary on-disk compiler log growth.
Source drift is null when the existing final source check was not reached.
Protocol fixtures, bootstrap rejection and actual process cases stay separate,
and no diagnostic implies browser, GUI, AT or whole-runtime qualification.

Subsequent protocol/bootstrap/process Cargo commands use the same bounded
diagnostic projection and retain whether an actual command status was observed.
Bootstrap execution counts require one exact expected case line, one consistent
harness footer and a matching command exit. Missing, duplicated, ignored,
truncated or inconsistent receipts keep execution counts null. Compiler errors
are retained ahead of warning saturation within the fixed output limit.
Captured bootstrap output uses --show-output to preserve the exact named result
line. Nested bootstrap child facts belong to the owning fixed-case contract;
the runner never infers them from a cargo exit or assertion message.

The CEF runner consumes the existing owner bootstrap_rejection contract's four
fixed case IDs, exact fields, nullable observations, error enums and 1024-byte
marker limit. It reconstructs only typed known fields and keeps missing records
null. Invalid or duplicate markers reject the diagnostic projection without
echoing payloads. A complete marker inventory is required alongside the unchanged
actual bootstrap test receipt; elapsed deadline observations introduce no new
refusal predicate. The owner's three synthetic marker protocol cases run as a
separate selected preflight before the actual bootstrap case. Their counts are
never combined with desktop protocol4, actual bootstrap1 or desktop process2.

Compiler/marker JSON and the embedded contract use the existing shared bounded
strict decoder. Duplicate decoded object keys, including escaped spellings,
are rejected before field validation. The standalone runner adds only the
existing test-support path dependency and its already-locked serde derive edge;
no external dependency version or checksum changes are required.

The dedicated mount diagnostic consumes only the lab's mount-probe contract and
existing scoped CLI. CI independently binds this hosted run, source manifest,
original private ledger, product recipe and observed image inspection. It checks
closed fields, ordered role/surface/path identities and public count consistency
without evaluating private UID, mode or custody predicates. Four complete
observations, including observed refusals, may establish collection completion
only. Owner exit zero, satisfied scoped requirement, zero infrastructure errors,
unique successful owned cleanup and unchanged source are mandatory. Custody
policy, whole product and release qualification remain false in every outcome.
Missing/invalid observations keep collection incomplete. The original null-image
receipt remains valid when no image or observation exists. The read-only bounded
Ubuntu caller has no automatic uploads, remote cache writes or OIDC.

The dedicated image caller uses read-only exact-source Ubuntu preflight and
the existing runtime image build/scan steps. Policy requires its build/scan
suffix to equal the full validation job, preserving the same typed Rust gate.
Both paths forbid pushing images, require a local loaded candidate and retain
all-package/all-severity Trivy JSON with no scanner or inventory waiver. Failure
projection still runs after a failed scanner when the image build succeeded.
The caller adds no scanner parser, package publication or qualification state.
Automatic artifacts, remote cache writes and OIDC remain disabled. Actual tool
installation, binary inventory and scan evidence require a separately admitted
hosted execution; source/helper success alone leaves the candidate unqualified.

The future tag-only release image matrix has exactly one runtime identity,
ghcr.io/modootoday/humanymous-runtime, built from the existing Rust candidate
Dockerfile for linux/amd64 and linux/arm64. This preparation does not make it a
core/gate compatibility alias. Readiness and a repeated pre-login image guard
require retirement of old core/gate shipping inputs and reviewed source-bound
build, nonroot, CVE, platform and full-product qualification. Current candidate
flags remain false, and the current source validator refuses fabricated true
flags. Promotion therefore needs an explicit reviewed implementation change and
actual receipts, not an inventory boolean edit. Max provenance, SBOM, cosign,
exact source/digest/platform verification and tag-only authorization remain.
The source manifest build argument uses the existing image-scan preparation.

Current CodeQL analyzes Rust with build-mode none using the existing pinned
action e4fba868fa4b1b91e1fdab776edc8cfbe6e9fb81 (4.37.3, bundled CLI2.26.1;
its builtin language inventory includes rust). Go setup/autobuild and old Go-only
exclusions are not part of this profile. Static analysis grants no runtime or
publication qualification. The manual historical Go oracle checks out only
981d25cf71a5a54855d7e2b4359e832a9d5998e5, uses that baseline's Go1.25.3,
and has no automatic artifact upload or remote cache write. Current Rust agent
maintenance belongs to current Rust validation, never the historical baseline.
No final Rust shipping gate depends on successful current-tree Go compilation.

Known adapted-source notices are distribution inputs. The audit sort's exact
crates/audit/LICENSE-Go.txt bytes and SHA256 are listed in the bounded Rust
distribution notice inventory. Native component archives include these bytes at
licenses/Go.txt alongside the root LICENSE, and record the inventory in their
source-bound manifest. The runtime candidate includes the same file at
/usr/share/licenses/humanymous/Go.txt. Source validation rejects a missing COPY,
changed notice hash or inventory identity; an in-memory archive regression
checks the actual packaged notice bytes. This addresses one known adapted Go
notice, not complete dependency license compliance. Actual final image contents
still need source-bound image execution; no publication is inferred.

The standalone docs-review tool declares its Apache-2.0 license and forbids
unsafe_code explicitly because its independent workspace does not inherit root
package metadata or lint settings. This metadata change adds no unsafe exception
and modifies no shared Cargo manifest or lockfile.

The native archive preparation additionally rereads the completed gzip/tar and
requires exactly one bounded regular licenses/Go.txt entry with the known
source hash; missing, duplicate, symlink, oversized or changed entries fail.
This verifier is preparation until an actual archive with the real CLI/runtime
binaries has been built and inspected at the same source.
Its scope is the known notice in the first tar/gzip stream of that owned output;
gzip footer CRC/length is consumed, but trailing or concatenated streams and
general safe extraction are not qualified by this verifier.

Final image notice extraction must bind the exact inspected image ID, use only
an owned stopped container and never start its entrypoint. A unique private
destination must reject missing, symlink, nonregular, oversized or changed data.
Create failure may still leave an owned daemon container, so cleanup requires
independent exact owner-label/image/stopped-state verification before removal;
never remove an unverified name. Primary copy/check failure and cleanup outcome
are separate, and missing observations stay unknown. Public output permits only
fixed status, hash and size. Current shared capture_with_stderr is not a hard
cleanup-deadline or joined-reader proof: stop uses blocking wait and reader
handles are not retained. No extraction implementation may infer reap/join or
successful cleanup from Drop or a deadline label. Actual process wiring awaits
an accepted supervision contract and the root-owned dependency lock handshake.
