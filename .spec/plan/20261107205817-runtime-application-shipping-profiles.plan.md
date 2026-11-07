---
id: plan:root/20261107205817
title: Runtime public Core and Gate shipping profiles and replacement slices
status: active
created: 2026-11-07T20:58:17Z
updated: 2026-11-07T20:58:17Z
rel:
  depends_on: [sot:root/runtime-application, sot:root/structural-development, plan:root/20261107205157, plan:root/20261107061820]
---

# Brief

Replace the actual legacy Core demo and Gate edge roles with Rust compositions,
then remove active Go build/deployment dependencies after positive role gates.
Keep the accepted eight public packages and Runtime's six contexts. Runtime owns
gateway composition; CLI owns executable/container lifecycle, Browser owns the
demo/collector assets, and Delivery owns images/required jobs. This is DESIGN
ONLY during platform consolidation: no source/import/path/Cargo change, process,
SDK installation, Docker run, hosted dispatch, release or deployment. Existing
private-loopback configuration and scoring/authority/custody/Ready/a11y gates
remain unchanged. Root must canonize the explicit new public contracts below.

# Source and inventory limits

## Accepted first DATA parser implementation — source authored

Root accepted the exact five-field envelope in
plan:root/20261107211302 and authorized source/tests under current Runtime,
eventually gateway::shipping. This accepts only pure DATA, not the rest of C1-C8.
shipping::parse_public_role_data is the single parser with immutable role/scope/
binding/provider views and fixed redacted errors. Original input is capped at
32768 bytes before parsing. Typed object visitors enforce closed maps and decoded
duplicate keys without an overwriting JSON Value intermediary; positional arrays
and object-form role variants refuse. All lengths/grammars precede hex decoding.
The provider visitor has constant six-element capacity, rejects duplicates and
missing mandatory first-four declarations, and refuses the seventh before its
value is decoded or collected. H2/collector declarations waive no later rules.

Scope and generation/digest/epoch values remain assertions. There is no binding
comparison helper, generation reservation/freshness check, serialization/raw
Debug, I/O, clock, callback, secret/address/path reference or native/Ready factory.
Existing private Configuration and Cargo dependencies are unchanged. Source is
split by envelope/visitor, scope and binding responsibilities with meaningful
negative and positive unit groups. All authored implementation/tests are Rust.

Nineteen authored units cover both roles and preserved input, raw/decoded size
boundaries, schema/role aliases and trailing JSON, every missing/unknown/type
case, decoded escaped duplicates at each object level, positional arrays,
epoch/digest/generation bounds, required providers/duplicates/types/seventh seed,
fixed redacted errors and repeated assertions without replay authority. Exact
paths/SHA256 and test names are attached as public-role-data-source.json under
evidence:root/20261107061821. Source formatting and diff whitespace check passed;
At source checkpoint2e9934fe, allocated immutable Windows fmt FIRST/all-target
check/strict Clippy passed; the exact98 library inventory and actual19 selected
names matched. All19 passed with0failed/0ignored/79filtered in0.05s. Full3036
archive/snapshot members,6576 registry files,69 selected tools/shims,18 config
candidates and14 environment hashes stayed unchanged. Stopwatch elapsed111.22s
within390work/450total; target returned with no live process before bookkeeping.
See public-role-2e9934fe-validation-summary.json in the owning evidence.

Two positional-array negatives have a known isolation limitation: root input
also has incomplete nested data and binding input uses sorted-value rather than
declaration order. They do not independently prove map-only rejection. The scope
negative is grammar-valid and the map-only implementation was read back; no
parser defect was observed. Source remained frozen, with a separate test-only
strengthening grant requested. No hidden executed-phase retry occurred.
Windows98/Unix102 remain authored inventories, not full-suite execution; actual
native/public ingress/W/Ready/shipping and the remaining27 behaviors stay open.
Earlier leaf/core/platform proofs do not qualify the new integrated product.

### Positional-array isolation refinement — authored, not executed

