---
id: plan:root/20261107061820
title: Runtime application operational integration
status: active
created: 2026-11-07T06:18:20Z
updated: 2026-11-07T06:18:20Z
rel:
  governs: [crates/runtime/**]
  depends_on: [sot:root/runtime-application]
---

# Runtime application operational integration

## Goal

Complete the assigned runtime follow-through while tracking every missing
responsibility. A native/session or proxy slice does not complete this plan.
Rust1.99.1/edition2024 only; scoring freeze, accessibility, defense-only lab,
dual-control and missing-required fail-closed remain invariant.

## Steps

### RUNTIME-SLICE-02 context design v0 — module map accepted

**Brief.** Consolidate six accepted responsibilities into one public Runtime
package while preserving existing authority domains. This is read-only design
against current source, not permission to move/import/edit other owners' files.
Root sequences atomic graph/path integration and obtains Audit/Authority/Network/
Native reviews. No detection, wire, lifecycle, custody or public policy change.
Root accepted the exact six-context module map below and the requirement to
restrict all98 matched crate-visibility lines to their original owning context.
This accepts design preparation only: no source move/import edit, constructor
implementation, compiler run, worktree or boundary qualification is authorized.
Root sequences the lower platform cluster first. Owner dispositions and actual
same-crate sibling mint-denial proof remain prerequisites for Runtime migration.
The attached runtime-slice-02-context-inventory.json lists EVERY matched
pub(crate) declaration/field line in the six src trees, cross-context crate-name
references, manifest/root hashes and fixture/target anchors. It is a source-bound
inventory, not a compile or complete semantic-access proof.

#### Accepted exact module map

Use crates/sdk/humanymous-runtime/src/lib.rs as the deliberate six-context root:
gateway, authority, audit, capture, network and native_security. Prefer gateway
for the old Runtime composition tree: src/gateway/mod.rs receives old
crates/runtime/src/lib.rs, and its existing application.rs, service.rs, startup.rs,
configuration.rs, session.rs, http.rs, settings.rs, management/, browser.rs and
tls.rs stay children. Thus gateway::application::Application and
gateway::service::ServiceHost remain coherent names without a new application/
application nesting or crate-name/runtime repetition. Root accepted this exact
gateway context name; its implementation/path freeze is still separately owned.

Each other old src/lib.rs becomes src/<context>/mod.rs and its existing subtree
remains below that context. Native-security's identifier is native_security.
Root exports the six intentional contexts; do not flatten all six old root
exports or add aliases recreating the old package graph. Any curated root API
needs an explicit supported API decision, not a compatibility shim.

#### Root collisions and directional dependencies

Old public root names collide: gateway/authority configuration and startup,
gateway/audit control, gateway/network http, gateway/capture/network tls,
capture/network http2, and audit/native_security storage. Integration test target
names and fixture roots collide separately. Preserve
context qualification for EVERY old crate:: path, including grouped crate::{...},
intra-doc links and cfg(test) helpers. Examples: gateway crate::tests::config ->
crate::gateway::tests::config; network crate::history -> crate::network::history;
audit crate::control -> crate::audit::control; capture crate::http2 ->
crate::capture::http2. Rejection is owned by authority; use its canonical type
and never create a second nominal Rejection during merging. Old crate-qualified
cross-context APIs become crate::<owner>::..., not the caller's context.

Allowed direction is gateway -> authority/audit/capture/network/core;
authority -> audit/network/native_security/core; capture -> network;
network -> core; audit -> native_security/core; native_security -> platform for
approved Windows primitives. Runtime as a package depends on core and platform.
Platform contains native primitives/reusable testing support and must NOT depend
on Runtime, Authority, Audit or Gateway. Test-support moves to an explicit
platform testing capability as a dev/test dependency, never a production proof
factory. Core remains independent of Runtime/platform/browser. No inverse
Authority -> Gateway Ready dependency or new scoring copy is allowed.

#### All widened crate visibility and custody dispositions

Every pub(crate) inventory entry must default to pub(in crate::<owning_context>)
or a narrower private/pub(super) scope after relocation, including type aliases,
reexports, tuple fields and methods. Do not change only type declarations and
leave writable fields crate-wide. Preserve existing private modules and ancestor
restrictions as well; do not publish modules merely to fix compiler errors.

P0: audit::keys::vault::ErasurePermit::issued is a capability mint and must stay
audit-only. audit::control::policy::Authorization's proposal/digest/administrators
fields must stay audit-only, otherwise a sibling could forge signed approval
data. Vault::authorizes/bound_to, retention accepts/matches, PrivateState open/
commit/promote/read, ExclusiveFile acquire/duplicate, custody::permissions and
framing/frontier/record bounded helpers all retain the original Audit domain.
Private custody/key/retained-ledger fields must remain private; merging does not
authorize a generic serializer, constructor, Clone or testing bypass.

Authority's protocol::framing Head/read and transport Stream/connect/reexports
stay authority-only; Head's public fields do not justify broader parent access.
The private empty NativeProof and private fields of preparation/session/failure
remain exactly uninhabited. observer::state::Owner and restricted gate methods
remain pub(super), with no sibling state constructor. SignatureCheckedAuthorization
and Expected DATA remain DATA, with no new conversion to NativeProof or Ready.

Gateway's Ack constructor/headers and Authorization.ack, Sessions browser/epoch
mutations, SharedAudit tuple constructor/field, sealed audit history/scope/owner
helpers, management operator credential accessor/commands, settings apply,
captured TLS parts and response/stream helpers stay gateway-only. Retained owner,
AuthorityGuard, RuntimeActivationFailure and any final Ready receipt remain
private to gateway::service; test-only epoch/temporal models stay cfg(test).
Network's OrderedTime new/check/commit and is_grease stay network-only, preventing
sibling mutation/reinterpretation of temporal completeness. Native-security's
service-name policy helper stays native_security-only. Capture has no current
pub(crate) match; its pub(super) RecordBoundary/headers decoder and private actual
socket/handshake fields still require preservation, never a public DTO constructor.

#### Public type and target/feature contracts

Gateway public Configuration exposes Authority Scope/Peer/Required and Audit
Checkpoint; ServiceHost constructors expose TailRecovery, ExternalWitness,
ManagementPolicy/SettingsPolicy and rustls::ServerConfig. CapturedRequest borrows
Capture Connection; gate/preparation/failure and key custody/retained-control
types have deliberate owner APIs. The attachment includes all121 current
crate-name cross-reference lines as a review superset, not just use statements.
Resolve these to the single owning context's nominal type. External CLI/adapters/
facade consumers require their own Root-assigned updates; do not widen internal
fields so they compile. Public data types being constructible today does not make
them capabilities after consolidation.

No current six-package manifest declares a default feature set. Preserve default
behavior and target cfgs explicitly; do not gate away currently default public
contexts or enable pilots silently. Proposed nondefault capture-hosted-pilot and
native-credential-pilot replace capture hosted-pilot and native credential-pilot
only if Root approves those exact names and updates required-features/cfgs.
Windows-only bridge/keyring/winsafe/interprocess/widestring/permissions and Unix
rustix features retain their target restrictions. Audit OS custody remains
cfg(any(unix,windows)); volume stays Windows-only. Match pinned versions and
crypto feature tuples; no dependency upgrade or unsupported WASM Runtime promise.
All-features may include actual pilot code, never a proof-minting feature.

Root must resolve four existing binary targets before moves: runtime/authority
src/main.rs, audit-witness and audit-control-retainer. Preserve exact installed
binary names and private stdin/stdout/watchdog behavior with explicit target
entries; do not accidentally replace both mains with one Cargo auto target.
Root/CLI determine final application placement. Tests referencing CARGO_BIN_EXE
must keep the real binary targets available until that migration is accepted.

#### Fixture and test-root migration

Use context-qualified integration target roots tests/gateway, tests/authority,
tests/audit, tests/capture and tests/network, with explicit unique Cargo test target
names. Runtime and Authority both have native_process.rs; blindly combining them
loses one target. Keep fixture directories under their source context and exact
bytes: signed authorization minimal/maximum, Audit canonical/Unicode/keys/Merkle/
checkpoint/verification corpora, Network TLS/H2/report/monitor fixtures and the
separately owned collector fixture. Update relative include_str roots and Audit's
CARGO_MANIFEST_DIR/tests/fixtures/records.json anchor. Authority decisions also
references core's frozen scoring corpus. No fixture regeneration, duplicate
signing encoder or copied collector contract is authorized for path repair.

#### Adversarial review and acceptance gates

Applying the blue/red/SRE/privacy/evaluator lenses yields NEEDS-WORK for actual
implementation freeze, despite acceptance of the module map and visibility rule.
P0 approval/erasure minting through widened crate scope is
resolved only by explicit contextual visibility plus owner review and negative
compile-access checks. P1 root/type/target collisions and feature/fixture drift
need a frozen per-path map and exact inventory, not compiler-driven pub widening.
P1 over-claim is prevented by fresh source-bound consumer tests: prior core162,
collector WASM or old Runtime6 passes do not qualify this merged graph.

Root obtains owning Audit/Authority/Network/Native dispositions against the exact
inventory, then freezes names, visibility exceptions (default NONE), bin/test
targets and dependency tuple. After implementation, fmt FIRST, all-target default
and all-feature check/strict Clippy, rustdoc links and exact per-platform test
inventories precede pure execution. Include external sibling compile-fail cases
for erasure/approval minting and NativeProof/Ready construction; actual OS custody,
process/IPC/TLS/W and Docker-only product tests remain separately allocated.
No new compiler/native/provider/pilot action was performed for this design.

#### Owner disposition questions — routed through Root

Audit: confirm each of42 matched visibility lines remains audit-context-only,
including ErasurePermit::issued, Authorization's three writable fields,
PrivateState/ExclusiveFile/custody helpers and cfg(test) duplicate-description.
Identify any genuine cross-context need; default exception is NONE. Specify
actual SAME-CRATE sibling negative probes for forged approval/erasure construction
and positive controls showing the legitimate signed dual-control path still
works. External integration tests and rustdoc compile-fail examples run as other
crates, so they would already reject pub(crate) and cannot prove sibling denial.
Require expected privacy diagnostics against otherwise coherent source; an
unrelated compiler error cannot count as denial. Confirm custody/retained-head
APIs preserve original owner/deadline/coverage semantics and never expose keys.

Authority: confirm each of7 matched visibility lines remains authority-only
(framing Head/read and transport Stream/connect/reexports). Confirm private
NativeProof remains uninhabited, observer state::Owner and all private phase
fields cannot be created/accessed by gateway/audit/network/native_security siblings
even under all-features/test builds. Provide legitimate DATA verification and
gate inspection positive controls without a preparation/Ready factory. Confirm
the canonical Rejection and public nominal API path mapping, binary/transport
target contracts and exact signed-fixture locations; no gateway->Authority
RuntimeReadyReceipt import or hidden activation callback is permitted.

Network: confirm all5 visibility lines (OrderedTime type/new/check/commit and
is_grease) remain network-only, including indirect reach through public parent
modules. Confirm capture->network and gateway->network use existing public typed
producer outcomes without direct temporal clock mutation or completeness
promotion. Freeze public history/correlation/traffic/report type paths, score-
before-append/no-op/GC/configure semantics and four parity fixture roots. Identify
any needed test adapter as a retained test-only diagnostic, never a production
healthy/fleet/capture producer or scoring change.

Native/platform: confirm the native_security service-name helper remains
native_security-only and all process/token/store/handle fields retain actual
private-owner custody. Runtime depends on approved platform primitives; platform
must not depend on Runtime/Authority/Audit or expose a general proof constructor.
Confirm Windows bridge's exact approved unsafe wrapper scope and deny policy,
Unix rustix/Windows keyring-permission cfgs and nondefault credential pilot/test
support capabilities. Specify process/credential/retained-handle positive and
sibling-negative controls without local service/account changes. No all-features
testing capability may mint NativeProof, successful W/ACK/Ready or accepted
production process identity from DATA.

Gateway remains this owner's43 visibility lines. They will default to
pub(in crate::gateway) or narrower, with private service retained owners/Ready
and cfg(test) temporal/epoch models preserved. Root receives dispositions and
freezes exact source/visibility/test-target changes before any implementation.

### CORE-SLICE-01 Runtime consumer edit

Root explicitly authorized mechanical Rust consumer path changes under
crates/runtime/src and tests: humanymous_signals/behavior/model/scoring/integrity/
challenge/watermark become humanymous_core::<owning_context>. This overrides the
prior import hold only for these references. Eleven files contain16 applicable
references, all integrity/scoring/signals; the other four context names have no
matches. Exact before/after SHA256 and replacement counts are attached in
core-slice-01-runtime-consumer-paths.json under evidence:root/20261107061821.
No file move, manifest/lock edit, visibility, behavior or scoring change is made.
The source lane was reclaimed with narrow source/spec paths. Old-reference search
and git diff --check passed. No compiler, Clippy, tests or full CI ran; Root must
declare the relocated seven-context workspace coherent and allocate validation.
Prior qualification remains historical and does not qualify these new imports.
Root committed the exact11 delegated consumer source paths in
c67d07c0114bd04793410f82f7182de7bfa6df2c with follow-up evidence1e2b4b38.
Fresh current blob/SHA256 checks agree with those integrated bytes. The previous
source-commit mutex blocker is resolved; do not recommit the same source. This
owner's spec/evidence changes were not part of Root's source commit. Core162,
core Clippy and collector WASM checks do not qualify Runtime consumers; separate
bounded consumer validation remains not run and requires a Root allocation.

### Eight-package integration handoff — 2026-11-08

**Mission and constraints.** Preserve every remaining R01-R17 responsibility while
Root sequences the accepted eight-public-package baseline v0. Runtime's accepted
destination is crates/sdk/humanymous-runtime; this is not permission for this
owner to move paths, rename imports or edit the common graph independently.
Detection freeze, fail-closed required signals, dual-control, accessibility,
opaque authority and source-specific historical evidence remain binding.

**Done.** Current crates/runtime paths exactly match the path/SHA256/blob manifest
source-commitments.json in evidence:root/20261107061821. The attached
runtime-package-handoff.json binds its exact hash, current clean-tree comparison,
API/visibility/unsafe/dependency hazards and tests. Latest sourceaba7934d and
receipt7bdda110 qualify six Windows checked-DATA consumers only, not the full79
library inventory or native/Ready/product. CLI's consuming composition and its
separate source-bound receipt remain CLI-owned.

**In progress and blockers.** No dirty Runtime source, active compiler/native
process, target lease or path-moving operation exists at this handoff. Root must
freeze the precise incoming context topology, public API/import mapping and
visibility changes before package integration. Actual original issuer/store,
retained P/S/O identity, process-wide claim, W, bounded ACK/transport and final
same-owner Ready remain independent missing producers.

**Next actions.** Root integrates core first, then freezes runtime/platform paths
and hashes with the relevant owners. Audit every pub(crate) constructor/field
before contexts become siblings; preserve private/uninhabited proof and custody
boundaries. Update the two cross-package include_str fixture references without
changing owner bytes. Revalidate the resulting frozen graph and real boundaries;
do not inherit old qualification from unchanged leaf files alone.

**Verification.** Under a fresh Root allocation run cargo fmt -p
humanymous-runtime -- --check first, then cargo check/clippy -p
humanymous-runtime --all-features --all-targets --locked (Clippy: -- -D warnings).
Compare cargo test -p humanymous-runtime --all-features --lib --locked -- --list
against the new exact inventory before execution. Run source-selected meaningful
pure tests, then actual native_process and OS lifecycle pilots in separate
approved venues; product e2e remains Docker-only. Existing Windows79/Unix83 and
native_process1 are authored inventories, not new merged-source passes.

**Read first and out of scope.** Read AGENTS.md, .agents/lessons/HARD-WON.md,
structural-development SoT, Runtime SoT and attached handoff hazards. No publishing,
main merge, ref synchronization, compatibility shim, scoring change or native
authority manufacture is authorized by this handoff. Codex Runtime chat retains
residual operational context; Root owns the integration sequence.

| ID | Requirement / accepted contract | Owner / prerequisite | Current status and acceptance |
| --- | --- | --- | --- |
| R01 | Shared14-field browser codec + counter ACK | root integrity;53abb0b/fabed20 | Runtime consumes shared APIs and compiled at709b0e2; actual current-source browser/ACK join pending; no copied codec |
| R02 | Explicit cookie-free browser session/collect/protected profile | runtime;R01 and normal lane claim | Source implemented, latest source tests NOT_RUN; actual shared-client nativeTCP fixtures authored, hosted execution pending |
| R03 | Fresh collection provenance/coverage | root collector + runtime join | Contract pending; required.collector422; actual epoch/version/coverage vs server time/counter separation |
| R04 | Actual browser Fetch/one-flight/ACK/body/abort lifecycle | root collector + runtime server | Pure client validated by root; actual join pending; actual browser and human/keyboard/AT separate |
| R05 | Shared response-head/framing and fidelity | runtime | Buffered/stream head parsing and bounded chunk/EOF source wired; HTTP10 pure passedadb8e3ed; actual current-source fidelity/failure matrix pending |
| R06 | Explicit bounded response streaming | runtime;R05 | Native stream wired; in-memory stream4 plus profile pure passedadb8e3ed; actual current-source backpressure/abort/drain pending; browser ACK stream separate |
| R07 | Operational settings/state CAS/restore | runtime; audit lifecycle | Actual Unix IPC/witnessed CAS/capacity32/session invalidation/restart passed immutable27675561; native failure/crash matrix still pending; default hosts read-only |
| R08 | Bounded session/global abuse admission | runtime | Source global/session/capacity/serialized concurrency implemented; quota2 pure passed709b0e2; actual429/capacity fixtures pending; no client-fingerprint identity authority |
| R09 | Actual separate local management handler | authority transport + runtime; e346834/e8ddb4c | Management4 pure plus actual Unix observer/operator IPC/drain passed; Windows native management integration and new production process-epoch bootstrap profile remain pending |
| R10 | Signed destructive controls and state effects | audit issuer + runtime | Typed target/assessment compiled and2pure passedadb8e3ed; controller owner/admission/effect unwired; actual two-party/one-use/target/restart/failure gates pending |
| R11 | Accessible Pass recovery + shared Engine fusion | runtime; shared challenge/root browser | Not implemented; no motor/speed gates or confirmed-human training; real input/AT separate |
| R12 | External witness library injection + deployment transport | audit/runtime/Compose | Actual same-process durable witness injection/settings restore passed Unix; bounded authenticated production broker and independent deployment remain unqualified |
| R13 | Actual TLS/H1 original capture | runtime + narrowly approved network paths | Joined worker/Application/ServiceHost compiledadb8e3ed and TLS-profile pure passed; native buffered capability only; actual TLS pilot pending, no product qualification |
| R14 | Complete H2 frame/HPACK/per-stream capture and server | runtime/network;R13+binding | Required follow-up; no H1-to-H2/fullM2 success inference |
| R15 | ServiceHost private input/readiness/stop/drain/flush | runtime; CLI/shared until/audit flush | Native process plus Unix management drain/controller loss/custody release passed; new observer-loss atomic gate/production protected provider still pending; synchronous I/O limits explicit |
| R16 | Actual CLI/system-service connection | CLI/runtime;R15+boot credential provider | Pending; signed descriptor/artifact/nonce, actual OS lifecycle, no unattended claim before credentials |
| R17 | Full source-bound product qualification | root/Compose/wargame/delivery/runtime | Pending; Docker full topology + actual effect/audit + ordinary human ALLOW/keyboard/AT; component results separate |

Mandatory remaining subrequirements: HttpOnly/path-scoped-cookie per-target
prepare or trusted application identity; adapter0.2 one-use request/session
receipt; shared audit-owned sealed Log+Custody owner; genuinely bounded-memory
WAL/Merkle recovery; prepared-candidate destination-bound rotation; complete
authenticated machine API and origin bypass semantics without fake browser
telemetry. Existing native0.1/adapter0.1 wire compatibility is retained.

Execution order agreed across chats: audit checkpoint→authority common client/
generic transport→audit two stdout-accounting test fixes→runtime. Root owns
shared Cargo/lock/toolchain and collector/integrity. This lane never edits those
files. Network expansion needs the separately approved narrow claim after
binding agreement; no existing parser/report/scoring edits.

Before each implementation tranche, update owning SoT/API status and integration
request. Run only authorized scoped checks, actual boundary pilot, then Docker
product gates in their qualified environment. Preserve raw failed/partial runs
privately and public sanitized source commitments through Specfold attach.
Compare LF working bytes against staged Git blobs; distinguish dirty source from
canonical commit. Commit only owned paths with git mutex and attribution.

## Record

- 2026-11-07: reread program-roadmap1.2, actual runtime sealed startup, collector,
  authority, audit and Go read-only oracles; applied coordinate-sessions,
  implement-sot-slice, design-sot/adversarial-critique. Three read-only reviewers
  returned browser/proxy-Pass/service-admin findings; no subagent wrote files.
- Shared native whole-header signer cannot be used by Fetch. Root accepted and
  implemented distinct browser codec53abb0b and ACK/client state fabed20. Cookie-
  free initial profile is explicit; path-scoped-cookie compatibility remains
  required. Exact agreements are promoted into runtime-application v0.3 above.
- Authority e346834/e8ddb4c provides real shared client/LocalHandler/explicit
  drain; Windows25+release3/Linux22+UID1 are owner-reported component results,
  not runtime validation or whole product. Audit d001ee6 provides witness/control
  operations,5528d74 only fixes two helper stdout-accounting tests.
- Actual read-only baseline attempt at HEAD1ee2c928247aacc748682a3a71bca9ad519941ff:
  cargo test -p humanymous-runtime --locked exited1 because shared Cargo.lock
  required update. Actual tests0. No lock bypass/update by this lane. Root later
  refreshed shared resolution; that does not turn the failed attempt green.
- User requested memory-surge triage before implementation. New cargo/rustc/
  build/browser/Docker/global-spec/subagent execution is held until root resource
  reassignment. No owned running process/container/session exists; all three
  review agents completed. No Desktop/other-chat/whole-WSL kill was attempted.
- Following audit normal release, gate-edge/rust-runtime-followthrough was
  claimed with runtime and own spec paths only. Current work is contract/source
  only; heavy validation remains held. No new implementation is validated yet.
- Root preserved waiting history and R01-R17 under private evidence20261106194410
  runtime-waiting-contracts.json. This plan is the owning follow-through canon,
  not an assertion the waiting requirements were implemented.
- Source-only R02 checkpoint adds explicit browser route/session profiles,
  Origin/metadata/cookie guards, exact shared browser canonical verification,
  consumed-counter ACKs, preserved application Authorization, origin reserved-
  header stripping and server projection of empty browser report sessionId.
  Native0.1 strict report matching and whole-header signer remain unchanged.
  Required collector/TLS/H2 still422. Two native shared-client fixtures are
  authored but NOT_RUN; they do not constitute actual browser/human validation.
- An intermediate production-only cargo check -p humanymous-runtime --locked
  -j1 with CARGO_BUILD_JOBS=1 passed before the report-binding/test additions.
  No exact intermediate manifest was captured; this result is not attributed
  to the final checkpoint bytes. No latest-source tests/Clippy/runtime/Docker/
  browser were executed locally. Scoped formatter passed; public-only Specfold
  check has0errors/1existing unrelated binary warning. Private global scan held.
- Validation policy now references the single program-roadmap/delivery canon:
  local coding/fmt/bounded pure unit, Cargo concurrency1; actual process/port/
  IPC/ACL/SCM/browser/Docker/CrossOS on free standard hosted runners. Fixture
  human/AT/interactive/device/independent-authority claims remain separate.
  Do not duplicate policy orchestration or remove required gates by path before
  the shared impact classifier exists. R15 source follows this checkpoint.

R15 source checkpoint adds service::{read_configuration,ShutdownRequest,Readiness,
ExitReport,ServiceHost}. ServiceHost::bind authenticates existing sealed custody
and binds ingress before readiness; run takes watch::Receiver<Option<ShutdownRequest>>.
The serialized application stops accepting on observed shutdown, checks the watch
before origin connect/write, bounds active asynchronous drain, and flushes the
retained audit before reporting stopped. Closed controllers and incomplete drains
return errors. No detached workers are introduced. Synchronous private input,
custody recovery and WAL flush require supervisor deadlines; this source cannot
promise their preemption. Main reuses this lifecycle with legacy inherited pipes;
actual SCM/SIGTERM controller, management listener and unattended credentials are
still R09/R16 work. This implementation and all lifecycle tests are NOT_RUN.
External witness injection source contract (R12/R15): private configuration may
set audit.require_external_witness=true (default false preserves legacy startup).
Without injection, startup refuses before opening custody/WAL or consuming the
unlock credential. startup::ExternalWitness::new consumes Box<dyn
CheckpointWitness>, a broker-pinned expected public key and WitnessDurability;
ServiceHost::bind_with_witness consumes that value and calls the shared
Log::open_with_external_witness. There is no failure fallback to writer-only.
The client is Send + owned/'static, not required Sync; all audit/pipe exchanges
remain serialized. Reuse audit WitnessServiceClient::connect/from_ready and its
private frame codec. Broker owns authenticated handles/peer, actual I/O deadlines,
child lifetime and reaping. FileAndDirectory is required for the Linux product
isolation profile; FileSynced remains explicit namespace-unconfirmed component
policy. Readiness records injection and required durability, while independence
qualification remains false pending deployment evidence. Injection itself is not
independence evidence. New missing-injection refusal test is authored, NOT_RUN;
all new source compilation/runtime/witness integration tests are NOT_RUN.
R15 lifecycle budget follow-through: service::StopBudget validates nonzero drain,
nonzero supervisor reserve and nonzero manager deadline, checked addition and
strict drain+reserve < manager deadline; drain retains existing1..30000ms bound.
It returns the existing ShutdownRequest for wire/API compatibility. This is a
configuration relationship, not a timer able to interrupt synchronous fsync or
witness I/O. CLI/broker must enforce the manager deadline and supervise actual
blocking work/process reaping. ExitReport success explicitly records drained and
audit_flushed only after those events complete. Closed controller, drain deadline,
audit flush failure and force-kill remain failures, never synthesized Stopped.
Admission429 and pre-response error writes must observe shutdown and drop pending
I/O immediately; they cannot extend a short drain to the ordinary connection
budget. Critique: equality/overflow budgets are rejected; positive typed fields
must not imply physical I/O preemption; partial response cancellation closes the
connection and never retries origin. Existing ShutdownRequest::new remains a
bounded component API without an OS supervisor-budget qualification claim.
- Explicit user goal set active for all R01-R17 on2026-11-07. Goal is incomplete;
  owned process/container count0. Next source tranches are lifecycle budgets and
  actual local management/dual-control joins, with external owners for neutral
  sealed audit startup, OS-protected unattended credentials, collector provenance
  and hosted product orchestration. No source-only checkpoint completes the goal.
- Lifecycle budget source and one pure unit fixture authored; compile/test NOT_RUN
  pending root's global Cargo slot. Added success fields drained/audit_flushed;
  unconsumed429/error writes now cancel on observed stop. Shared runtime APIs stay
  compatible. Current hosted42348518353/63f69ae cannot qualify these later edits.
R09 first real management slice: a separate filesystem UDS/local named-pipe
listener uses authority LocalHandler/LocalContext and serve_handler_until; no TCP
management listener or application-route forwarding. An independently generated
management credential uses authority Credential's exact constant-time Bearer
verifier and is provisioned only through an explicit trusted broker accessor,
never readiness JSON. Expected native UID/PID is rechecked in handler. Read-only
Observer role can POST only /management/v0.1/status with exact JSON five-field
installation/tenant/origin/revision/authority scope and management Host
humanymous.management.local. Reject duplicate headers, body >8192, non JSON,
Cookie/Origin/Referer/FetchMetadata/proxy headers and session/RIT credentials.
The bounded response contains scope and sanitized runtime capability snapshot,
not secrets/client data. No settings mutation, session reset, kill, erasure,
ban or key rotation exists in this slice; those remain refused pending exact
real signed permits and durable state effects. Management admission uses fixed
capacity8/rate20/timeout1000ms; stop uses validated service drain budget. Locality
is not admin authority. Runtime ingress continues to refuse reserved admin paths.
Critique reflected: transport peer filtering is not assumed; handler checks the
actual context peer. A credential is not imported by copying comparisons; shared
credentials owner supplies zeroized private provisioning next. Concurrent native
transport handlers read immutable snapshots and perform no detached work. Both
listener lifetimes must end before final audit flush; failure triggers sibling
shutdown and cannot release custody early or yield a successful ExitReport.
R09 source now implements the separate observer management listener and the
ServiceHost dual-listener join. ManagementPolicy consumes a separately provisioned
shared authority::Credential (generate or from_private_provisioning); scope,
role and native peer remain runtime policy, not token grammar claims. The runtime
never duplicates constant-time comparison or imports arbitrary bearer strings.
Both listener tasks are joined without detached runtime workers. This source and
two pure policy fixtures are NOT_RUN pending the allocated Cargo/hosted gates.
R07/R10 writes, durable state replay, independent credentials producer, actual
IPC/ACL/cancellation/lock-release pilots remain incomplete. Actual hosted run
42348518353 at63f69ae does not include these later changes.
R05 bounded complete-response framing extension: isolate origin response parsing
from request framing. Decode complete chunked entities with checked hex lengths,
checked decoded64KiB budget and explicit128KiB wire budget; reject TE+CL,
non-chunked/duplicate TE, malformed/truncated chunks and trailing data. Chunk
extensions and nonempty trailers remain explicitly unsupported and fail closed,
not silently dropped. Preserve duplicate ordinary headers/Set-Cookie and raw OWS;
strip hop/reserved gateway fields and reframe decoded entities with exact CL.
Bounded close-delimited complete bodies may be reframed after EOF; keep-alive
origin headers are allowed only with valid complete framing, never forward
Connection nominations. HEAD/204/304 remain bodyless;204 forbids CL/TE.
This complete-buffer implementation does not claim R06 incremental streaming,
full trailer support, original TLS/H2 provenance or browser body authentication.
Native/browser request codecs and frozen engine are unchanged. Pure malformed
framing and boundary fixtures precede hosted origin/abort/effect validation.
R05 source complete-buffer framing now decodes bounded simple chunked bodies and
reframes close-delimited EOF bodies, retains duplicate cookies, strips origin ACK
forgeries and rebuilds decoded CL. Chunk extensions/nonempty trailers/upgrade
remain explicit unsupported refusals; streaming and informational responses are
not qualified. Three new pure malformed/boundary fixtures authored NOT_RUN.
Hosted-only Unix service fixtures2 now exercise real shared IPC status with
credential/scope negatives, custody exclusion until stop, both listener cleanup,
explicit drained/flushed outcome and controller-close failure. They use actual
sealed/WAL implementations with synthetic fixture keys, no external witness;
this is not independent witness/product/human evidence. Never execute these via
a local broad cargo-test command. No current-source tests have executed.
Public spec check briefly reports duplicate external plan id20261107072500 in
native-abi/network-capture; root notified, owner files left untouched.
R08 authenticated-session admission: bounded session/global admission is resource
control, never a fingerprint/identity/scoring tell. Add validated private setting
session_rate_per_second (default100, range1..10000). Each volatile session owns one
fixed monotonic one-second window; no global per-client-key map or live eviction.
A valid next RIT is verified and consumed before session admission, preserving
browser ACK progression on429. Invalid signatures/replays never debit a victim's
session quota. Session-bound429 occurs before report mutation/scoring/audit/origin;
consumed browser responses retain actual429 ACK. No verdict/rule/weight changes.
Window reset uses elapsed monotonic duration, rejects rollback at the pure seam,
never extends a request deadline and cannot revive expired sessions. Global
pre-auth admission remains shared Admission; capacity rejects new sessions while
live epochs remain intact and expired entries are pruned. Serialized ingress
bounds concurrency1; no new throughput or identity flood-resistance claim.
Pure quota/boundary/rollback tests and shared-client429counter tests are required;
actual global/concurrent/socket origin-effect tests remain hosted-only.
R08 source adds per-session monotonic fixed windows with bounded validated config,
after next-token verification/consumption. Two pure window fixtures plus one
hosted-only native TCP/shared-client fixture now cover quota boundary/rollback,
invalid-proof non-debit, actual429 ACK progression, replay refusal and no origin
accept. This fixture is resource timing/protocol evidence, not human speed/motor
classification. Source compilation and all fixtures remain NOT_RUN; no local
native process or Cargo execution. Required missing collector/TLS/H2 still422.
R06 explicit native bounded-stream profile: route.response_profile defaults to
buffered; native_stream opts in to incremental entity forwarding with1MiB total
entity cap,8KiB copy buffer and existing exact signed request/connection deadline.
BrowserAuthOnly+native_stream configuration is refused: browser full-body/ACK
contract is unchanged. Shared response-head validation/header stripping is reused.
Content-length streams preserve exact length; chunked/EOF origins are decoded and
re-encoded as downstream chunked, emitting final zero chunk only after successful
completion, so premature close is detectable. Malformed size/CRLF/truncation,
extensions/nonempty trailers, excessive bytes and unsupported upgrades fail closed;
after the response starts, close without appending503 or retrying origin. Known
length over cap is refused before downstream head. HEAD/204/304 stay bodyless.
Backpressure is awaited, no unbounded queue/detached producer. Stop/drain cancels
or bounds active streaming with the same custody lifetime and no new origin after
observed stop. This is request integrity, not response-body authentication or
infinite SSE availability. Required collector/TLS/H2 remain independent422.
R06 source now wires route.response_profile=native_stream through the actual
connection origin branch after audited ALLOW, client liveness and exact deadline
checks. Browser routes cannot select it. The stream reuses response-head and
chunk-size validators,8KiB buffers and1MiB entity cap; no detached producers or
retry. Three in-memory async decoder fixtures plus one pure config-profile
fixture are authored. A hosted-only TCP fixture gates origin completion on proof
that the client already received its first SSE event; synthetic client report
and mock audit ordering are explicitly fixture controls, not durable/product
qualification. All new source compile/Clippy/tests remain NOT_RUN. Root's earlier
shared Clippy observed9deadcode errors while this wiring was WIP; no suppression
was added. Current wiring references every stream helper. Source requires a new
exact hosted/pure validation result, not attribution of the earlier failure to
success. Actual abort/backpressure/deadline/drain/malformed socket pilots remain
required. Global Cargo slots A/B stay root-assigned; no local Cargo started.
R05 connection nominations: parse bounded comma-separated Connection tokens with
existing HTTP token grammar, reject empty/invalid nominations and nominations of
framing/Host/Trailer authority, then strip nominated response fields case-
insensitively in the shared encoder for buffered and native stream profiles.
Static hop fields include Connection/Keep-Alive/TE/Proxy-Connection and proxy
credentials; origin reserved gateway metadata is always stripped. Ordinary
nonnominated duplicate Set-Cookie and application fields retain original bytes.
Normative references: RFC9112 message framing and RFC9110 section7.6.1 Connection.
https://www.rfc-editor.org/rfc/rfc9112.html
https://www.rfc-editor.org/rfc/rfc9110.html#section-7.6.1
R07 read-only operational view: expose a typed bounded OperationalSettings snapshot
(session capacity/TTL, connection deadline, global and session resource rates),
separate from scoring policy/scope/secrets. Configuration validation reuses this
one validator; status readiness includes this nonsecret snapshot. Native Observer
may POST exact /management/v0.1/settings with the same exact scope envelope and
credential/native-only rules as status. Response is explicitly readonly; no CAS
write or durability claim until actual private snapshot/audit replay service is
available. All mutation methods/paths and scoring weight/threshold/rule/band fields
remain refused. Audit PrivateState is currently crate-private; request its owning
API instead of copying native permissions, locks, atomic storage or cryptography.
R05 response nominations source adds case-insensitive Connection-field stripping
in the shared encoder (buffered+native stream); protected framing/authority
nominations fail closed. One pure fixture authored NOT_RUN. R07 actual native
Observer settings route now returns a nonsecret OperationalSettings snapshot,
with one shared resource validator used at configuration startup. Two pure
settings-schema/route fixtures and an extension of the hosted Unix IPC fixture
are authored NOT_RUN. Frozen scoring and secret fields are absent/refused;
settings_writable remains false. Full CAS apply/durable replay/recovery is still
required and depends on an audit-owned private snapshot service, not copied OS
storage. No current-source Cargo/native process/IPC/browser execution occurred.
R12/R15 shared sealed owner migration: audit29ee901 owns SealedLog custody+WAL+
witness lifetime and all key loading/recovery logic. Runtime startup retains only
its compatible public SealedAudit adapter and Configuration/Peer mapping, calls
SealedLog::open with UnlockCredential::Utf8 and PinnedWitness, and never extracts
mutable Log/Custody/keys. Existing authority private-directory checks still bind
the explicit configured Peer UID/SID before secret consumption; shared audit
checks its own actual custody owner. Required missing witness refuses first.
ExternalWitness delegates key pinning to PinnedWitness; no copied pin validator.
Public readiness and witness durability/independence distinctions are unchanged.
Explicit recovered startup requires a supplied witness plus independently pinned
head and TailRecovery choice; no implicit history/key provisioning, effect retry
or tail erasure. Actual recovery coverage/ambiguous tail is reported separately
and does not qualify full witnessed product state. Audit shared pure6/Clippy
results are producer scope only; runtime adoption compile/hosted tests are pending.
Shared startup migration now consumes audit29ee901 SealedLog/PinnedWitness and
removes runtime-owned key loading, HMAC/signing seed composition, WAL creation
and custody ownership logic. Public compatibility wrappers retain configured
native Peer binding and missing-witness-first refusal. ServiceHost::bind_recovered
requires an explicit witness/TailRecovery and independently retained trusted head;
startup rejects missing head or history-provisioning mode before reading state
or consuming unlock. Readiness reports bounded startup recovery coverage facts,
not arbitrary pin/key data or completed effects. One pure missing-pin refusal
fixture uses a rejection-only witness whose I/O methods panic if invoked; no
synthetic successful witness is claimed. Runtime adoption compile/hosted tests
remain NOT_RUN; shared audit pure6/Clippy results remain producer scope.

Git coordination incident record: a failed claim raced audit29ee901 and the old
PowerShell batch continued to owned runtime1a9c48a because it lacked an immediate
native exit-code guard. Verified history audit29ee901→runtime1a9c48a→root82df720;
shared index empty, no index.lock, git-ops free. Audit owner independently confirmed
its source commit/files preserved and no observed loss. No reset/restore/unstage/
amend/force action was taken. All subsequent writes must check successful claim
exit AND actual git-ops session before add/commit, and release only that session.
Historical hosted evidence recovered2026-11-07: run42348518353 at exact source
63f69ae9de703939b7b9091e2f215b12bea265de completed FAILURE,16jobs:3success/13failure.
MSRV job128338631933 explicitly executed runtime15tests with15passed/0failed/
0ignored/0filtered, including the native shared-client ACK fixture. That source
predates later management/stream/admission/shared-owner work. These are native
fixture/component results with client-asserted report and mock audit, not actual
browser/human or full witnessed product evidence. Latest source remains NOT_RUN.
Private captured CLI stdout (not original HTTP response bytes) is attached under
evidence:root/20261107061820; sanitized exact-source summary is attached under
public evidence:root/20261107061821. Other failed native probe details remain
unrecovered; no success is inferred. No rerun/dispatch occurred.
R06 adds one pure in-memory AsyncRead/AsyncWrite flow-control fixture: stalled or
broken downstream after the head must stop further origin consumption within a
bounded read-ahead window rather than buffer the1MiB origin. The model uses no
sockets/OS processes and does not qualify actual network abort/drain behavior.
New fixture execution remains NOT_RUN. Actual deployed resource monitoring,
malformed socket, force-stop and independent custody/witness pilots remain open.
R15 failure accounting: attempt the final audit flush after all owned listener
work has ended even if drain returned an error. Never convert either failure to
Stopped. service::ExitFailure reports bounded Drain/AuditFlush/DrainAndAuditFlush
stage and independently observed drained/audit_flushed facts, without raw paths
or secret-bearing source chains. ServiceHost::run_report returns that typed
outcome; existing run preserves io::Result compatibility by wrapping ExitFailure.
Synchronous flush remains supervisor-bounded, not watch-preemptible. An unreturned
flush/force-stop is unknown/incomplete, not an inferred successful flush.
R15 run_report source now attempts final flush after returned drain failure and
returns typed bounded ExitFailure phase/facts; existing run keeps io::Result.
A pure phase/leak fixture covers all returned error combinations. Hosted Unix
controller-close fixture asserts actual drain failure plus successful final flush,
not Stopped. Both remain NOT_RUN. Blocking flush/unknown supervisor force-stop
cannot produce a receipt. No current-source Cargo/runtime/process execution.
R02/R05 post-consumption origin errors: keep the consumed browser ACK signer in
the per-connection response state until dispatch/write ends, not only inside the
successful decision branch. If origin connect/read/framing fails before any client
response byte, the local error response carries the consumed counter ACK bound
to its actual status. Pre-consumption errors/admission remain unsigned. After any
response byte has started, close without a second response. Deadline/abort/drop
still invalidates the client epoch if no complete ACK response can be delivered.
A503 ACK proves counter consumption only; an origin already dispatched may have
acted. Never infer zero effects or automatically retry from that ACK.
R02/R05 origin-error ACK source now retains the signer in outer connection state
and signs pre-response origin failures with the actual local status. A hosted-only
shared-client/TCP fixture increments the independent origin-effect counter before
sending a malformed response, verifies503ACK/counter progress and replay refusal,
and asserts the effect count remains1 rather than claiming zero effects. Client
report/audit are explicit fixtures; not actual browser/durable product evidence.
New fixture execution and current compile/Clippy remain NOT_RUN. No local native
process/Cargo execution; no origin retry or partial-response second write.
R07 typed CAS candidate model: OperationalSnapshot serializes fixed struct fields
in order protocol,scope,node,revision,settings with protocol
humanymous.operational/0.1. Scope reuses the exact five-field ManagementScope;
node is the configured audit node. Revision is an integer0..MAX_SAFE_INTEGER.
Snapshot decoding is bounded16KiB, strict unknown-field rejection, exact current
scope/node/policy revision and the shared resource-settings validator. A candidate
requires exact parent revision and checked revision+1; no scope mutation can be
smuggled through desired settings. Commitment is lowercase SHA256 of UTF8 domain
humanymous/runtime/operational-state/0.1 followed by one NUL byte then exact compact
serde struct JSON bytes. PreparedSettings is not Deserialize/Clone and freezes
those bytes plus parent/new commitments; it is a candidate, never an action permit
or durable commit receipt. Actual private store, authenticated journal commit,
apply/session invalidation and crash recovery remain required before exposing
writes. No scoring fields, secret credentials or implicit policy migration exist.
R07 pure scoped OperationalSnapshot/PreparedSettings candidate source is authored.
Persisted payload decoding requires exact compact canonical bytes and16KiB budget;
CAS validates parent, exact scope/node, resource settings and safe revision bounds.
Two pure fixtures cover canonical bytes/independent SHA256 vector, frozen candidate,
stale parent, scope mutation, forbidden scoring fields, noncanonical input and
revision exhaustion. The initial commitment vector is
 a022446c62c485f1cc18cf7a51c7f32dc7a753c68a39a105fe3484c57b61e867
(system SHA256 calculation, not a passed Rust test). All new execution NOT_RUN.
Candidates grant no effects. Read-only settings endpoints remain unchanged;
actual snapshot store/audit commit/apply/session invalidation/restart recovery
must still be wired. Audit has proposed TrustedSnapshot native API; canonical
source checkpoint and qualification are awaited before adoption claims.
R07 durable pipeline: SettingsStore owns audit TrustedSnapshot(namespace
runtime-settings,max64KiB) and stores a canonical semantic envelope with committed
and candidate OperationalSnapshots. Native .pending is a separate interrupted
atomic replacement. A dedicated runtime.settings.commit/v1 event uses request_id
settings:<canonical revision>, correlation_id exact candidate commitment,
tenant_id and config_version exact provisioned scope; audit stamps node/sequence.
The authenticated journal must start0 and advance contiguously, with canonical
32-byte hex commitments, exact tenant/node/policy binding. No values are hidden
in reason/risk fields. CreateNew is explicit and only when file+journal absent.
Pipeline: stage candidate+parent durably, audit commit, finalize private envelope,
then issue nonserializable CommittedSettings. Errors after staging poison the
owner; no retry or successful receipt. The actual caller must poison the serving
epoch on an ambiguous failure. Startup selects only the WAL-committed snapshot:
finalize audited candidate or discard only an uncommitted semantic candidate
whose parent still matches WAL. Missing/mismatched/rolled-back state refuses.
Explicit native pending recovery validates both typed envelopes/current-or-parent
journal binding, promotes exact inspected bytes under lock, then runs the same
semantic recovery. Default requires native pending absent. Policy requests native
durability explicitly; file-sync never implies directory durability. This source
uses authenticated SealedAudit records and a shared native store, not arbitrary
HTTP evidence. Operator route/apply/session invalidation/hosted failure pilots
remain separate required joins, with all writes refused until wired.
R07 source now uses actual audit TrustedSnapshot7dba59d:64KiB canonical semantic
envelope, explicit CreateNew/Existing, exact inspected native.pending recovery,
authenticated contiguous journal, candidate/WAL/finalize and opaque committed
receipt. Application consumes it to change resource fields only and clear live
sessions without reviving a poisoned clock/audit epoch. Shared global admission
rebuilds its resource limiter after a committed rate change. ServiceHost settings
bind restores before listener readiness and holds the native lock through drain/
flush. Operator HTTP/IPC mutation command path remains read-only until separately
wired; actual failure must poison the serving epoch. External-witness provenance
and current verified checkpoint coverage are mandatory; journal append flushes
before grant, and unwitnessed recovery-tail settings events refuse startup.

Two pure crash/journal fixtures and one hosted-only actual snapshot/WAL/custody/
same-process durable signer fixture are authored NOT_RUN. The latter stages and
commits, invalidates an old session, retains the signer's actual head through
writer restart, and restores revision1/resource values with no event replay. This
is not independent process/OS/organization qualification. A temporary shared
Witness key-holder derives only the synthetic writer public key; it never signs
or grants. The distinct durable signer performs actual signatures and fsync.

CLI d9ad5c7 observed runtime library dependency compile and Clippy PASS after4
runtime diagnostics were fixed. Its final9file manifest before/after is identical
SHA256d183d10df0f7b8297bb2a0251c87f95dc5b413c38cbf47ee18c58fafbd83642c.
No runtime unit/native test executed there. After the source freeze was released,
only hosted Unix fixture canonicalization changed among those9files; later root-
approved capture/challenge/rustls manifest edges are separate graph work. Do not
promote that dependency result to latest whole-runtime all-target qualification.

Root approved the manifest window: capture/challenge path edges and exact
rustls0.23.46 std/ring/tls12(defaultfalse) were written and frozen; root owns lock
resolution. No Cargo command, push or dispatch from this owner. Source8132117
actual run42355168729 is separate from these worktree changes. Its predecessor
42354873001/a26d startup_failure with0jobs/0tests remains a failed attempt.
R15 shared readiness identity: service::standalone_profile(&Configuration) is a
pure expected full host-profile builder for the standalone executable (no witness,
management, recovery or persisted settings). ServiceHost uses the same builder
with actual owned startup facts. Defender/lab must compare the complete result,
not drop strict identity/scope checks or duplicate optional-field defaults.
Configuration::readiness remains a capability/configuration view, not the full
host manifest. Expected profile construction is not listener/producer attestation.
Source813 Docker receipt26=0evaluation/19errors remains unchanged; readiness-field
mismatch is a source-proven incompatibility, not a recovered raw failure cause.
Readiness source fix: service::standalone_profile(&Configuration) and ServiceHost
now share service/readiness.rs. Pure schema-default/managed-difference fixture and
hosted actual standalone-host parity assertion are authored NOT_RUN. Defender
owner consumes the full result and preserves exact identity/capability checks;
no subset fallback or duplicated expected JSON. Root graph6500ec23 resolves the
approved38member dependency window; this owner ran no Cargo. Exact813 run is live
and remains separate from this source fix. Operational Rust source will freeze
for root-assigned compile/pure validation after this checkpoint.
Frozen709b0e2 local checkpoint: Root allocated pipeline A; locked all-features
all-targets no-run and Clippy passed; scoped pure27passed/0failed/0ignored.
Recursive454-file Rust/manifests/root Cargo+lock+toolchain closure stayed identical.
No native/process/socket/IPC/browser execution. Private captured outputs/manifests
are registered under evidence:root/20261107061820. A returned immediately.
Root lifted own source freeze afterwards, keeping manifests and Cargo frozen.

R13 next consumer source: tls::CapturedRequest now owns PendingHandshake and an
explicit cancel-and-join blocking worker, reuses the strict complete http::read
parser and retains original Connection evidence with the exact Request. Response
writer also consumes and joins. Only absent/http1.1 ALPN; no H2 or NativeStream
consumer, no required.raw_tls readiness transition. Source NOT_RUN. Capture owner
is providing check_h1_idle for actual same-socket FIN/RST/trailing-data observation
after audit; remaining() alone is insufficient. Actual hosted worker-cancel,
request-body/evidence and origin effects remain required. No manifest edits.

R13 consumer wiring WIP: ServiceHost::bind_tls carries injected ServerConfig
through management/listener lifecycle, with native buffered/H1-only validation
before custody opens. Same Application joins original Connection ClientHello to
shared network report/Engine, uses actual negotiated ALPN metadata, and checks
same-owner idle before decision/post-audit/post-origin-connect. Request and RIT
lifetimes bound origin response reads; no fallback/retry. TLS readiness is owned
transport capability, never product qualification. Plaintext rawTLS and all
collector/H2 prerequisites remain refused. Source NOT_RUN; capture check_h1_idle
is now present in its owner's WIP. No Cargo or manifests changed in this slice.
Execution policy steering: user authorized actual local Windows/WSL validation
for measured comparison with GitHub, superseding the earlier hosted-only blanket.
Root opened a benchmark quiet window: this owner has no retained slot or running
Cargo/native/runtime/model/Docker command and will start none until allocation.
Source/docs may continue; shared manifests/lock/toolchain remain Root-owned.
The new pure tls configuration rejection fixture changes provisional library
inventory to46 Unix/43 Windows, plus native_process1; CI owner notified. These
counts must be verified from exact-source --list, never inferred as executed.
R17 qualification-contract.json now supplies exact common43/Unix-extra3 library
names and native_process1 to CI owner, with per-OS expected counts, all-features,
locked, zeroignored/filtered and both-targets-required semantics. Source data is
provisional until exact-source --list agrees; no adaptive expected denominator.
Capture check_h1_idle source checkpoint is committed by its owner. Runtime
consumer will checkpoint source NOT_RUN during Root benchmark quiet window;
actual TLS fixture still needs approved dev-dependency graph window.
R07 management operator source WIP: distinct observer/operator credentials,
pre-custody role collision/missing-witness checks, native exactscope resource-only
commit envelope and bounded queue1. Both plain/TLS accept loops serialize commands
between requests. Actual LocalContext remains attached through stage/witness/
finalize/apply and is rechecked; failed ambiguous mutations poison/stop serving.
Receiver scope drops queued contexts before management drain; store remains held
through final audit flush. Shared observer profile reports committed resource
values/revision. Default hosts stay read-only and R10 destructive controls absent.
One pure role/native-envelope/no-scoring negative fixture and one actual Unix
IPC/witness/CAS/session invalidation/restart fixture authored NOT_RUN. Updated
source inventory common44+Unix-extra4=>Unix48/Win44 plus native_process1. No Cargo,
actual native execution, common manifest/lock/toolchain changes during quiet.

Operator fixture additionally exercises actual capacity:32 fresh sessions succeed
and the33rd rejects429 after the committed capacity change; old session rejects401.
Observer response remains writable=false even when operator_available=true. Scope
still excludes destructive controls. Direct service stop watch and monotonic
command wall-time checks supplement native LocalContext throughout commit/apply.
Exact813 historical qualification update: terminal run42355168729 FAILURE, but
MSRV job128359709010 SUCCESS explicitly executed runtime library41 and native
process1 with zero failed/ignored/filtered. Library includes actual Unix observer
management drain/controller-close plus native TCP ACK/stream/quota. This is old
813 source/default workspace feature graph, not current all-features/TLS/operator
qualification. Current Rust source remains5ee2e0c NOT_RUN in benchmark quiet.
Private captured stdout and public sanitized names/counts attached under evidence
20261107061820/20261107061821; original Ubuntu native prefix remains separate.
R10 owner contract accepted: fresh exact independently retained control state is
required at startup/admission. Covered kill=>killed; retained consumed intent with
no covered effect=>unavailable/pending, no auto replay or healthy fallback. Audit
owner is adding proof query; runtime has not enabled mutations. Pure KillTarget
source fixes protocol/install/node/killed canonical data and shared commitment,
with independent106byte vector and policyrevision/foreignscope tests NOT_RUN.
Shared Scope::validate requested rather than copying control identity rules.
This adds one pure name: library49Unix/45Windows (45common+4Unix), native_process1.
R10 typed assessment WIP consumes actual RetainedControlState (not JSON/bool),
pinned authority and actualSealedAudit coverage. It rejects other consumed kill
targets, contradictory/malformed records and all binding failures; preserves
Pending for spent intent without covered effect. Exact state/binding metadata is
nonClone/nondeserializable and no grant/effect method. One pure classification
matrix authored NOT_RUN; now46common+4Unix library50Unix/46Windows/native1.
ServiceHost issuer/query/admission serialization and mutation remain unwired;
no permanentkill capability claim. Depends audit owner's new query/Scope helper
source checkpoint, awaiting commit and post-quiet validation.
Audit producer query/Scope helper canonical9c47f8e4dfb8dcfe0dee4e3359bb3c5e573050d5
is committed. Runtime target/assessment consumer checkpoints against that source;
new producer/consumer compile and actual lifecycle tests remain NOT_RUN in quiet.

Allocated C local checkpoint at frozenadb8e3ed: Windows allfeatures/alltargets
locked no-run PASS36.74s; Clippy-Dwarnings PASS11.25s; reviewed pure31/0failed/
0ignored across14 invocations,613 intentional filtered occurrences. No actual
socket/process/IPC/TLS/browser execution or full-library claim.476-file before/
after changed Apache coordinator and audit integrationtest; Runtime/manifests/
lock/toolchain/checkeddependencyLIBsources unchanged, not fullworkspaceidentity.
Private logs/manifests and public local-adb8e3ed-validation-summary attached.
C returned immediately. Control-owner feature draft parked privately before
freezing; actual controller wiring follows only after assigned validation stage.

Current actual component proof: allocated nonadmin Windows46+1 and immutable
ordinary-UID WSL27675561 Unix50+1 allPASS0failed/ignored/filtered with exactcase
inventories. Each476source before/after matched; rawprivate and sanitizedpublic
native-adb8e3ed-validation-summary attached. Unix source newly proves operator
IPC/witnessedCAS/capacity/sessioninvalid/restart and existing custody/drain cases;
production independent witness/observer/SCM, actual TLS/H2/browser/human/AT and
full Docker product remain open. RuntimeRustunchanged; no heldvalidationprocess.

Root accepted R2 two independently SCM-owned S/O roles/distinct principals with
O=broker and ownedWchild, and R3 irreversible atomic observer-loss semantics.
OwnSoT667 records exactadmission/acceptedwork/deadline/finalflush rules and
separateobserver-status vs native-manager stop authorization. R1 concrete native
protectedgrantpublisher/discovery/processpins/crossSIDaccess and actualtyped
activation remain P0; no secretchannel code approval. Finite24h/no-renewal is a
proposal, not an accepted universaldefault. Runtime independently reviewed primary
Microsoft process/SCM/DPAPI/pipe boundaries and sent required deltas to owners.
