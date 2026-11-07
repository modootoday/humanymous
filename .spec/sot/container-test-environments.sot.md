---
id: sot:root/container-test-environments
title: Canonical Compose lab and Cloudflare test environment profiles
status: active
created: 2026-11-06T22:41:55Z
updated: 2026-11-07T09:54:33Z
verify: manual
rel:
  governs: [deployments/lab/**, build/rust/**, test/lab/**]
  depends_on: [sot:root/cross-os-runners, sot:root/adapter-ecosystem, sot:root/automation-evidence]
---

## Rust collector component lab delta c3389b6

Collector advanced probes use a512-task cgroup budget. At256, the isolated
full advanced export failed with thread-spawn EAGAIN/content-process SIGSEGV and
the same container's pids.events recorded max2; media/capability/profile alone
passed with max0. Preserve those failed receipts. Raising this finite resource
budget changes no UID, read-only mount, network-none, cap-drop, no-new-privileges
or browser sandbox setting. A successful retry must record its own exact image,
compose source and executed test count; it is not retroactive success at256.

compose.collector.yaml and collector-browser.Dockerfile execute actual Rust WASM
component tests in Firefox ESR with pinned wasm-bindgen/geckodriver tooling.
The lab inherits read-only root, UID65532, cap-drop ALL, no-new-privileges and
network none, with bounded tmpfs and loopback-only in-container test transport.
Native6/browser3 pass in the source manifest recorded by
evidence:root/20261106235519/collector-results.json. The test runner uses Firefox
because its Chromium/Edge paths append no-sandbox. No disabling flag is authored.
Firefox reports user namespace creation EPERM; complete browser sandbox policy
is unqualified. Headless/WebDriver preferences are interventions, not a human
baseline. This component lab does not qualify full product e2e or Meari+CEF.

# Container product-test environments

## Local Docker engine affinity

Follow the canonical Local Docker engine identity and cleanup section in
sot:root/rust-migration and .agents/rules/96-docker-engine-affinity.md. Default
local execution uses Windows Docker Desktop's desktop-linux context with the
installed Windows docker.exe path; missing PATH/binary is not permission for
WSL CLI/daemon fallback. WSL-native Rust compilation is a separate profile.

Before local build, run and cleanup, independently select the Desktop binding,
inspect context endpoint, server ID/version and OS type, and retain observations.
Refuse missing/mismatched identity before mutating resources. The expected
Windows endpoint is npipe:////./pipe/dockerDesktopLinuxEngine and container OS
Linux. The server ID is an observed execution binding, not a universal constant.
Windows Desktop29.9.2 and standalone WSL29.1.3 were different engines; their
images/caches/timings cannot be merged as one environment. Explicitly allocated
WSL/act clients must verify the same Desktop server ID and record their actual
proxy endpoint separately. act SDK admission requires explicit Desktop
DOCKER_HOST; CLI context selection alone is insufficient. Hosted Linux's own
engine and provenance remain unchanged. New local product qualification is
pending actual affinity preflight and measurement, not satisfied by source rules.

The lab wrapper's source consumes shared test-support verify_desktop and
verify_allocated_cli. Its independently supplied private baseline has exact
schema/context/endpoint/server_id/server_version/os_type fields; never infer the
expected binding from the daemon currently selected. Windows selects the fixed
installed CLI and desktop-linux on every command. Explicitly allocated local
Linux CLI proxies additionally require matching configured DOCKER_HOST and
actual queried server identity, while rejecting unallocated WSL fallback.
This is CLI evidence, not SDK evidence. Root/act must separately establish its
actual SDK connection. Save private expected/observed tuples and sanitized
comparison proof; observations are not authenticated remote provenance.

Perform preflight before commands, force local resource-boundary rechecks and
recheck before project/expiry cleanup. Refuse changed context/endpoint/server
ID/version/OS without deleting resources through a newly selected daemon.
Hosted Linux records its own distinct engine and checks continuity, without
requiring Desktop equivalence. Native/tooling and actual Docker qualification
remain NOT_RUN for this new source until root admits measurement. No workload
starts during the benchmark quiet window.

Root's authorized image prune is separate from owned project teardown: no lab
prune is performed here. Image-only cleanup never implies container, volume,
build-cache or system prune. Recheck the selected engine before any project
cleanup. Keep the benchmark quiet window until root allocates execution.

## Go bounded HTTP actual composition

The sole public projection uses humanymous.lab-go-public/1 and the
test/lab/runner/go-public-receipts.json field/status contract, emitted at
lab-report.json/public_receipts/go and go-public-receipt.json. Preserve all8
owner IDs/indices and terminal pass/fail/not_run statuses; recompute counts,
retain known evaluator assertions through teardown errors and mark unknown
assertions null. The original evaluator stdout is saved before drain. A cleanup
or drain error remains case failure even when semantic evaluation passed.
Only static Go-owned semantic codes or lab-owned infrastructure codes may leave
private records. No body/detail/configuration/actor observation/raw path is copied.
Bind exact source/manifest/original-private-ledger SHA256 and explicit run context.
Reject unknown status/schema, duplicate IDs, changed original bytes or false
qualification. Local act cannot receive an attributed real hosted run ID;
environment metadata remains conventional and requires independent actual
GitHub run/job/source/image verification. No signed attestation is claimed.

The --go-product profile consumes adapters/go/pilot-contract.json and the
Go-owned typed Deployment, scenario evaluator and Receipt qualifier. It uses
test/lab/go-configuration.json as the complete parent/actor configuration
template; owner routes and the measured fixed loopback contact-witness tuple
are the only per-case policy/endpoint changes. The existing private passphrase
is retained in startup only and replaced by a marker in configuration evidence.

The dedicated bounded invocation uses --go-product --require-go-component.
Its typed requirement is humanymous.lab-requirement/1, kind go-component.
Exit zero requires the validated eight-case hosted public receipt, no failed
lab assertions and successful final owned cleanup. The requirement and receipt
always retain full_product_qualified=false. This follows roadmap decision
8f8f5e07 and supersedes an always-red standalone full-product invocation.
--require-full remains a separate mutually exclusive full-product requirement;
bounded Go success cannot satisfy it. Other profiles refuse the Go requirement.

The fixture is untouched upstream Go source, never authored Go implementation.
Verify the pinned revision, full tracked byte inventory, entrypoint, compiler
index/platform manifest and actual version before a network-none build. Exclude
Git metadata and untracked bytes from the compiler context. Hash the exported
artifact and the independently copied actual container binary. Backend, Gate
and attacker use different namespaces; Gate alone joins two owned internal
bridges. The upstream binds loopback18081 behind a fixed bounded relay18082;
the Gate's independent contact relay and actual ServiceHost bind loopback.
Independent cryptographic witness stays on separate UID/storage/private pipes.

Every case starts with fresh writer/witness state and one real synthetic native
collection record, so even parser/kernel negatives have an independently pinned
audit head. Setup must make zero origin contacts. Attack traffic waits for actual
running namespace inspection and explicit private release. Actual origin-bypass
tests target Gate bridgeIP:contact-port and backend bridgeIP18082, not attacker
loopback. Timeout, missing counter or unobserved namespace is an error. Count
conservative TCP contacts separately from positive real-handler response proofs.
Preserve all eight cases, actual reasons and cleanup results; never retry effects
or manufacture zero counters. Complete successful ledgers alone may invoke the
owner qualifier against the parent's independently provisioned source/artifact,
configuration and OS tuple. New source is NOT_RUN until its own hosted receipt.

Docker build failures retain typed transport outcomes before returning errors.
For the owned Go runtime and canary build commands, public static diagnostics
distinguish deadline, nonzero exit and other capture errors. Deadline requires
io::ErrorKind::TimedOut; never infer it from elapsed time or arbitrary messages.
Retain argv/error details privately. If the shared capture API cannot return
partial output on failure, explicitly record that unavailability rather than
claiming an original compiler transcript was recovered.
Cold image compilation uses a separate bounded1200-second command budget;
ordinary Docker control remains600seconds and private-input commands30seconds.
Select the image budget only at explicit Compose build/buildx build call sites.
Actor, scenario, witness, runtime and evaluator deadlines are unchanged.

The --mount-probe profile is diagnostic observation, never product qualification.
First measure the unchanged empty-directory COPY recipe: custody-layout is the
exact ancestor of runtime and the only owner of that recipe. Build only this
small target. Observe fixed writer/witness roles both in the image and with
fresh project-owned named volumes, network none, nonroot, read-only, cap-drop
and no-new-privileges. No arbitrary role/path, foreign volume or privileged
initialization is allowed. One shared Rust predicate retains directory,
non-symlink, expected UID and mode&0777==0700 checks. Public refusal codes are
closed; actual UID/GID/mode/type/error facts remain private with source/image/
recipe binding. OBSERVED_REFUSAL is a complete diagnostic observation, never
a passing custody policy. Missing/invalid observations or failed cleanup exit
nonzero. Do not change the COPY recipe or policy before actual provenance is
measured. Preserve8 attempted actor failures and unknown assertions separately
from earlier8NOT_RUN build failures.
Metadata observation processes have their own30second capture budget. Bound
kernel status reads to65536bytes and reject overflow/invalid encoding. Parent
directory facts are private nonpolicy observations only; do not create a new
permission gate from them. Retain OS error numbers privately when available.

Hosted baseline42372825588/sourceb482d5be measured all four role/surface
observations and all eight paths as actor_mount_mode refusals, both in the
image and fresh named volumes. Lookup, directory, non-symlink and expected
UID predicates passed; exact numeric mode remains private. Collection and
cleanup succeeded, custody/full qualification did not. Evidence-backed repair
uses source0700 child directories inside /empty/writer and /empty/witness,
then copies each role parent tree with the existing role ownership. Preserve
child entries instead of relying on the metadata of an empty source root.
The runtime and probe share this single custody-layout ancestor and unchanged
predicate. Require the same actual four observations/eight policy boundaries
to accept before attempting Go8 or the browser product again.

Save the initial eight-row NOT_RUN inventory before any fetch/compiler/image
operation. Setup failures retain the unexecuted denominator. After a case,
independently inspect backend liveness, then await controlled actor exit and
the actual ManagedChild reap receipt before Compose cleanup; this does not
qualify graceful Go termination. Reuse the owned positive outside DNS/TCP canary
and deny-only DNS policy in each Go namespace. Require explicit TCP/UDP no-route,
actual libc-query denial, no default IPv4 route, disabled bridge IPv6 and no new
outside DNS arrival. A timeout or Internal configuration field alone is not proof.
Dependent images use the explicit service build context from Docker's official
https://docs.docker.com/compose/how-tos/dependent-images/ contract.

Browser/Go startup failure collection saves bounded Docker logs and inspect
privately before cleanup. Project only exact static stage markers from the
sole safe-diagnostics enum, never arbitrary exception/path/body text. The shared
factory verifies actual writer mount UID65532, mode0700, directory/non-symlink
before key initialization. The original813 browser-product failure remains
pre-writer-identity/unknown; added markers cannot retroactively explain it.

The browser-driver capture must be joined and its closed static entered-stage
marker classified before a missing witness-exchange assertion can return.
Preserve original bounded stdout/stderr/exit privately. Emit fixed markers for
browser hardening, endpoint configuration, DNS bind/probe, namespace controls,
proxy setup, WASM inventory/test capture, proxy negatives and DNS finish. These
markers establish the last entered stage, not an exact failing operation or
proof that a browser request occurred. Classify transport TimedOut separately
from other capture errors without interpreting absence as a witness defect.
Only complete known lines may enter the existing public error_code vocabulary;
unknown lines, appended payloads and raw browser output remain private. Preserve
the existing witness assertion, failure exit and full-product refusal. The
delivery consumer embeds this vocabulary, so its regression proof must be
renewed for changed contract bytes without borrowing an earlier test count.

Captured stderr also contains child-tool output. A complete known marker is
not an authenticated statement of which process emitted it; preserve this
source-prefix trust limit. The existing scoped capture worker autojoins on
early return, and the shared subprocess helper may retain blocking wait or
reader lifetime obligations. The130/135-second application timers alone do
not prove bounded return, cancellation, drain or cleanup. A diagnostic run may
use the genuine hosted outer job ceiling with those limits explicit; migrate
process ownership through the native/shared owner's retained-owner API rather
than implementing another local process primitive or relaxing failure gates.

### Typed browser actor diagnostic outcome v1

The bb763c1f/run42381878489 diagnostic reaches actor_browser_dns_finish but
still fails the unchanged witness-exchange assertion. Its original private
actor JSON is not recovered locally. Reuse that existing actor report before
the assertion; do not add another JSON-on-stderr protocol or publish raw tool
output. This slice changes diagnosis only, not scoring, test selection, fixture
request counts, image recipes, deadlines or product qualification.

The optional safe_diagnostics.browser_actor object uses
humanymous.lab-browser-actor-outcome/1 and the sole
test/lab/runner/browser-outcome-contract.json. Its exact fields are schema,
state, process_success, exit_code, inventory_matched, inventory_count,
selected_count, filtered_count, parser_summary_matched, test_passed,
dns_finish, source_prefix_authenticated and full_product_qualified. States are
observed, not_observed, missing, invalid, capture_deadline and capture_failed.
Unknown facts are null, never fabricated zero or false. Inventory/filtered
counts are unsigned and at most10000; selected count is at most1. Private
inventory names must be nonempty, unique and at most1024bytes; they never
enter the public outcome. Exit code is
null or a signed32-bit integer. A known successful process requires exit0;
known failure cannot carry exit0, while an unavailable numeric exit code is
allowed for a failed process. Inventory count must equal selected plus filtered
when observed. Test pass requires actual process success, matched inventory and
the independently parsed exact test summary. DNS state is succeeded, failed or
unknown; success/failure requires actual finish result in the owning report.
All non-observed states retain null scalar facts and unknown DNS. Both provenance
authentication and full-product qualification are always false.

The runtime adds actual test process success/exit and independent parser result
to its existing private report. It emits fixed test_passed/test_failed markers
after computing the predicate and dns_finished/dns_finish_failed markers after
the actual finish result. Preserve the private report on DNS finish failure
before returning the same error. The parent strictly decodes one bounded JSON
report, rejects duplicate or unknown fields and ambiguous inventories, compares
the fixed selected name and expected fixture with the frozen source, and builds
only the approved scalar object before the unchanged witness assertion. Failed
capture, missing report and invalid report remain distinct non-observations.

One pure test/lab/browser_outcome.rs validator reconstructs this closed public
object for both Lab and Delivery using existing dependencies. Delivery must
reject an invalid optional object, never silently copy or discard it. Existing
diagnostic failure rows and all fail-closed/full-required outcomes stay intact.
Compile and test the newly consumed contract in both owners; retain full source,
registry and effective configuration closure, then require a separately reviewed
literal source for one genuine Docker diagnostic run.

The architecture, red, operations, privacy and evaluator review converges on a
diagnostic-only boundary. Critique reflected: malformed/extra/duplicate data and
counter contradictions reject; unknown observations remain null; raw strings
cannot enter the rebuilt object; process exit and stage markers cannot stand in
for browser effects, authenticated witness exchange or cleanup. Combined actor
and child output is not independently attested, and shared subprocess lifecycle
limits remain explicit residuals. The source design is ready for implementation;
actual browser behavior and full product remain unmeasured by this delta.

The actual306/run42390164354 outcome matches inventory67/1/66 and completes
DNS finish, but the selected runner process exits1 without the exact passing
summary. The next source-only diagnostic layer may classify existing private
test output into three finite error codes, without changing the13-field object
or the witness assertion. Match whole source-owned lines only: pinned
wasm-bindgen-test0.3.80 rt/mod.rs emits `test {owned_name} ... FAIL` for a failed
test; wasm-bindgen-cli0.2.130 headless.rs emits its fixed missing-summary
sentence and a fixed `webdriver POST /session request failed` context. Only
standard Anyhow `Error: ` or a bounded two-digit numbered context prefix may be
removed for that last context. Never publish the matched line or any suffix.

Use browser_selected_test_failed, browser_runner_no_summary_reported and
browser_driver_session_request_failed in the existing closed error vocabulary.
Require one unique match, a valid observed actor report, failed process,
matched inventory and successful DNS finish before replacing the execution
diagnostic code. Duplicate, ambiguous, prefixed-payload, appended-payload and
unknown input retain the prior diagnostic; a missing summary is not proof of
timeout or non-execution. Source-prefix authentication remains false. Preserve
18-case accounting and the witness failure. Pure positive/injection tests and
consumer recompile review precede any separately granted actual retry.

Actual b4fdbc38/run42394466559 (Delivery evidence committed dcfe454d)
reports browser_selected_test_failed, successful DNS finish and unchanged
18/16/2 accounting. The next diagnostic refines only that fixed category using
the exact selected test and its compiled source bytes. Pin SHA256
c61b764905884233d7dc814cd6179e4e31e439921314d8a83a4475bbdadf4064
for crates/collector/src/browser/transport/runtime_pilot.rs. The pinned
wasm-bindgen-test0.3.80 panic hook preserves PanicInfo display in private
console output. Recognize only a complete trimmed `panicked at` location line
with that exact relative source path, an enumerated assertion/call-site line
and a canonical positive column within that source line. Emit a fixed reviewed
error code, never the location, panic message, value, stack, path or suffix.

Critique reflected: require the already validated failed actor, matched
inventory, successful DNS and exactly one selected-test FAIL before refinement.
Hash drift, foreign selected name, unknown/malformed panic location, duplicated
or ambiguous panic reports retain browser_selected_test_failed. Source bytes
and enumerated sites must be checked together in pure tests; every emitted
code must belong to the sole safe diagnostic vocabulary. This is a reported
panic site, not authenticated execution provenance or an independently proven
underlying cause. The13-field DTO, witness assertion, origin-zero expectation,
fixture, scoring, time budgets and full refusal remain unchanged. Record
source-qualified unit/check/Clippy/fmt and Delivery consumer proof before one
separately coordinated Docker-hosted diagnostic candidate; no unchanged retry.

For the distinct repaired candidate, both owned HTTP proxies record API request
header observations using only four fixed counter keys in the sole
test/lab/browser-wire-contract.json. Compare Origin exactly with the existing
browser_origin fixture; missing or different Origin increments the other
counter. Any Referer header, including an empty value, increments present;
otherwise increment absent. Record after strict owned API parsing, without
rewriting or logging either header. Negative foreign/unknown API controls do
not enter this denominator. Counters describe observed attempts, not completed
responses or authenticated effects; all existing response, witness and
zero-origin assertions remain required independently.

The parent adds two assertions requiring matching Origin and absent Referer at
BOTH proxies for the checked sum of existing expected.request_counts, with no
opposing observations. Missing good counters, malformed values and overflow
fail closed. Expected totals derive from the sole unchanged collector fixture,
never hardcoded five/two. No new actor or13-field public DTO fields, raw header
projection, request rewrite or production policy change is permitted. Pure
counter tampering tests, actor compilation/Clippy and actual Docker wire
observations qualify this separate source. The pre-fix e70 run measures neither
these counters nor the repaired source; successful repair cannot prove its
original assertion retrospectively. Privacy/red/operations/evaluator critique
is reflected in finite keys, opposing-count refusal and independent effect gates.
Retain the already classified finite actor failure through normal runtime drain
and the actual-test-completion assertion: a failed test after a witnessed effect
must not lose its panic-site code to a later receipt stage. Reset the receipt
stage only after actual test completion succeeds; no assertion is skipped.

Source8132117 run42355168729's actual Docker receipt reports26 scheduled,
0evaluated/19errors/7unsupported. It is internally consistent but not a scoped
pass. evidence:root/20261107083259 preserves the decoded public log and derived
projection; original private wargame bytes/causes were not recovered. Readiness
contract fixes and later Go source do not change that result.

## First-class Docker Compose integration support

Docker Compose is a supported deployment and product-test integration target.
It composes Humanymous runtime, authority, independent witness and backend with
nginx, Apache and language adapters; it is not a request-processing language
adapter. Support scope does not certify an unmeasured topology or authorize an
actual deployment. Current configuration/source and measured qualification are
tracked separately. New orchestration and tests are Rust1.99.1/edition2024;
Compose YAML remains configuration. Execution admission, local benchmark
exceptions, resource slots and runner budgets follow roadmap/delivery policy.
The user-authorized local/hosted timing comparison supersedes the earlier
blanket local restriction; no lab command starts during root's quiet window.
The current Go qualification contract still requires actual free hosted Linux.

For this strict-hosted Go profile, require GITHUB_ACTIONS=true,
RUNNER_OS=Linux, RUNNER_ENVIRONMENT=github-hosted and no ACT environment marker.
Reject local/unknown context before builds while retaining the eight scheduled
NOT_RUN rows. These conventional markers are not a signed remote attestation;
independently bind actual GitHub run/job/source/image receipts. Local same-YAML
act comparisons must not set a qualifying hosted=true receipt. A distinct
nonqualifying local profile is required before using Go8 for local comparisons.
Official references checked2026-11-07:
https://docs.github.com/en/actions/reference/workflows-and-actions/variables
and https://nektosact.com/usage/index.html#skipping-steps .

Canonical configurations are deployments/lab/compose.rust.yaml, its contextual
witness/browser overlays and topology/sandbox.yaml. The collector component
overlay remains distinct and network-none. Adapter-owned pilot configurations
are consumed only through reviewed fixed Rust operations and their explicit
source-bound contracts; never execute commands read from arbitrary JSON.
nginx's adapters/nginx/pilot-contract.json declares35 scenario groups, not35
Rust test-harness tests. Apache implementation and its actual hosted pilot
contract belong to the server-framework owner. Node/Python/Ruby adapters retain
their own authority/FFI contracts. Their component successes cannot fill the
composition's browser, witness or human/AT denominator.

Required integration contract:

1. Bind every role to immutable source inventory, exact root Cargo.lock,
   built/run image IDs, resolved Compose bytes and explicit protocol/scope.
   Never silently alter workspace membership or dependency locks inside builds.
2. Start the backend positive control, then authenticate/unlock witness and
   audit identity/head state, then start runtime/authority, privately provision
   scoped adapter credentials, and only then admit ingress/browser requests.
   Typed private readiness must bind actual endpoint, identity and required
   capabilities; a running PID, open TCP socket or loaded page is insufficient.
3. Use bounded Rust health/readiness probes and explicit startup ordering.
   [Docker startup documentation](https://docs.docker.com/compose/how-tos/startup-order/)
   distinguishes running from healthy and provides service_healthy and
   service_completed_successfully conditions. Dependency restart propagation
   covers explicit Compose restart/update operations. Request-time capability,
   credential, nonce, audit and witness checks must still fail closed after
   dependency loss; health state never authorizes a request.
4. Secrets and credentials travel over trusted inherited/private framed input
   or explicitly granted private files, never source/image layers, public
   readiness, environment dumps or secret argv. Verify actual ownership, mode,
   mount grants, peer identity and credential scope; YAML settings alone are
   not permission evidence. [Docker secrets documentation](https://docs.docker.com/compose/how-tos/use-secrets/)
   describes per-service file mounts; that transport does not establish the
   runtime's required ACL/UID or application authorization semantics.
5. Keep browser/attacker, trusted Gate/adapter, backend and witness trust
   boundaries explicit. The B profile separates browser and Gate namespaces;
   its backend/runtime remain Gate-loopback-only. Default UID65532 and witness
   UID65533, read-only roots, empty capabilities, no-new-privileges, disjoint
   key/head mounts, no host Docker socket, no public management/admin/origin
   ports and denied outward traffic remain mandatory. Management/observer IPC
   uses its separate authenticated native seam, not an ingress HTTP alias.
6. Drain adapters/ingress before runtime audit flush and witness shutdown;
   retain backend access only for already authorized work. Apply bounded
   private shutdown control and StopBudget, join/reap workers, and acknowledge
   Stopped only after the actual flush. Preserve timeout/poison/ambiguous state
   as failure or explicit closed recovery evidence, not graceful success.
7. Measure witness loss before effect, ambiguous durable tails, same-identity
   restart and externally pinned head recovery. Never delete an authenticated
   complete record, reprovision identity, automatically retry a logical effect
   or label a partially witnessed tail fully attested. Audit-owned recovery
   policy/receipts and runtime-owned fresh credential epochs govern recovery.
8. Capture exact case/group inventory, statuses/ACKs, independently observed
   backend effects and authenticated audit/witness receipts. Preserve failed
   raw evidence privately; only validated safe_diagnostics and approved
   summary/log hashes enter artifact-free public CI output. Run owned-project
   cleanup on success/failure/cancellation and verify containers, networks and
   volumes removed; labels alone are not cleanup or a scheduled janitor.

The external-DNS profile explicitly configures only a loopback resolver. A
Rust deny-only UDP53 responder inside the browser namespace records hashed
owned questions and never forwards them. The actual libc lookup must reach
that responder and receive no address. An independently healthy Rust DNS
canary on the separate outside internal network first proves a real query/
NXDOMAIN response. Browser direct UDP routing to that inspected canary must
produce an expected no-route/permission error, and the canary's before/after
query counters must remain unchanged. Local denial, outside counters and
kernel route observations are separate required receipts. No public DNS is
probed; timeout, unknown error or inability to bind caps0 UID65532 UDP53 is
failure/unverified, never a reason to add privileges. Shared bounded wire
frames live in test/lab/dns_wire.rs, with one implementation for both contexts.

Wargame reports retain their exact emitted bytes and use the owner's shared
strict decoder/verifier. The wrapper matches raw SHA256, verifier schema/mode,
scoped status, totals and full-product=false before accepting a byte receipt.
Generic duplicate-key-safe JSON parsing is shared test-support code; lab does
not duplicate that visitor. Current source invokes audit-owned
plan_witness_recovery with mandatory external latest pin and RequireCovered
for normal browser shutdown, rejects an unexpected repair requirement, and
checks the returned authentication/coverage separately from deployment claims.
Resolved Compose files are loaded from the same immutable frozen context as
the images, rather than rereading mutable working-tree configuration.
The small public receipt contract is test/lab/runner/public-receipts.json.
Its wargame projection includes exact source/raw/manifest hashes, verified
schema/mode/scoped status, all six totals and every contiguous case index with
terminal status/expectation. It excludes session, installation and process
identities, raw bodies and free-text reasons. A generated receipt survives a
later component failure; absence or malformed accounting is not replaced by
zero success. Recovering this projection does not recover the private raw.

The supported composition is qualified only when all its required actual gates
pass with complete source/effect/cleanup evidence. Skipped, ignored, unknown,
unrecovered private raw, component-only or old-source green results cannot
satisfy it. Current browser B source0fa5c31 is not_run on Linux/hosted; witness
outage/recovery, external DNS and adapter-composition gates remain active work.
No paid resource, public deployment, publish, tag or release is authorized by
this support declaration. Deployment consent remains a separate user action.

## Remote execution design delta (2026-11-07, not_run)

### Accepted same-origin browser product pilot contract (not_run)

Reuse the pinned wasm-bindgen-test-runner0.2.130 and Firefox ESR on hosted
Ubuntu Linux. Its installed Rust source uses WASM_BINDGEN_TEST_ADDRESS for
both test-server bind and WebDriver navigation (wasm_bindgen_test_runner.rs
around424, headless.rs around175). A different proxy port cannot be selected
through that variable alone. Do not modify upstream generated JavaScript or
author a JavaScript harness. Existing tool-generated glue remains tooling.

The minimum proposed wiring uses separate browser and Gate network namespaces:

- Browser namespace: WBG asset/test server S=127.0.0.1:31111 and bounded Rust
  explicit HTTP proxy F=127.0.0.1:31112. Firefox navigates S, preserving the
  runner's existing test completion accounting. Typed WebDriver JSON selects
  manual HTTP proxy F, empty noProxy and the Firefox localhost-proxy preference.
  These settings are interventions; actual proxy hit receipts must prove they
  work. No browser sandbox flags are disabled.
- Gate namespace: only lab Rust frontend G is bridge-reachable. Actual runtime
  Q and fixed synthetic backend R remain distinct loopback listeners. Runtime
  scope.authority is127.0.0.1:31111 and browser.origin is
  http://127.0.0.1:31111, independent of Q's physical assigned port. The runtime
  configuration's loopback-only validation stays intact.
- F sends exact runner asset requests to local S and only exact browser API/
  protected fixture routes to fixed G. G forwards those APIs only to fixed Q;
  G never routes to R or serves an alternate origin. Browser loopback cannot
  reach Q/R in the Gate namespace. Use a dedicated internal bridge, no host
  port publication, no external default route and no public DNS/metadata/daemon
  access. Component compose.collector.yaml keeps network_mode:none unchanged.
- Witness remains a separately owned UID65533/key/head-volume role; browser
  and writer use UID65532 without its key mount. Authenticated external-witness
  integration in the actual runtime host is a prerequisite, not supplied by
  merely rerunning the separate witness component. Until that dependency is
  implemented and measured, witness integration and full product remain false.

F accepts only owned http absolute-form targets whose authority equals the
unique Host header. Remove exactly that owned prefix to obtain origin-form;
preserve method, encoded path/query, entity bytes, signed Content-Type and
application Authorization, Origin, Fetch Metadata and RIT fields. Never repair
missing required browser fields. Remove proxy-only hop fields that the actual
runtime rejects. Reject CONNECT, foreign authorities, userinfo, alternate ports,
unknown API paths, redirects and ambiguous/duplicate framing. G preserves the
same request bytes after this bounded transport conversion. Relay actual
status, body and ACK unchanged, with no automatic retry or synthetic response.
Reuse the runtime's bounded HTTP framing/parser where applicable; extending
absolute-form parsing belongs to lab Rust, not the production protocol.

Acceptance requires: actual bootstrap and signed protected Fetch requests,
valid ACK advancement, independently counted backend effects and authenticated
audit records; missing/bad ACK invalidates the client epoch; altered/missing
binding, replay and witness outage cannot release an effect. Exercise direct
browser-to-backend, unknown route, foreign proxy destination and metadata/DNS
negative controls. Record OS/browser/tool versions, namespace/listener tuples,
UID/caps/read-only/egress/sandbox observations, source/lock/image commitments,
proxy asset/API hit counts and complete test denominators. A loaded page or
WebDriver zero exit alone is insufficient. This cookie-free buffered browser
session slice does not qualify collector producer provenance, TLS/H2 planes,
human/AT input or the intentional T4 ceiling. Collector-required routes still
fail closed when qualified reports are unavailable.

Responsibilities: collector/root owns Rust WASM pilot tests and typed browser
configuration; lab owns F/G, Installation reuse, the distinct product Compose
profile and effect/audit receipts; runtime owns external-witness host injection
and production browser contract; delivery owns its image/hosted execution.
No implementation or execution of this candidate has occurred. The one-
namespace/manual-proxy alternative was rejected for product isolation because
browser code could directly contact the loopback backend. It remains only a
possible Fetch/session/ACK component experiment with isolation unqualified.
Root accepted the two-namespace contract before implementation. Exact fixture
scope/routes are consumed directly from crates/collector/pilot/fixture.json,
schema humanymous.browser-runtime-pilot/1, owned by collector/root. It specifies
installation=install,tenant=tenant,origin=origin,revision=r1 and authority equal
to browser location.host at S. Exact POST allowlist is
/_humanymous/browser/session, /_humanymous/browser/collect, /effect and
/requires-collector. Both protected routes opt into browser_auth_only; only
the latter requires the unqualified collector plane. Root's feature-gated
runtime-pilot WASM flow expects bootstrap success, pre-report /effect403 with
valid ACK, real BrowserCollector snapshot submission200 with valid ACK,
collector-required route422 without ACK, which closes the client epoch; a
subsequent fetch refuses without network activity. Do not
rewrite the actual browser report to obtain ALLOW. Independent origin effects
for this flow are zero; existing synthetic ALLOW fidelity retains a separate
denominator. Status/ACK expectations must be reconciled with actual runtime
behavior before claiming success. Existing hosted run
42348518353/source63f69ae1 observes earlier source and must not be duplicated
or cited as this pilot's evidence.

The updated single selected browser test uses fixture.expected.effect_requests
as its sole request-count authority. Local65537-byte rejection occurs before
client state.begin, leaves the epoch open and emits no request; it is not
evidence of server413. Actual65536-byte Fetch plus the original small effect
produce the fixture-declared pre-collect403/ACK requests. Other known routes
remain one request each. Actor/runtime declared expectations and independent
parent comparisons share one pure Lab helper; missing, zero, noninteger,
unknown fields/routes or counts outside1..32 refuse rather than fall back.
Derive expected decision records, total audit sequence and witness coverage as
N CHALLENGE decisions plus one COLLECTED record. Validate actual proxy response
and request counts against this declaration, not a duplicated literal2 or an
at-least threshold. This source delta requires a new actual browser run; no
existing native headers-only413 control or earlier browser source qualifies it.

Purpose: relieve local validation memory pressure while preserving source-bound
Rust product evidence. This is a design-only delta: no workflow dispatch, image
publication, account mutation, paid resource or remote pilot was performed.
Delivery owns the GitHub-hosted Ubuntu execution/source-artifact contract in
delivery.sot.md; this document owns provider compatibility and isolation gaps.
GitHub-hosted Ubuntu running the existing Compose lab is the primary product
e2e candidate. Cloudflare Containers is a supplementary actual Cloudflare-path
API compatibility candidate. Neither candidate is qualified by this design.

Official Containers material checked on 2026-11-07:

- [Pricing](https://developers.cloudflare.com/containers/platform/pricing/):
  Workers Paid starts at USD5/month; monthly included compute is25GiB-hours
  memory,375vCPU-minutes and200GB-hours disk. Additional rates are
  USD0.0000025/GiB-second,0.000020/active-vCPU-second and0.00000007/GB-second.
  Provisioned memory/disk and active CPU are billed in10ms intervals. Workers,
  Durable Objects and logging have separate charges. Egress is region-dependent:
  NA/EU USD0.025/GB after1TB, Korea/Oceania/Taiwan USD0.05/GB after500GB,
  elsewhere USD0.04/GB after500GB. Included amounts are account-wide monthly
  allowances, not a per-run budget or a hard spending ceiling.
- [Limits](https://developers.cloudflare.com/containers/platform/limits/):
  predefined sizes range from lite(1/16vCPU,256MiB,2GB) to
  standard-4(4vCPU,12GiB,20GB); custom sizes require at least1vCPU and3GiB/vCPU,
  with defaults capped at4vCPU/12GiB/20GB. Account defaults include1500vCPU,
  6TiB memory and30TB disk concurrently. These are capacity limits, not budget
  protection. Current collector512-task settings have not been qualified there.
- [FAQ](https://developers.cloudflare.com/containers/faq/): disk is ephemeral
  by default and sleep starts a fresh image filesystem; durable_object snapshots
  are immutable point-in-time state. Docker-in-Docker needs root, does not
  support rootless Docker or iptables manipulation, and uses inner host
  networking; durable_object scheduling additionally needs IP forwarding off.
  Consequently its documented DinD example cannot substitute for our three
  isolated Compose bridges or demonstrate independent durable witness state.
- [Outbound traffic](https://developers.cloudflare.com/containers/configuration/outbound-traffic/):
  public internet access defaults on. With enableInternet=false, permitted
  outbound HTTP(S) uses ports80/443 and explicitly allowed hosts/handlers;
  non-HTTP traffic is denied. DNS follows outbound policy and can resolve
  placeholder addresses, so successful resolution alone does not prove egress.
  deniedHosts with internet enabled does not block other-port traffic. HTTPS
  interception requires an ephemeral CA and changes the measured TLS path.

The rate-only compute upper estimate before allowances for one15-minute
standard-1 instance at continuously active0.5vCPU is
900*(4*0.0000025+0.5*0.000020+8*0.00000007)=USD0.018504.
This is arithmetic, not a quote: add all instances, startup/retries, regional
egress, Worker/DO/log/storage charges and any plan subscription. Account usage
and taxes are unknown. No free run or total-cost bound is claimed.

| Boundary | Compose reuse requirement | Containers compatibility gate |
|---|---|---|
| Execution | Same Rust runner, immutable source inventory, built lock and image/config digests; inspect actual remote daemon | Rust image support does not establish Rust-only lifecycle/binding support; direct container API, Rust Worker/DO bindings, private framing and cf CLI lifecycle remain untested |
| UID/sandbox | Re-measure UID65532/65533, capabilities, no-new-privileges, EROFS, namespace/browser sandbox and task limits | Provider VM isolation alone proves none of these application boundaries; inspect actual behavior without sandbox-disabling flags |
| Network | Retain internal bridges, separate roles, no published admin/origin, no default route or metadata/daemon access | No Compose/DinD equivalence; exact private role routing, bypass denial, DNS, non-HTTP and IPv6 tests required |
| Witness | Separate populated key/head volumes, public supervisor pin, denied writer key access, restart and outage with zero effects | Snapshot/object storage is not FileAndDirectory fsync or independent latest-head durability; rollback, identity persistence and physical failure remain unqualified |
| L1/L2 | Label direct runtime observation separately from any provider edge | Cloudflare termination/re-TLS changes provenance; HTTP success cannot qualify original-client TLS/H2 or fill required missing planes |
| Evidence | Remote run/job ID and exact source/image/lock tied to raw assertions, independent effects and cleanup receipts | Provider API success is a separate evidence type; never add its cases to Compose/product denominators |

Before any future provider run: explicit account/owned-domain opt-in, maximum
instances and runtime, cost ceiling, exact allowlist, no public management,
private credential input, independent expiry janitor and verified cleanup are
required. Expiry labels and sleep are not a janitor or hard budget. Do not
provision until the Rust-only control-plane contract is demonstrated without
new authored JavaScript/Python/shell implementations. Use cf CLI unless a
Wrangler project configuration applies. No CLI account inspection occurred.

Adversarial design review (single author, five persona lenses): block provider
equivalence claims on unmeasured networking/sandbox/state boundaries(P0);
block required-plane satisfaction from re-terminated traffic(P0); require
source/artifact binding and cleanup on failed/cancelled runs(P1); keep budget,
Rust bindings and human/AT evidence explicitly unavailable(P1). These findings
are requirements above. Verdict: needs-work for execution; acceptable as a
design-only candidate split. All remote execution and compatibility gates are
not_run. Local source60627 remains its own71-assertion component receipt;
later source edits and this design have no new execution result.

## Native writer-lifetime regression profile

compose.custody.yaml builds Rust audit/authority tests and warnings-denied Linux
Clippy in an owned image. Default execution is UID65532,network none,read-only,
cap-drop ALL and no-new-privileges. The separately named peer profile requires
container UID0 plus only SETUID/SETGID to launch the UID65534 negative child;
no privileged-container flag,host mount or network exposure is added. The parent
must verify the helper actually executed, not just a zero-exit harness.

The pre-fix duplicate-description regression fails with WouldBlock. The shared
guard explicitly unlocks on Drop and both custody/WAL regressions pass without
unlink or retry. This is native ownership evidence; the separate real-runtime
process pilot remains bounded origin/audit evidence, not full product e2e.

## Passive behavior component extension

The collector-browser test image includes only the unchanged existing
web/js/features/{mouse,keystroke}.js as read-only differential oracles alongside
Rust WASM tests. New reducer/listener/lifecycle implementation and tests are Rust.
Native13/browser13 pass within the recorded source manifest, including16oracle
datasets and real synthetic dispatch/cleanup/reentrancy. This is not a human/AT
baseline or product e2e. Production WASM excludes these cfg(test) imports; retain
the final foreign-source cleanup gate and all network-none/non-root restrictions.

Version 0.3.0. Rust-only coordinator and contextual component implementation checked 2026-11-07 KST.
Accepted design, not a claim of implemented Rust full-lab or Cloudflare deployment.

## Decision

Make an isolated Linux Docker Compose lab the product integration authority.
Use Cloudflare as separate opt-in compatibility profiles: named Tunnel/Access
to the owned Compose Gate, guarded Browser Run sessions, and optionally managed
Containers for provider-specific HTTP/lifecycle tests. Native Windows/macOS
qualification remains on actual OS runners. These are complementary contexts,
not interchangeable certificates of full protection. Preserve scoring freeze,
local-only management, missing-signal fail-closed policy and human/a11y controls.

## Evidence vocabulary

| Evidence type | Meaning | Cannot substitute for |
| --- | --- | --- |
| engine-simulation | Actual shared Rust scoring engine, synthetic controls | Live network/browser/Gate e2e |
| adapter-conformance | Real framework/proxy with mock authority | Rust authority security/durability |
| rust-product-lab | Actual Rust Core/Gate/state/witness/browser, isolated Docker | Native Windows/macOS or provider deployment |
| provider-local | Provider development runtime plus local Docker | Deployed network/capabilities/storage semantics |
| provider-remote | Owned dedicated account/hostname, actual managed deployment | Original-client raw capture or independent OS qualification |

Current deployments/compose.yaml is the Go oracle; core/gate Dockerfiles still
compile Go. The nginx lab is a Linux njs/mock-authority compatibility slice and
the Rust wargame network-none container is an engine simulation. None is a Rust
full-product pass. Existing Go admin host publication and shared web/origin
network must not be copied into the new local-only Rust topology.

## Canonical Compose structure and boundaries

Proposed future layout: deployments/lab/compose.yaml, topology/, authority/,
clients/, adapters/, scenarios/, providers/. Share base service definitions;
overlays express only contextual differences, avoiding duplicated topology.
Profiles select services; they are not security boundaries. Hash the resolved
configuration and inspect actual container/network/mount/port state. [D1-D4]

- Gate accepts only the ingress lab network and private origin/state networks.
- Origin has no published port and is reachable only through Gate's backend
  network. Attacker, human and connector cannot call origin directly.
- Management is loopback inside the owning process/container or restricted UDS,
  never a public or Tunnel route. Positive management checks use local CLI/exec;
  paired negative checks run from unauthorized containers/identities.
- Each adapter has narrowly scoped IPC mount/credential, real peer validation,
  exact request/body/header/target binding and no origin retry.
- Witness has its own process, key and state volume. Writer cannot read its
  private seed. Separate containers alone do not prove operational independence.
- Replay/state has no public ingress and must fail closed on loss/reload.
- Attackers receive owned installation-bound targets, no host mounts, Docker
  socket, production credential, cloud metadata or unrestricted network.
- The host coordinates Docker; product HTTP probes execute inside the lab.
  Control-plane downloads and attack execution are separate phases.

Use internal networks, non-root, read-only root filesystems, cap_drop ALL,
no-new-privileges, resource limits and bounded tmpfs where compatible. Verify
effective behavior rather than trusting YAML. Readiness needs health checks and
service_healthy dependencies; test dependency loss after readiness as well.
Compose secrets are file injection, not encrypted secret management. File-source
secret uid/gid/mode declarations can be ignored by bind mounts; verify actual
host/container permissions. Do not mount the container API socket. [D1-D4]

## Test profiles

| Profile | Authority and coverage |
| --- | --- |
| direct-edge | Rust directly receives client TLS/H2; actual authenticated capture |
| trusted-l4 | TLS passthrough and authenticated forwarding provenance |
| terminated-proxy | Deliberate capability loss after TLS termination; no invented raw signals |
| node-complete-http | Supported bounded Node/Bun Express/Hono requests and real IPC authority |
| nginx-complete-http | Supported nginx contract, body/header/target and response compatibility |
| state-recovery | WAL/replay/witness failures and restart, not complete physical power-loss proof |
| wargame-runtime | Actual owned Gate/browser attacks and honest human/a11y controls |
| cloudflare-local | Worker/provider API integration in development runtime |
| cloudflare-tunnel | Access-authenticated edge-to-owned-Gate HTTP compatibility |
| cloudflare-containers | Managed Linux amd64 image/lifecycle and HTTP compatibility |
| cloudflare-dind-experimental | Supplementary execution with shared inner host network; no canonical isolation claim |

## Cloudflare Containers findings

Current docs describe Linux amd64 Firecracker execution with Worker/DO-mediated
ingress. Do not treat it as a native Compose import target or arbitrary client
TCP/UDP capture environment. Worker request.cf metadata is provider observation,
not full original ClientHello/H2 frames/QUIC packets. HTTP/WebSocket platform
examples do not expand the adapter v0.1 streaming contract. [C1, C2, C6]

Docker-in-Docker is supported with root dockerd and disabled iptables/ip6tables/
IP forwarding; documented inner-container networking uses host mode. This does
not preserve canonical Compose bridge/internal-network isolation. Rootless Docker
is not a documented replacement. Therefore never mark origin/attacker/witness
separation proven by merely running compose inside a managed Container. [C2]

The default filesystem is ephemeral. Current snapshots exist under the
durable_object scheduling policy but are immutable point-in-time state, not
continuous fsync WAL authority or latest-head antirollback. External durable
append receipts/epochs and independently persisted witness heads need their own
design and crash/rollback tests. R2 FUSE does not automatically establish POSIX
fsync semantics. Local development and deployed capabilities/lifecycle differ;
non-root local hardening cannot be assumed equivalent after deployment. [C1-C4]

For owned tests use enableInternet=false plus narrowly allowed destinations or
handlers and actual negative assertions. deniedHosts with internet enabled does
not close alternate-port traffic. Interception introduces another TLS/CA boundary.
Containers requires the applicable paid plan; limit, memory, CPU, disk, egress,
Worker/DO and log costs need explicit budget before provisioning. No paid
Cloudflare resource was created by this research. [C5, C7]

## Tunnel, Access and Browser Run

Preferred opt-in route: guarded remote Browser Run Chrome -> exact owned test
hostname -> Access Service Auth -> named Tunnel -> connector -> Gate data-plane
listener -> private origin. The connector alone gets necessary external egress;
it has no origin/admin socket, host network or Docker API access. Catch-all
ingress rejects, origin TLS is verified, and Access audience/team is checked at
the connector where configured. Access is lab admission, not a Humanymous verdict
or proof of a local peer. Separate Access, Tunnel, provider API, adapter and admin
credentials. Never expose administration because Access protects a hostname. [T1-T3]

Quick Tunnels support current email restriction options, but temporary hostnames,
concurrency/SSE limitations and interactive authentication make them unsuitable
for canonical CI qualification. Remote browser localhost is not CI localhost.
Browser Run session allowedDomains must contain exact owned hosts; omitted is
unrestricted and an empty list blocks HTTP/HTTPS. These session guardrails do not
apply to Quick Actions and are not a kernel firewall. No broad wildcard or CDN
allowlist is needed for self-contained fixtures. Avoid credential leakage via
redirect/subresource headers; test it explicitly. [T4, B1, B2]

Browser Run's automation headers and Web Bot Auth require actual cryptographic
coverage/freshness/replay validation before attribution. Header presence and UA
alone prove nothing. Cloudflare edge TLS/header changes make proxy observations
different from original raw capture. Provider WAF skip/allow rules must be named
in the evidence; a provider block is not Humanymous detection success. Remote
Chrome results do not generalize to all Computer Use vendors or humans. [B3, C6]

Use synthetic data only. Screenshot/HAR/session recordings must exclude secrets
and personal data; provider cache/retention are separate settings. Track budget,
resource IDs and expiry. finally cleanup plus an external TTL janitor must close
browser sessions and revoke tokens/routes/DNS/Access/Tunnel/container resources
after cancellation or runner loss. Cleanup failure is a recorded failure.
Restrict remote testing to owned bounded compatibility cases; do not send attack
floods or third-party probes through Cloudflare shared infrastructure. [B4]

## Assertions and acceptance

Required assertions include compose-resolved-topology, no-public-admin,
no-admin-via-adapter, no-origin-bypass, attacker-no-internet, attacker-no-host,
attacker-no-metadata, attacker-no-container-api, installation-target-binding,
unknown-target-refused, tls-direct-provenance, proxy-termination-capability-loss,
spoofed-forwarding-rejected, body-header-target-binding, nonce-replay,
replay-after-restart, origin-once, dependency-not-ready,
dependency-lost-after-ready, state-unavailable-closed,
audit-record-fsync-before-effect, checkpoint-failure-blocks-effect,
witness-fork-refused, witness-head-restart, human-allow-control,
keyboard-challenge, assistive-challenge, zero-motor-control,
coherent-browser-ceiling, container-recreation, rolling-config-epoch,
credential-rotation and cleanup-owned-resources-only.

Provider cases additionally require provider-local-remote-differences,
provider-outbound-deny, provider-control-plane-loss, provider-data-plane-loss,
provider-session-instance-binding and anonymous/wrong-audience/expired/cross-run
admission rejection before origin access. Test direct IP, IPv6, alternate ports
and host paths separately; DNS failure alone is not an egress isolation proof.

Evidence binds exact SHA, Cargo.lock/Compose/config/schema digests, actual image
digests, observed capabilities and peer provenance, profile/provider mode,
case assertions and result denominators. Unavailable/unsupported/not_run differ
from passed. Mock/engine/local-provider/remote-provider cannot be aggregated as
Rust full-lab successes. Missing required cases block release readiness.

## Critique and rollout

Blue requires authenticated capability loss instead of invented raw signals;
red requires direct-origin and egress negatives; SRE requires readiness/loss and
orphan cleanup; DPO requires synthetic fixtures and redaction; evaluator requires
honest denominators and T4 ceiling. Design accepted for implementation planning;
runtime/product/provider qualification remains incomplete.

Complete Rust authority/server/browser/state/witness first; then create new Rust
images and Compose topology, wire real adapters and run the direct isolated full
gate. Add provider-local and protected remote compatibility profiles afterward.
Current repo has no Wrangler config; Cloudflare account operations use cf CLI
unless that project condition changes. Documentation examples mentioning Wrangler
are product research, not permission to install or migrate to Wrangler.

## Implemented-context acceptance boundaries

The base `deployments/lab/compose.yaml` owns synthetic origin/canary controls
and hardened Docker client images. It contains no substitute Gate, collector,
state service or witness. Its results are `topology-fixture`, never
`rust-product-lab`. All three networks are internal; an owned canary on a
separate bridge tests actual inter-network denial without third-party traffic.
Direct origin IP denial is paired with a healthy backend positive control.
DNS denial alone is insufficient. Gateway Docker API and metadata probes cover
specific addresses/ports, not every host service or IPv6. These residuals stay
unavailable rather than being converted into broad no-host/no-internet claims.

The contextual `compose.authority.yaml` adds the actual available Rust authority
and installed Node Express/Hono adapters in a network-none container. It uses
the authority owner's private inherited-stdin configuration and piped-readiness
contract; tokens and ephemeral test audit keys are not image/config/environment
data or persisted evidence. Same-process supervisory custody is intentional,
not independent witness operation. Its evidence type is
`rust-authority-node-ipc`; application-HTTP observations cannot qualify original
TLS/H2/browser capture. nginx remains unexecuted against Rust because its pilot
protocol differs. Per-process restart/WAL authentication in tmpfs is distinct
from durable volume/container recreation, external latest-head or power loss.

`test/lab/run.py` coordinates unique owned projects and runs network clients
inside Docker. Required product checks missing at the end force
`releaseReady=false`; `--require-full` exits 2 after scoped cases pass. Execution
failures exit 1. Cleanup runs in finally and checks project resources are gone.
Resources carry an explicit 1800-second expiry and owner labels; an external
operator/runner can invoke `test/lab/cleanup.py` to reap only expired projects
matching the generated project grammar. No active external scheduler is implied.

Images use context allowlists and pinned base digests. Build/dependency fetches
are control-plane activity before network-isolated tests. Runtime keys are
generated after image creation; image provenance and vulnerability/package scans
are separate required outputs. A scanner authentication/download error is
`not_run`, never a clean CVE result. Shared-checkout source/config digests and
dirty paths accompany Git SHA because another chat can change an unrelated
crate between builds. Do not describe a dirty working-tree build as exactly the
committed SHA.

### Provider opt-in contract (not implemented deployment)

Named Tunnel/Access and Browser Run remain disabled. Before any future provider
profile can execute, require an explicit opt-in, owned exact hostname, Access
team/audience, separate Tunnel/Access/browser credentials, bounded budget and
expiry, a qualified Gate data listener and external cancellation janitor. Missing
inputs must cause a nonzero preflight before cf account operations. Browser
allowedDomains contains only the exact owned fixture host, redirects and
subresource credential leakage have negative cases, ingress has a catch-all
reject, origin TLS verification is enabled, and no management listener/socket is
mounted or routed. This implementation performs no paid provisioning, public
tunnel or account mutation. Cloudflare DinD/ephemeral WAL is not equivalent to
canonical internal bridges/independently anchored durable state.

### Rust-only continuation and provider preparation (2026-11-07 KST)

#### Accepted inter-owner contracts and current state

`humanymous.product-lab/2` (implemented; source-specific execution pending after
each delta) has project, evidence_types, cases[{id,status,detail}], components,
required_unavailable, full_product_qualified, release_ready and ttl fields.
`status` is passed/failed for executed orchestration assertions; component
not_run/unsupported must stay explicit. Exit1 is execution/assertion/I/O failure,
exit2 is consistent scoped success with required full gates missing, exit0 is
scoped success only. Neither exit0 nor exit2 certifies the whole product.

Accepted with CI owner: default run requires actual runtime; --collector adds
Firefox component tests; --topology-only explicitly records runtime not_run.
Inputs are an existing private Specfold bundle, no target URL. Host coordinates
Docker only. Before copying, tracked dirty deletions are inventoried as deleted;
source disappearance/reappearance/content or inventory change during copying is
a refused race, never a missing-file stub. New root-workspace Rust tooling roots
are allowlisted without private/caches/foreign executable files. Common Cargo
and toolchain remain root-owned. Runner and runtime-pilot are root members;
the old runtime-pilot standalone lock is retired.

Accepted with Windows/native owner and implemented consumption: the runner uses
test-support::process::capture_with_stderr from26d9925, bounded stdout/stderr and
actual status, kill/reap deadline. Private outputs never become public error
dumps. Source/image requests are argument vectors, without an authored shell.

Accepted with wargame owner1ee2c92 and implemented lab context:
attacker::network::OwnedLab {spawn,snapshot}; defender::lab::OwnedRuntime spawns
only the fixed sibling runtime and checks private readiness/live child/same
loopback-only namespace. VerifiedSnapshot::read accepts only an authenticated
shared Log and observed origin count. Installation has one custody/config/origin
owner for original process and new wargame profiles. The26case ledger preserves
19 actual program definitions and7 unsupported producer/participant cases;
actual network success awaits the source-bound Docker run. Public checkpoint
and binary/source hashes are not remote signed target attestation.

Accepted with audit ownerd001ee6: WitnessServiceConfig/Request/Response bounded
private stdio frames; WitnessServiceClient::connect/from_ready implements the
shared CheckpointWitness trait and verifies pinned key/head/receipts. Lab owns
transport provenance, deadlines, separate writer/witness UID and key/head
volumes; audit owns crypto/proofs/state/codec. Implemented execution uses repeated
short-lived witness containers with durable named volumes and an independently
retained supervisor pin, private passphrase delivery only to witness, and no
writer witness-key mount. Whole-profile qualification is source-specific and
pending the complete failure matrix; early setup/transport failures are retained.
Runtime owner has accepted a future trusted library dependency injection seam;
main binary authenticated sidecar transport and full Gate remain unavailable.

The lab-only wrapper bootstrap1 is one private newline JSON frame
`{create_identity:bool,config:WitnessServiceConfig}`. The wrapper refuses
identity replacement, generates witness seed/HMAC with OS entropy inside
UID65533, opens only `/witness/keys` and `/witness/head`, and emits a verification
public identity before handing all requests to the unchanged audited service.
The writer's separate bootstrap is `{create_identity,passphrase,trusted_head}`;
its fresh seed/HMAC stay inside UID65532 sealed custody. Shared
WitnessServiceClient::from_ready and Log::open_with_external_witness enforce the
signature/head/durability contract before an actual counted owned-origin call.
No scoring is performed by this witness ordering fixture and its synthetic
audit ALLOW is not an engine/human result. Runtime/wargame engine results remain
a separate component.

Broker transport consumes e30e0e9 ManagedChild::spawn_duplex; private stdin is
kept open until typed Stopped acknowledgement. Framing uses audit's128KiB
read_frame/write_frame, per-frame15/30s deadlines and at most8 writer commands.
The supervisor owns direct child reaping and project cleanup. Public identity,
State/CounterSign/Shutdown and signed receipts are the only exposed frames;
secret bootstrap inputs never enter transcript/args/env/images. Empty-volume
initial directory ownership/mode is explicit in image COPY metadata and is
verified through actual native custody and different-UID EACCES tests.

`humanymous.independent-witness-lab/1` reports status, per-writer results
(status/origin_effects/audit_sequence/checkpoint), transaction count, pinned
checkpoint/public identity and actual UID/volume separation. It explicitly
marks runtime_binary_integrated, power_loss_qualified and persistent external
supervisor pin as false. After the actual witness transaction acknowledges
shutdown and exits, an injected broker control-plane outage returns typed
Unavailable rather than a signature; an actual Log record may already be
durable but the origin callback must remain unexecuted. This is controlled fault
injection, not a claimed production provider outage or EOF observation.

Accepted evidence lifecycle delta: each run requires a fresh Specfold bundle;
pre-existing source-manifest/lab-report causes refusal before any overwrite.
Pre-existing audit-report is also refused. The mistaken55200 first witness
attempt lost its original dirty source graph during a retry-path error; its
failure report/transcript/image descriptor are preserved under55537 failure-*.
55200's retry source,60036's later transport-timeout failure, and54017's complete
33assertion/19evaluated/7unsupported success stay distinct. Never infer the lost
graph from a later manifest. Later success is not replacement of these failures.

Rust audit command consumes only recorded immutable image IDs and the exact
built lock bytes/hash. Scanner DB/time/export/config digests and Cargo warnings
are separate outputs. Secret matches are sanitized before any artifact is
persisted. Scratch/musl controls/runtime avoid unrelated OS/installer packages;
this does not qualify the collector image or eliminate the separate dependency
and native-engine supply-chain gates. Fresh scans remain required.

The canonical v2 coordinator is `humanymous-lab-runner`, a normal root workspace
member at test/lab/runner. It owns bounded Docker subprocesses, immutable source
snapshots, actual inspect checks, Rust HTTP fixtures and negative clients,
project-scoped cleanup and non-mutating provider preflight. No Python/Node or
embedded foreign-language execution string implements these responsibilities.
Legacy run.py/cleanup.py/audit.py and Node prototype profiles remain historical
migration debt and are not invoked by compose.rust.yaml. Their old scan/pass
results do not qualify new images. Full source-bound execution results are
recorded separately after each immutable snapshot.

The runtime process container preserves its existing loopback-only native
protocol. Rust client, owned origin and runtime share network-none; actual
origin/audit counts are meaningful component effects, not independent L5
isolation. Separate live fixtures on backend/outside bridges establish bounded
direct-IP negatives from the ingress client. Absence of a default IPv4 route is
observed in addition to finite probes. No third-party egress destination is
probed. Full runtime cross-namespace origin/audit effects, IPv6/host breadth,
session-bound real browser evidence, independent witness and human/AT remain
required unavailable gates. Sandbox-disabling flags remain prohibited.

Provider execution preparation is an explicit local manifest/preflight only.
The example is fail-closed: opt-in false, zero budget/expiry and missing Gate
commitment. Preflight checks exact owned hostname, positive budget, expiry within
one hour, an external cancellation janitor, Gate source commitment and credential
presence without reading/printing values. Cloudflare also requires exact
allowedDomains plus Access team/audience. Passing prerequisites does not enable
deployment; the command exits 2 and performs no account/resource operation.

AWS Lightsail Containers expose a managed TLS endpoint and optional private
service domain. Omitting its public endpoint does not establish per-container
Compose bridge policies or original-client TLS capture. Its price depends on
power and scale; no fixed cost or free operation is assumed. An EC2/Lightsail VM
profile would require a separately approved instance, firewall/metadata-egress
policy, independent witness storage/account, artifact/boot provenance and TTL
cleanup before execution. Neither managed-provider ingress nor same-VM namespace
sharing certifies canonical isolation. Cloudflare enableInternet=false plus
allowed destinations and ephemeral/snapshot filesystem limitations remain
separate provider semantics. No paid resource or external deployment is created.

Official preparation sources checked 2026-11-07:

- https://docs.aws.amazon.com/lightsail/latest/userguide/amazon-lightsail-container-services.html
- https://docs.aws.amazon.com/lightsail/latest/userguide/amazon-lightsail-creating-container-services.html
- https://aws.amazon.com/lightsail/pricing/
- https://developers.cloudflare.com/containers/configuration/outbound-traffic/
- https://developers.cloudflare.com/containers/faq/

### Documentation integration note

Public docs owners should describe the topology control and actual authority IPC
slice separately. Full Rust Gate/TLS/capture, real browser human/a11y controls,
independent witness key/state, physical crash recovery and provider profiles
remain release blockers until their own measured evidence lands. No docs/**
files are edited by this lane.

## Official sources, checked 2026-11-07

- D1 https://docs.docker.com/reference/compose-file/networks/
- D2 https://docs.docker.com/compose/how-tos/startup-order/
- D3 https://docs.docker.com/compose/how-tos/use-secrets/
- D4 https://docs.docker.com/reference/compose-file/services/
- C1 https://developers.cloudflare.com/containers/concepts/architecture/
- C2 https://developers.cloudflare.com/containers/faq/
- C3 https://developers.cloudflare.com/containers/guides/snapshots/
- C4 https://developers.cloudflare.com/containers/guides/local-dev/
- C5 https://developers.cloudflare.com/containers/configuration/outbound-traffic/
- C6 https://developers.cloudflare.com/workers/runtime-apis/request/ ; https://developers.cloudflare.com/fundamentals/reference/http-request-headers/
- C7 https://developers.cloudflare.com/containers/platform/pricing/ ; https://developers.cloudflare.com/containers/platform/limits/
- T1 https://developers.cloudflare.com/tunnel/get-started/
- T2 https://developers.cloudflare.com/tunnel/reference/origin-parameters/
- T3 https://developers.cloudflare.com/cloudflare-one/access-controls/service-credentials/service-tokens/ ; https://developers.cloudflare.com/tunnel/reference/tunnel-tokens/
- T4 https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/
- B1 https://developers.cloudflare.com/browser-run/get-started/
- B2 https://developers.cloudflare.com/browser-run/features/guardrails/ ; https://developers.cloudflare.com/browser-run/features/outbound-workers/
- B3 https://developers.cloudflare.com/browser-run/reference/automatic-request-headers/
- B4 https://developers.cloudflare.com/browser-run/faq/ ; https://developers.cloudflare.com/browser-run/pricing/ ; https://developers.cloudflare.com/browser-run/limits/