After the original2e993 run and receipt e8f71220, Root separately authorized a
TEST-ONLY refinement of the existing scalar_types_and_positional_objects case.
It now constructs valid full root/scope/binding arrays in each actual wire
declaration order. Direct derived Envelope/Scope/Binding deserializer acceptance
controls and the matching valid-map control are asserted alongside public parser
refusal, to isolate map-only enforcement from unrelated value/type/missing-field
errors. On immutablebe9eec6d, allocated Windows fmt FIRST/check/strict Clippy
passed. Exact98 inventory matched; ONLY this exact case ran and passed with
0failed/0ignored/97filtered in0.01s. The three derived acceptance controls and
public map-only refusals were actually exercised. Full19 was not rerun.

Only shipping/tests/envelope.rs changes; the other nine recorded Runtime source/
inventory paths, production parser, private Configuration and Cargo are unchanged.
There are no new test names:19 shipping cases and Windows98/Unix102 inventories
remain. Exact before/after hashes are attached in
public-role-positional-refinement-source.json. Standalone formatting and diff
whitespace checks passed. Full3039 archive/snapshot members,6576 registry files,
69 tools/shims,18 configs and14 environment hashes stayed unchanged. Stopwatch
elapsed55.69s under390work/450total; target returned/live processes0 before
bookkeeping, no retries. See public-role-be9eec6d-validation-summary.json.
Original2e993's
19-pass receipt and its isolation limitation remain historical and unchanged;
they do not qualify this refinement or imply a failed parser implementation.

Read plan:root/20261107205157 and Delivery's shipping-role-source-map.json first.
The attached runtime-shipping-behavior-inventory.json in
evidence:root/20261107061821 binds186 observed source/deployment inputs,62 route/
admin registration lines,62 flag declaration lines, environment selectors and461
legacy test function names. These tests were NOT executed. The census includes
legacy cmd/server, cmd/gate, internal/gate and web assets, current Runtime seams,
capture interfaces and shipped deployment consumers. It is not an immutable
whole-workspace build or proof that every legacy test is applicable or reviewed.
Nested optional integrations outside these inputs require their owning inventory
before completeness can be claimed. Every enumerated selector must receive a
per-behavior disposition; optional does not mean disposable or silently ignored.

The pure SDK humanymous-core is not a server. The current humanymous-runtime
executable requires private nonterminal stdin/stdout, and Configuration requires
loopback listen/origin SocketAddrs. Its profile is experimental-loopback-http1;
TLS-H1 primitives do not establish public/browser/H2 serving. Existing management
is private UDS/named-pipe, observer read access plus a distinct resource operator,
not Gate's public TCP RBAC/admin console. No binary rename satisfies these gaps.

# Behavior inventory and required dispositions

P means a partial Rust primitive exists, not a shipped role. M means missing
composition/qualification. C means an explicit contract choice is required. No
whole behavior is marked equivalent by this plan; no retirement is proposed.

| ID | Legacy behavior and source | Rust gap / mandatory disposition | Slice |
| --- | --- | --- | --- |
| B01 | Public TLS Core :8443/:443, Gate :8444; cmd/server/main.go, cmd/gate/main.go | P loopback TLS-H1; M explicit public-role binds and certificate ownership | S1/S3 |
| B02 | BYO/ACME/domain/cache/email/directory, self-signed dev; cmd/server/acme.go, cmd/server/cert.go, cmd/gate/tls.go | C provisioned certificate first plus explicit automated-issuer provider contract; no production self-signed fallback | S3 |
| B03 | Original TLS ClientHello and H1/H2 capture; cmd/server/tlsconn.go, h1capture.go, h2capture.go, h2monitor.go | P Capture; M full original-owner H2 request/body/flow-control composition and browser ingress | S4 |
| B04 | Max connections, timeouts, body budgets, GC; both mains and internal/gate/config.go | P Runtime bounded sessions/connections/body; C finite public budgets vs legacy unlimited/disable selectors | S1/S4 |
| B05 | Core page/demo/static and Go WASM loader; cmd/server/server.go, handlers.go, build/core.Dockerfile | M Browser-built Rust/WASM assets and exact static manifest/HTTP integration; no Go loader in shipping image | S6 |
| B06 | Core session/collect/cookie/RIT/rotating seed; handlers.go, rit.go, collect_pipeline.go | P shared Rust RIT/counter ACK; C versioned endpoint/client migration from cookie legacy to accepted explicit browser subset | S2/S6 |
| B07 | Report single/list, CSP reports, resource/trace/traffic views; handlers.go, resource_handlers.go | P source data kernels; M scoped authorized handlers/storage/quotas/retention, no cross-session disclosure | S6 |
| B08 | PoW and Pass page/new/pow/attest/solve/KPI; pass_handler.go, pass_store.go, pass_proof.go, powhandler.go | P pure challenge; M accepted finite single-use/recovery/W interaction and accessible UI; no motor/speed gates | S7 |
| B09 | Core operator explain/counters/mlcorrect; ops.go, mlcorrect_ops.go | P frozen core trace; M separate operator route/auth and non-PII metrics projection | S5/S8 |
| B10 | Optional ML signed/pinned bundle/shadow/canary/traces; main.go, ml_setup.go, mlcorrect_ops.go | Core/Model-owned contracts; preserve explicit configured provider/refusal and no scoring promotion from untrusted labels | S8 |
| B11 | Optional loopback playground SSE/launch/nonce/abort and score receipts; playground.go, launch.go, external_input_receipt.go | CLI/Browser lab-only role, excluded from public surface by explicit profile; retain separately required lab replacement | S8 |
| B12 | Gate longest-prefix strict/balanced/attested/monitor/off route table; cmd/gate/routes.go, internal/gate/config.go | P exact Runtime route inventory only; C canonical route matching/preset expansion using frozen Core policy, never new thresholds | S2 |
| B13 | Control/admin split then ordered ban/framing/header/auth/token/upgrade/sweep/verdict gates; internal/gate/gate.go, gates.go | P request/audit/engine primitives; M exact ordered Gate composition and per-gate semantic parity | S2/S7 |
| B14 | Origin reverse proxy, fixed upstream/base URL, origin-auth, forwarded/header hygiene, errors; gate.go, guard.go, request.go | P loopback bounded forwarder; M explicit service origin set, TLS trust, origin isolation/authentication, cancellation | S1/S2 |
| B15 | HTML loader injection/CSP/encoding/size handling; inject.go, gate.go | M bounded response transform with explicit required-collector refusal, no opaque-pass-through bypass | S6 |
| B16 | Gate session/collect/stepup/loader/CSP, health/ready under /__hmn; control.go, control_collect.go | P shared session/collect; M role-specific routes, single-use stepup/recovery and owned loader | S2/S6/S7 |
| B17 | Verdict token rotation, replay/sticky verdict, upgrade/recon/sweep; token.go, gates.go, recon.go | P core logic where present; M issuer/epoch/replay/atomic control composition; all trust upgrades owner-approved | S7 |
| B18 | Temporary bans/rate and permanent/CIDR/lift/bulk; ban.go, admin.go | P rate/resource controls; M retained authoritative enforcement; permanent actions remain dual-control | S7/S8 |
| B19 | Admin read APIs: bans/integrity/proof/audit/incidents/policy/settings/approvals/erasures/whoami/metrics/keys/checkpoints/virtual-usb; admin.go | P Audit primitives/native diagnostics; M authenticated TCP RBAC, bounded projections and sanitized UI | S5/S8 |
| B20 | Admin settings proposals/dry-run/rollback, approve, kill, erasure/cancel; admin.go, approval.go, admin_settings.go | P resource CAS/control permits; M full retained dual-control/coverage/effect/recovery handlers; no bool authorization | S5/S8 |
| B21 | Admin console and roles auditor/operator/approver/DPO; console.go, adminauth.go | M Browser assets and explicit admin principal mapping; no embedded live tokens outside disposable lab | S5/S6 |
| B22 | Sealed keystore/WAL/verify, pseudonym vault and independent coverage; cmd/gate/main.go, internal/audit | P Audit custody/verification; M qualified startup/retained-control/recovery/actual W composition; no auto-create missing keys | S0/S8 |
| B23 | Redis fleet bans/sticky/correlation/rates; redisledger.go, redisratelimit.go, both mains | M actual owner-qualified coordinator; local degradation is explicit and cannot claim fleet completeness | S8 |
| B24 | Redis/ClickHouse audit projections and drop metrics; cmd/gate/main.go | Audit-owned bounded independent projections; outage cannot forge audit coverage or alter verdict | S8 |
| B25 | Web Bot Auth/PAT/WebAuthn trust and credential fanout; webbotauth.go, privacypass.go, webauthn.go | Core/Authority-owned actual verifier/issuer/epoch inputs; absent configured verifier refuses, never bypasses Pass | S7/S8 |
| B26 | PROXYv2 trusted balancer CIDRs; proxyproto.go, cmd/gate/main.go | Network-owned transport-level explicit peer allowlist; no HTTP forwarded-header identity or TLS provenance substitution | S4 |
| B27 | Operational logs/HSTS/metrics/build identity, signal drain; both mains/logging.go, compose.release.yaml | P stop budget/flush APIs; M CLI container signal adapter and role manifest, readiness revocation before drain | S0/S5/S9 |

Each family must expand into leaf route/flag/test dispositions before shipping;
the captured62 registrations and62 flags are the reconciliation inputs. A
configured unsupported option is an explicit startup refusal until implemented,
never success with ignored input. That refusal is an interim safety result, not
completion of a retained legacy feature or permission to delete its required job.

# Explicit configuration profile proposal

Keep existing private-loopback v1 documents and validators byte-semantically
unchanged. Introduce a NEW closed versioned public-role envelope, not a boolean
allow_non_loopback on Configuration. Proposed roles are core_demo_tls_v1 and
gate_edge_tls_v1; a separately selected disposable lab role may use local test
certificates/fixture secrets. Profile names/schema and compatibility must be
canonized by Root before implementation. Unknown/mixed/duplicate fields, oversized
documents, unresolved required providers and role-mismatched options refuse.

The new envelope separates installation identity, public ingress, exact served
authority/SNI policy, fixed origin policy, public control/static routes, isolated
admin plane, audit/custody/W, required evidence capabilities, finite budgets and
owned lifecycle. No secrets in argv/environment/query/log/readiness; CLI receives
the bounded nonsecret document plus protected existing secret/certificate handles
or private inherited input. No implicit auto-provision or current-user fallback.
Configuration DATA never becomes NativeProof, a W receipt or Runtime Ready.

For Core demo, no proxy origin is invented merely to reuse Gateway::Application:
compose local static/control/report/challenge handlers with the SAME shared
Engine/session/capture/Audit owners. Gate edge uses the same owners plus an
explicit origin effect adapter. Role manifests distinguish local handler effects
from upstream effects; no second scoring implementation or self-proxy loop.

## Public ingress and origin

The public bind is allowed only by the new explicit profile and installation-
provided concrete listener tuple. Local deployment may bind an unprivileged
container port and publish443; this does not bless arbitrary listeners in the old
private profile. Public data/admin listeners must be separate sockets and network
identities. Reserved control/admin/health paths never proxy through catch-all
routes. Host/authority/SNI mismatches, ambiguous framing and smuggling refuse
before any origin effect; no request chooses an upstream, DNS name or trust store.

Propose a finite OriginPolicy with scheme, exact service authority/base-path,
explicit address set and port, optional TLS server name and pinned trust bundle,
plus origin-auth key reference/epoch and total connect/request/response budgets.
Allow a non-loopback container origin only through this profile. For hostname
discovery, validate every resolved address against the installation-provided
service set at each new connection; do not re-resolve into an unrestricted peer,
follow redirects, accept URL credentials/fragments or honor request-controlled
forwarding metadata. Private/container address space is not itself authentication.
Require origin network isolation and authenticated gateway-to-origin requests;
Root must choose exact approved origin-auth wire and rotation semantics before
code. Never silently reuse client session/RIT/audit custody key material for it.

Production origin TLS verifies the configured name/trust; no skip-verify.
Explicit HTTP origin is eligible only on the accepted isolated deployment link
with independent origin authentication and documented trusted-network premise.
Do not claim it has end-to-end transport secrecy. No transparent origin retry on
ambiguous effect, partial write, audit/W loss, shutdown or recovery. Existing
request authorization/consumption -> verdict -> required durable audit -> effect
gate -> first connect/write order remains enforced, including after awaits.

Keep finite request/header/response/connection/stream/body/queue budgets, separate
from frozen scoring policy. Existing64KiB request and147456-byte origin frame are
not silently raised; proposed public budgets and large-upload/stream semantics
need explicit values and tests. Legacy unlimited max-body is not adopted by
default. Oversize input never reaches origin; post-consumption ACK reflects actual
status when deliverable, not a fabricated clean413 on an already reset connection.

## TLS/H2 and evidence provenance

Begin the implementation with installation-provided certificates and exact
private-key custody; no production fallback to auto-generated self-signed keys.
Automated renewal/ACME remains required behavior B02, implemented through an
accepted bounded provider/CLI certificate owner with protected cache/key storage,
explicit domain/directory/egress allowlist, renewal generations and failure facts.
External issuance may be an accepted topology choice, not silent retirement of
automatic certificate management. No paid ACME/provider action is authorized here.

TLS terminates at Runtime's owned ingress BEFORE HTTP normalization. L4 TCP
passthrough may preserve this; an external TLS-terminating proxy cannot supply
original-client TLS/H2 truth with X-Forwarded headers. Capture and dispatch bind
ClientHello, negotiated ALPN, connection epoch and H2 stream IDs to the actual
request owner through audit and effect. Require actual H2 SETTINGS/HPACK headers,
DATA/trailers/body integrity, multiplexing/flow control/stream reset/cancellation
and bounded concurrent budgets. Existing Capture metadata is not an H2 server or
body-digest provider. Required original_h2 remains422 until the real owning path
exists; do not advertise ALPN h2 and then parse it as H1. h2c/HTTP3 are disabled
unless separately accepted. Retained role parity needs actual H1 and H2 positives,
not a permanent required-H2 refusal labelled migration success.

PROXYv2, if selected, is parsed on the actual socket only for configured retained
L4 peers before TLS, with size/deadline/address-family checks. It cannot replace
actual peer verification or make TLS termination upstream appear original. No
generic trusted-forwarded-header option. HSTS is an explicit HTTPS deployment
policy, not enabled for disposable self-signed lab traffic by inference.

## Administration, health and readiness

Preserve private native management/observer-only status and separate resource
operator APIs. Add a NEW admin TCP/TLS adapter for Gate role RBAC; do not broaden
ManagementPolicy.peer or reuse observer/session credentials as admin principals.
Propose dedicated internal admin network bind with mTLS AND separately mapped
application roles, independent auditable actor identities and request/replay
budgets. Root/Authority must approve exact credentials/RBAC/SSO exchange and
required unauthenticated denial statuses. Host-loopback published ports do not
restrict other containers on a shared network; public ingress/admin cross-access
must be denied by handler and deployment topology. No live token embedded in a
production console, URL or browser static artifact. UI is secret-free until login.

Map every admin read/mutation in the source inventory, preserving role checks,
scope, authorization-before-read/effect, bounded pagination/redaction, meta-audit,
resource CAS and dual-control. Erasure, kill and permanent/CIDR bans require the
actual retained signed second-approver permit and covered effect state; no direct
endpoint-to-ErasurePermit factory, operator-only switch or erased-key success
claim without evidence. Missing provider is explicit unavailable, not a cosmetic
success or removal of the corresponding required behavior gate.

Public liveness is a minimal process response; public readiness exposes no keys,
session data or grant details and returns503 whenever actual required owners are
not active. CLI owns signals and external watchdog; Runtime first closes Ready/
admission under the same irreversible gate, then bounded drains accepted work,
joins workers/monitors and records separate audit flush/cleanup outcomes. Final
Ready still requires the opaque native preparation, actual credential ACK and
private retained resource owner where the profile requires them. No flag may
declare a missing native/container provider present. Root must define an actual
container admission/identity/W trust provider or an explicitly different accepted
profile with equivalent reviewed controls; a container label/PID/env bool is not
SCM/native proof. Unknown final effect/cleanup never becomes normal stopped.

## Browser, Gate policy and optional features

Root/Browser/Collector choose versioned role routes and loader protocol. Core's
legacy /api/session, /api/collect and Gate's /__hmn endpoints cannot silently use
different cookie/seed/RIT contracts. Preserve accepted cookie-free browser subset,
fresh collector provenance and status ACK semantics; update actual owned clients
and assets together. Unavailable required collector stays422, not client JSON
being promoted to coverage. Static assets are build-manifest-bound Rust/WASM/
approved generated glue, with traversal/MIME/cache/CSP/integrity controls and no
Go WASM runtime shim. No query label or public Pass result is confirmed-human
training truth. Pass uses the existing pending accessible integration plan;
finite single-use deadlines and keyboard/switch/AT paths precede public reachability.

Gate's longest-prefix presets and ordered gates need an explicit shared Core
policy/route contract. Do not implement new weights, thresholds, hard predicates,
verdict bands or trust upgrades while translating Go presets. Monitoring cannot
turn malformed required input, missing custody or absent Ready into admitted
origin work. WebSocket/upgrade, injection/encoding/CSP, credential verifiers,
fleet Redis and ML provider behavior remain named required slices when selected.
Unsupported inputs refuse configuration until implemented; no generic pass-through
fallback or perpetual optionalization. Log-only observers/projections stay outside
the decision authority and retain bounded drop/failure facts without PII leakage.

# Ordered implementation slices after lower-cluster integration

Every slice uses the accepted runtime gateway/authority/audit/capture/network/
native_security contexts and existing public core/platform/collector/CLI/Browser
packages. Root owns Cargo/SDK/graph admission; no new helper crate is proposed.

| Slice | Concrete implementation / dependencies | Required tests and completion evidence |
| --- | --- | --- |
| S0 | Root closes native/container pre-unseal provider, Audit startup/W/control contracts; CLI container role entrypoint and protected input/signal lifecycle | Real owned Docker PID1/signal/parent death/failed custody/Ready refusal, bounded drain/flush; never relabel private main |
| S1 | Gateway versioned profile parser/validator, finite origin policy resolver and installation-bound effect adapter; retain private v1 | Pure role/unknown/mixed/secret/bind/DNS/origin mismatch negatives; isolated origin allow/deny, no redirect/SSRF/network escape, no request-selected target |
| S2 | Pure endpoint/reserved-path/route-policy compiler and explicit Core local-handler vs Gate origin composition; Core owns frozen decisions | Frozen decision/route vectors, path ambiguity/admin reservation, post-audit gate loss, audit failure before connect, preserved ACK/consumption/order; no threshold edit |
| S3 | Protected certificate loader/renewal generation and actual public TLS-H1 binding; CLI/Native certificate handles | Actual container TLS chain/name/key mismatch/expired/reload/renewal failure, original capture same owner; no self-signed/verify-skip fallback |
| S4 | Network/Capture actual bounded H2 request server/body/trailer/flow-control bridge, Runtime multiplexed dispatch and optional socket PROXYv2 | Real H2 ALPN/settings/streams/interleave/CONTINUATION/HPACK/reset/body integrity/backpressure/deadline tests; required planes cannot be forwarded headers |
| S5 | Authority authenticated admin network adapter, isolated route/RBAC matrix and readiness/liveness adapter | Cross-plane denial, mTLS+role/forgery/replay, observer cannot mutate; actual readiness closes before signal drain and on W/native loss |
| S6 | Browser/Collector built asset manifest, Core pages/local report handlers, Gate loader/injection/CSP and real session collect | Real BrowserSession/Fetch plus H1/H2 where required; asset traversal/content encoding/size/cancel/CSP negatives, actual origin effects and fresh collector refusal |
| S7 | Accepted accessible Pass/stepup/recovery/verdict epoch + Gate ordered enforcement/upgrade/token/ban composition using Core owners | Single-use/expiry/replay/unknown audit/effect cancellation; no motor/speed gate; actual keyboard/AT and policy/credential negatives; no permanent grant shortcut |
| S8 | Audit/Authority/Core/Network optional configured services, full admin control/recovery, bounded report/ops/ML/fleet/projection views, CLI lab tooling | Every captured registration/flag/test reconciled; actual retained dual-control/restart/rollback/outage/ban/fleet/issuer controls; lab truth not public training authority |
| S9 | CLI role packaging and Delivery Rust-only recipes, probes/volumes/network/cert orchestration; Browser assets and exact notices | Positive required Core/Gate image jobs, expected-vs-observed inventory/all-severity CVE, source/image/platform records and Docker product role matrix before legacy deletion |

Independently implementable preparation AFTER Root freezes lower contexts: pure
closed profile schema, endpoint/role reservation table, bounded origin policy
validation, admin authorization matrix and source-selector reconciliation tests.
These can be authored without listeners/secrets/native factories, but public
contracts C1-C8 below still need acceptance first where they define new policy.
Actual wire/provider/TLS/H2/admin/Ready/image integration follows owner APIs and
separate bounded execution allocations; no test stub is a shipping substitute.

# Root canonization and owner questions

C1: accept exact public envelope version/role names, supported address/family and
finite budgets while retaining private v1; choose Core local handlers instead of
an invented origin. C2: accept canonical preset/route expansion and endpoint/
client protocol migration with Core's unchanged frozen verdict semantics.
C3: choose actual origin service discovery/address binding, auth wire/epoch,
TLS/isolated-HTTP trust premise and no-retry behavior. C4: approve certificate
provider/ACME automation and custody/renewal API without credential env fallback.
C5: define actual container admission/native/W/Ready provenance and CLI PID1/
signals, not a container-as-SCM assertion. C6: Authority admin mTLS/RBAC/SSO,
read/deny/mutation matrix and retained control producer/effect authority.
C7: Network actual TLS/H2/PROXYv2/body/flow API and Browser/Collector assets,
freshness/Pass accessibility/profile reachability. C8: inventory optional feature
owners and require positive replacement behavior/job gates; no quiet retirement.

Route questions through Root: CLI (C1/C3/C4/C5 entrypoint/handles/deadlines),
Authority/Native/Audit (C3/C5/C6 proof/custody/permits), Network (C2/C7 capture and
temporal producer), Browser/Collector/Core (C2/C7/C8 clients/assets/policy/ML),
Delivery/Lab (C8 exact image/topology/gate/retirement matrix). Runtime does not edit
their source/specs or admit graph changes by preparing this plan.

# Verification and legacy cleanup gate

Require each behavior row and every enumerated route/flag/optional selector to
name implementation, tests, actual scope and source/image proof, or an explicit
Root/user-approved replacement contract. No feature is considered retired here.
Use Rust1.99.1/edition2024 fmt/check/strict Clippy/pure source-bound gates first,
then actual owned OS/container/browser/provider pilots and Docker-only product
e2e. Record failed/ignored/filtered/unsupported separately. Full Core/Gate role
parity includes ALLOW origin effects and refusal/no-effect, real H2/browser,
authenticated admin, retained custody/W/recovery and failed shutdown facts.

Delivery replaces the two required Go image jobs with positive Rust role checks
without deleting/optionalizing them or weakening image-source refusal. Only after
accepted source/role replacement and actual build/runtime/product gates may Root
delete active Go recipes/loader/build/deployment inputs and migrate all Compose/
systemd/k8s consumers in one reviewed dependency sequence. Preserve the original
981d oracle Git-only and source-specific historical proofs. No live Go execution
bridge, renamed unrelated binary or placeholder green image is allowed.

Adversarial lenses: P0 loopback widening/private-binary alias, P0 native/Ready/W
substitution, P0 normalized proxy capture and origin-before-audit effects, P0
admin/erasure privilege mixing and Pass motor/speed rules must remain blocked.
P1 unlimited budgets/ambiguous retries, certificate fallback, feature deletion and
component-to-product over-claim require explicit tests and source-bound evidence.
Verdict NEEDS-CONTRACTS before source implementation, not permission to leave
required retained roles permanently unsupported. This design ran no compiler,
runtime process, container, network provider or release action.
