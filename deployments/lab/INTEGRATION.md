# Rust product-lab runner contract

Local engine canon is .spec/sot/rust-migration.sot.md, Local Docker engine
identity and cleanup, and .agents/rules/96-docker-engine-affinity.md. The local
default is Windows Docker Desktop: invoke
`C:/Program Files/Docker/Docker/resources/bin/docker.exe --context desktop-linux`.
Missing PATH/binary never permits automatic WSL Docker substitution. Standalone
WSL Docker29.1.3 and Windows Desktop29.9.2 were measured as different servers.
WSL-native Rust compilation remains a separate valid profile.

Before a local build, run or cleanup, preflight and retain selected context,
endpoint, actual server ID, server version and container OS type. Match the
independently selected Desktop binding and Linux OS; fail closed on missing or
mismatched identity before resource mutation. Expected Windows endpoint is
`npipe:////./pipe/dockerDesktopLinuxEngine`. IDs are per-execution observations,
not universal constants. An explicitly allocated WSL/act proxy is equivalent
only after its server ID matches Desktop; record its actual Unix client endpoint
separately from the Windows parent endpoint. act's Docker SDK must receive an
explicit Desktop DOCKER_HOST; a CLI context flag alone does not bind its daemon.
Hosted Linux uses its own runner engine and records distinct provenance.

Local affinity I/O now consumes the shared test-support docker comparison API
(1660a251/b2f04b66), not a duplicate policy. Supply
HUMANYMOUS_DOCKER_DESKTOP_BASELINE pointing to a private independently observed
JSON object with exactly schema=humanymous.lab-desktop-engine-baseline/1,
context, endpoint, server_id, server_version and os_type. Baseline IDs/versions
come from actual preflight, never this document's historical numbers. The local
Windows wrapper queries the selected context and actual daemon before resource
mutation, checks the Desktop credential-helper executable's presence and scopes
the child PATH to the installed CLI directory. It does not read credentials.
Actual compose/buildx commands must still establish plugin availability; a
successful info/presence check is not a successful pull or authenticated login.

Local Linux clients are rejected unless explicitly allocated with
HUMANYMOUS_DOCKER_DESKTOP_PROXY=unix:///... and matching DOCKER_HOST plus the
independent Desktop baseline. The actual queried CLI server must match Desktop
through verify_allocated_cli, whose proof does not claim SDK observation.
Windows parent npipe and Linux client Unix endpoint stay distinct. SDK callers
still require their own actual connection evidence; CLI equality is not that
evidence. Genuine hosted Linux keeps its assigned engine, records distinct
context/endpoint/server metadata and rejects a changed engine before cleanup.
Local build/create/run/start/down boundaries recheck affinity; project cleanup
and expired-project cleanup force a recheck. docker-engine-affinity.json,
per-case ledgers and audit-engine-affinity.private.json retain private tuples.

Shared engine-affinity pure tests5/5 and scoped Clippy passed at99e09209, with
43-file closure0 drift; no actual engine query was part of that result. The
wrapper baseline-negative test is authored; runtime preflight and latest scoped
wrapper compile remain NOT_RUN pending root allocation. New local product
profiles remain unqualified until actual engine affinity and execution are measured.
After allocation, a read-only I/O check is available independently of builds:
`cargo run --locked -p humanymous-lab-runner -- engine-preflight --evidence FRESH_PRIVATE_BUNDLE`.
It uses an independently supplied Desktop baseline, records actual CLI/context/
daemon observations and the running wrapper's executable hash privately, and
prints only status/report hash. It does not create/run/clean Docker resources
or claim SDK/remote attestation. Never reuse its bundle for a product run.
No Docker command starts during the root benchmark quiet window. Root owns the
user-authorized image cleanup; this lab does not perform prune. Existing lab
project cleanup remains separate and must recheck engine identity before down.
Never broaden image cleanup to containers, volumes, builder cache or system prune.

The sole public Go receipt is lab-report.json/public_receipts/go, also saved as
go-public-receipt.json. Its schema is humanymous.lab-go-public/1 and its field,
status and count contract is test/lab/runner/go-public-receipts.json. CI consumes
this projection rather than parsing private semantic observations. It requires
all8 static owner IDs/indices, pass/fail/not_run, evaluated/assertions-known,
bounded assertion counts or explicit null, source/manifest hashes, actual run
context and the original private Go ledger receipt SHA256. No detail, body,
configuration, actor observation, arbitrary error text or raw path is copied.
Semantic error codes come from the Go-owned16-code pilot contract; infrastructure
codes come from the lab's safe-diagnostics enum. Unknown infrastructure failure
text maps to execution_failed; unknown semantic/terminal schemas fail closed.

Public counts retain setup NOT_RUN rows and post-evaluation teardown failures.
The original evaluator stdout is saved before drain, and errors retain known
evaluation counts without upgrading their case to pass. Unknown assertion
counts remain null, never fabricated zero. A qualifying component requires8
passed/evaluated cases and the actual owner qualifier in genuine hosted context;
full_product_qualified remainsfalse. Local act refuses hosted admission, discards
emulated run IDs and cannot mint a hosted receipt. Run metadata is conventional
provenance only: the root reviewer independently verifies real GitHub run/job/
source/image receipts. The new projection pure negative test is authored/NOT_RUN
until root allocates its compile; no actual Go8 result is inferred from source.

The Go bounded HTTP profile has an actual Rust Docker caller:
`cargo run --locked -p humanymous-lab-runner -- run --evidence FRESH_PRIVATE_BUNDLE --go-product --require-full`.
Use a fresh bundle, separately from browser/collector/topology-only profiles.
The caller is authored; Windows orchestration units10/10 and scoped Clippy pass
in evidence:root/20261107090102. Its latest Unix compile and Docker execution are NOT_RUN
until the next source-bound hosted batch. A scoped pass still exits2; execution
failure exits1. Full product qualification stays false.

The eight-case inventory/routes and untouched upstream/compiler inputs come
from adapters/go/pilot-contract.json and compiler-input.json. The runner fetches
the exact upstream revision, verifies every tracked file and entrypoint digest,
copies only those bytes into a clean compiler context, checks pinned index and
amd64 child manifest plus actual Go version, and builds with network none.
It never authors Go source. The exported binary and actual container copy must
hash equally. Each case gets a fresh owned project, writer/witness keys, WAL and
external head; Gate alone joins ingress and backend internal bridges. Go binds
backend loopback18081 behind a fixed byte relay18082. Gate uses actual
go-adapter Deployment and ServiceHost with independent FileAndDirectory witness.

The attacker starts in ingress, waits for the host to inspect its running
namespace, then receives an explicit private release. Binary/HEAD probes use
the existing explicitly synthetic native session/report/RIT helper. Negative
cases preserve actual HTTP reasons, write half-close or pre-handoff cancellation
and deadline observation. Bypass probes the actual Gate bridge address at its
loopback contact-witness port and the separate backend bridge address; self
loopback refusal and timeouts cannot satisfy this proof. Origin accounting is
conservative TCP contacts, never inferred application mutations. Each real
observation goes through the Go owner's evaluator. All eight cases, errors and
cleanup failures remain in the private ledger. Missing counters remain unknown.

The scheduled eight-row NOT_RUN ledger is saved before compiler/source/image
setup; setup failure retains all rows without pretending they executed. Backend
liveness is checked after each case, and controlled stop must produce actual
actor exit plus child_reaped before Compose cleanup. An owned outside canary
has positive UDP53/TCP8080 controls; Gate/backend/attacker each run actual libc
DNS denial, direct UDP/TCP no-route and no-default-route prerequisites. Outside
DNS counters must stay unchanged. No public DNS or third-party endpoint is used.
The final image consumes product_image:service:go-gate as its explicit BuildKit
context, following https://docs.docker.com/compose/how-tos/dependent-images/ .

Parent-selected complete sanitized configurations must equal actor configurations.
The parent binds frozen source, actual images, full fixture/binary/compiler
closure, engine binary, all per-case configurations and actual OS into the
independent expected tuple before constructing the measured Receipt. Only eight
successful measured cases and cleanup can invoke the Go qualifier. This is
bounded net/http conformance, not human/browser, other Go frameworks, streaming,
204/304, post-origin rollback or graceful upstream termination qualification.

Source8132117 run42355168729 failed: Docker public receipt retained26 scheduled,
zero evaluated,19 errors and7 unsupported, with strict internally-consistent
accounting but scoped/full false. Original private case causes remain unavailable.
evidence:root/20261107083259 preserves the decoded log and derived public receipt.
Later fixes and Go source do not overwrite or qualify that historical run.

Its separate browser-product job failed before the first writer identity:
7 assertions passed/1 failed, execution/browser_gate_start. The particular
hardening/installation/mounted-key failure is not recovered. New startup source
emits fixed stage markers, checks actual writer mount UID/mode/type, and saves
private Docker logs/inspect before cleanup. Only complete allowlisted stage
codes can enter public safe_diagnostics; raw exceptions and paths stay private.
This narrows a subsequent failure, not the historical cause. Root's later
user-authorized local/hosted timing comparison controls execution admission;
no lab command starts during its benchmark quiet window. The current Go
qualification contract still requires its actual standard free hosted runner.

The strict Go profile requires GITHUB_ACTIONS=true, RUNNER_OS=Linux,
RUNNER_ENVIRONMENT=github-hosted and ACT absent. Missing/self-hosted/act context
is rejected before image builds and retains eight NOT_RUN rows. Environment
markers are an admission check, not cryptographic attestation: the owning
reviewer must independently match real GitHub run/job/source/image receipts.
A local act comparison cannot mint hosted=true qualification; it needs a
distinct nonqualifying context/profile if Go execution is requested there.
References: https://docs.github.com/en/actions/reference/workflows-and-actions/variables
and https://nektosact.com/usage/index.html#skipping-steps . The new pure context
test is authored but NOT_RUN during root's benchmark quiet window.

The accepted browser product profile is separate from collector component tests:
`cargo run --locked -p humanymous-lab-runner -- run --evidence FRESH_PRIVATE_BUNDLE --browser-product --require-full`.
Do not combine it with --collector or --topology-only. Its source is authored,
Linux compile and actual execution are not_run pending the next hosted batch.
The canonical fixture is crates/collector/pilot/fixture.json; lab actors include
that exact JSON, not a copied scope/route definition. Root owns its runtime-pilot
WASM feature/test and typed WebDriver JSON; delivery owns the separate pinned
build/rust/browser-product.Dockerfile. Component network_mode:none is unchanged.
Browser WBG31111 and proxy31112 bind only browser loopback, Gate frontend8080
is on a dedicated internal bridge, actual ServiceHost/backend bind only Gate
loopback. Separate-UID witness keys/head persist on disjoint named volumes;
actual ServiceHost::bind_with_witness uses FileAndDirectory and private framed
audit-owned client traffic. No public shutdown API: trusted Docker exec invokes
the Rust --browser-stop control and waits for runtime flush/drain evidence.
Host checks namespace/EROFS/UID/caps/no-new-privileges/no-origin-route, actual
proxy hits and response403+ACK/200+ACK/422-without-ACK, closed-epoch no-retry,
independent zero origin effects, authenticated WAL and external signed head.
Missing receipts fail; no full-product qualification is implied. Browser-profile
witness outage/recovery, physical failure, full sandbox, human/AT and qualified
collector provenance remain unavailable until their own actual evidence lands.
Free hosted policy comes from program-roadmap and delivery SoTs: no private raw
artifact upload, no duplicate dispatch, sanitized summaries/logs/hashes only.
Reports now expose safe_diagnostics under product-lab/2. Its sole enum/limits
contract is test/lab/runner/safe-diagnostics.json. Public CI may copy only that
validated schema's fixed failed-case ID/status/error-code rows and counts;
raw case.detail, exception text, credentials, request/identity data and private
evidence remain excluded. Truncation is explicit partial=true and cannot close
readiness. Source63f69ae browser failure remains cause-unknown because those
safe diagnostics were absent; a future diagnostic improvement does not supply
missing historical evidence.

Current implementation is authored Rust 1.99.1/edition 2024. Root owns workspace
registration: member `test/lab/runner`, package/binary `humanymous-lab-runner`,
workspace serde_json/sha2/getrandom only; no new production dependency version.
No root Cargo/toolchain changes belong to this lane.

```
cargo run --locked -p humanymous-lab-runner -- run --evidence EXISTING_LOCAL_SPECFOLD_BUNDLE --require-full
cargo run --locked -p humanymous-lab-runner -- run --evidence EXISTING_LOCAL_SPECFOLD_BUNDLE --topology-only --require-full
cargo run --locked -p humanymous-lab-runner -- run --evidence EXISTING_LOCAL_SPECFOLD_BUNDLE --collector --require-full
cargo run --locked -p humanymous-lab-runner -- cleanup
cargo run --locked -p humanymous-lab-runner -- audit --evidence EXISTING_LOCAL_SPECFOLD_BUNDLE
cargo run --locked -p humanymous-lab-runner -- provider-preflight LOCAL_MANIFEST
```

Each run requires a fresh bundle. Existing source-manifest/lab-report is refused;
an existing audit-report is also refused. No retry may replace a failed result.
`run` builds from an immutable allowlisted source copy and uses unique owned
Docker project/image tags. Host code coordinates Docker only. All HTTP/negative
clients execute in hardened containers, without shell/script execution. Exact
files and hashes are source-manifest.json, resolved-compose.json,
runtime-image.json and per-service/network inspect JSON. Docker CLI output is
registered as private docker-transcript.json. New Dockerfiles live in
deployments/lab/images; build/rust remains another owner's read-only surface.

Output lab-report.json schema `humanymous.product-lab/2` has evidence_types,
cases[{id,status,detail}], components.runtime (actual binary pilot report),
components.wargame, components.target_attestation, components.pass_recovery,
components.origin_audit, components.attack_catalog,
components.independent_witness,
optional components.collector, required_unavailable, required_cases,
full_product_qualified and release_ready, plus the pilot-read top-level
status/qualification/errors/skips/source_sha/documentation_review_sha256.
required_unavailable is derived: a capability is listed unless every one of its
required case IDs (orchestration/qualification.rs, a superset of the delivery
pilot IDs) was evaluated in this run and passed; skips counts required cases not
evaluated. full_product_qualified is true only with all required cases passed,
no failed case and owned cleanup. Exit 0 means scoped cases passed; --require-full
returns 2 while full product gates are unavailable; failures return 1.
The default profile's attestation-pilot (network none, same hardening) has the
shipped runtime binary generate its installation key at lab start, pins the
enrollment public key and verifies fresh-nonce answers; installation-target-binding
and unknown-target-refused are recorded from that exact eight-row ledger.
The default profile's pass-pilot (network none, same hardening) runs the shipped
runtime binary with one `attest_floor` route and the legacy Pass surface
(read-only web/pass.html in the runtime image). An in-container Rust HTTP/1.1
client is challenged, issues a Pass instance, pays the proof of work, solves from
the public scene with no motor trace, redeems the receipt at /__hmn/stepup and is
admitted with exactly one origin effect. `pass` and `pass-forgery-refused` are
recorded from that exact sixteen-row ledger (missing proof of work, missing,
forged, wrong-session and expired receipt, lifted and wrong-session proof,
replayed solve, a proof never overriding a scoring CHALLENGE, positive control
after negatives). The expired-receipt row waits for receipt expiry plus skew,
about four minutes. Keyboard/AT human Pass runs remain human-attested and are
not part of this case.
The default profile's origin/audit stage (roadmap 2c, components.origin_audit,
origin-audit-ledger.json) reuses the internal `ingress` and `backend` bridges:
oa-origin (backend only) logs every TCP contact with its peer; oa-gate (both
bridges, custody and WAL on the oa-keys/oa-wal volumes) runs the shipped runtime
binary behind two plain TCP relays, because the gateway accepts loopback
addresses and peers only; oa-client (ingress only) and a network-none oa-tamper
that mounts only the WAL volume. The runner supplies the passphrase privately and
retains the signed head between sessions as the trusted anchor. Sessions:
provision, restart, `docker kill` crash, five tampered copies (copy and torn tail
must start; forged, truncated and rolled-back copies must be refused by runtime
startup authentication) and a final positive control. `origin-isolation` needs
the exact topology, every client row (origin and Gate loopback ports
unreachable, unsigned and absolute-form refused, signed effect forwarded once,
replay refused) and exactly one origin contact per phase, all from the Gate's
backend address. `audit-restart` needs one contiguous verified chain across all
live sessions, ALLOW decisions equal to origin contacts and the exact tamper
outcomes. Ingress is plain HTTP/1.1, not TLS/H2, and this Gate has no
independent witness.
The default profile's attack-catalog stage (roadmap 2, components.attack_catalog,
attack-catalog-ledger.json) runs the `humanymous-attacker` network catalog
(seed 42) from `ac-attacker` (ingress only) through `ac-gate` (both bridges,
custody and WAL on its private tmpfs, same relays as oa-gate) to `ac-origin`
(backend only). The runner starts, restarts and stops one shipped-runtime
session per case on the attacker's request and returns that case's effects:
the audit suffix of the Gate's offline-authenticated history (a verified
extension of the previous one) and the origin contact delta from the origin's
own log. `attack-catalog` needs the exact topology, all 19 supported cases
evaluated with the catalog's exact expectations against those effects, the
Go rules of `scripts/assert-attack.mjs` at 981d25c (no attack step forwarded or
ALLOWed, the controls never DENY), exactly the seven unsupported rows (human,
AT, accessible Pass, signed installation, coherent T4 and TLS/H2 are owned by
other required cases; audit-outage injection has no producer), one runtime
digest for every session and every origin contact from the Gate's backend
address. Go's 72 browser/uTLS/HTTP2/proxy profiles have no Rust client in this
HTTP/1.1 topology. `swarm` is not evaluated: Go's swarm asserted
`l5.correlation.proxy_rotation`/HR-19 from three subnets, but the shipped
runtime accepts loopback peers only (the relay hides the client subnet) and
serves no cross-session correlation (`application::temporal` is test-only), so
it needs an owner decision on a peer-identity protocol and serving correlation.
The gate_tls stage (roadmap 4 item 2a, components.gate_tls, gate-tls-ledger.json)
runs the attack-catalog supervisor in `tg-gate` with TLS terminated by the shipped
runtime itself: the Gate generates an ephemeral lab CA and server pair on its
private tmpfs (no committed key, no self-signed fallback), each session starts
from the closed `humanymous.runtime-loopback-tls-ingress/1` launch envelope, and
the ingress relay is L4 passthrough, so the runtime captures the original
`tg-client` ClientHello, H1 order and H2 frames. `gate` needs the exact
topology, 11 catalog rows with unchanged catalog expectations plus three TLS
edge rows (truthful synthetic rustls client CHALLENGEd, replay refused,
duplicate Content-Length closed without a response), every row's effects bound
to the runner's measurement, no origin effect, the origin unreachable from
ingress and only the Gate's certificate presented. `raw-tls-h2` needs raw-TLS
and original-H2 routes decided (not 422) over TLS H1 and hand-framed H2,
original-H2 refused over TLS H1, a Chrome claim over a rustls hello never
forwarded, no plaintext fallback and every session ready in TLS mode. Under the
frozen scoring no forwarded origin effect is reachable over TLS for a synthetic
client, Go's gate admin/ban/RBAC e2e checks are not ported and browser
ClientHellos have no Rust client.
The deployment stage (roadmap 4 items 2h/2i, components.deployment,
deployment-ledger.json) executes the shipped adopter manifest
`deployments/compose.release.yaml` in the same owned project with only
`deployments/lab/compose.deployment.yaml` on top, through the manifest's own
commands: `docker network create` for its external origin network (internal,
owner-labelled), `up -d edge` and `run --rm -T runtime < runtime.json`. The
layer may change only the runtime's image (product.Dockerfile
`deployment-runtime`: scratch, the runtime-stage binary as entrypoint, UID
65532, an empty 0700 /var/lib/humanymous), build, pull_policy and labels;
`compose config` with and without the layer must differ in exactly those keys.
The edge is the manifest's adopter placeholder run as a plain L4 relay (no TLS,
ports replaced by the internal `dep-ingress`); dep-origin answers on the
external origin network; dep-custody (network none, the manifest volume)
performs the documented custody step that has no shipped command; dep-client
is ingress only; dep-challenger is network none. `documentation-examples`
needs the reviewed docs/reviews.json digest, the Kubernetes Secret stub's
embedded document equal to `deployments/humanymous-runtime.example.json`, that
example as shipped refused (placeholder passphrase, no readiness), and with
only the documented passphrase edit readiness on 127.0.0.1:8444 and every
documented route refused unsigned (401) and forwarded signed in a fresh session
(200, origin body) with replay refused (403), in a first and a restarted
`run --rm`. `deployment` additionally needs the runtime container as Docker ran
it (manifest user, read-only root, cap drop ALL, no-new-privileges, pids 512,
/tmp tmpfs, the edge's network namespace, only hmn-data mounted, no ports, no
argv, the built image), the volume layout copied from that image, origin
contacts equal to forwarded routes and all from the edge, a verified contiguous
audit history on the volume whose ALLOW decisions equal the forwarded routes
and which the restart extends, and lab-scoped signed target attestation: the
deployed image generates the installation key inside the volume (and a second,
foreign installation), a re-enrollment is refused (`already_provisioned`) and
the challenger pins the enrollment, issues every nonce and verifies against its
own copy of the runtime binary digest and the edited example's digest (fresh
answers accepted; stale, tampered, foreign-key and changed-configuration
answers refused). Not executed: a live cloud provider, the Kubernetes and
systemd manifests (no cluster or systemd PID 1; the runtime's gateway test
parses them), the published ghcr image (the lab builds the same shape from
source), an adopter TLS edge, hardware/loader/listener-bound attestation.
Never relabel exit 2 as
product success. Collector is optional Firefox component evidence, never a real
human/AT or Meari+CEF product browser. No sandbox-disabling option is supplied.

Topology controls use three distinct internal IPv4 bridges, no public ports,
root filesystem read-only, nonroot 65532, cap-drop ALL, no-new-privileges, no host
or Docker API mounts, bounded resources. A live origin positive control is
paired with direct origin-IP and cross-bridge canary negatives. Metadata and
gateway API negatives cover exact addresses/ports, not all host/IPv6 paths.
Inside-client absence of a default IPv4 route complements finite probes without
third-party traffic. Cleanup targets only the generated project's resources.
An external runner may call cleanup for owner+grammar+expiry-matched projects;
no scheduler is installed by this tool.

Actual runtime remains loopback-only. Its child, Rust client and counted owned
origin share a network-none container and sealed audit state. This proves the
bounded session/RIT/audit-before-origin/restart path, not cross-namespace L5
product topology, raw TLS/H2, browser collection or independent witness state.
No external signed target attestation endpoint exists. Wargame will use a
supervisor-owned child handle/private readiness mode only; remote/full-Gate
binding remains unsupported. `test/lab/runtime` consumes shared
session::canonical, not a copied signer. Attacker/defender API integration is a
separate sequential red-catalog lane stage.

Existing run.py/cleanup.py/audit.py, clients/*.py and authority/pilot.mjs are
historical foreign-language prototype debt. Canonical compose.rust.yaml and the
new runner never host-execute them (no spawn/shell of those files from the
lab-runner process). Cap CLI `--authority-node-ipc` may Docker-coordinate
`deployments/lab/compose.authority.yaml` built from freeze-allowlisted inputs
(`build/rust/authority-lab.Dockerfile` (+ dockerignore),
`deployments/lab/authority/**` Cap inputs, `adapters/node/src/**` as needed).
Container ENTRYPOINT execution inside that image is not runner host execution.
Prefer a Root-allocated Rust Cap pilot that retires `pilot.mjs` from the Cap
path; until then Cap may measure the existing authority-lab composition only as
component evidence. Authority Cap green alone is never `pass_product_clean_room`
and cannot invent G1/G5/G6 product cells. Retire foreign prototypes after old
evidence consumers move; their older successes cannot qualify new source. Node
prototype is not the current Rust adapter implementation.

First v2 checkpoint4f89ec3: Rust runner unit4 and Clippy passed; actual Docker
topology-only25 assertions passed, including live cross-namespace direct-IP
negatives and absence of a default IPv4 route. `--require-full` exits2. This is
not a runtime pass: components.runtime is explicitly not_run. The first default
runtime snapshot failed because a concurrently authored audit service module
was declared before its source file existed; that failure is retained. Fresh
runtime source, collector and independent witness must be executed separately.
The topology controls image and runtime process image are separate targets/tags;
the controls image contains only the Rust runner. Do not reuse old CVE results
for these new runtime images; supply a fresh source/image/database-bound scan.

V3 integration consumes test-support26d9925/e30e0e9 bounded capture/duplex and
auditd001ee6 shared client/log/codec. Local process-lifetime duplication is gone.
Root27-member registration includes test/lab/runtime; its standalone lock is
retired. One Installation factory shares fresh synthetic sealed custody/config
and independently counted origin with the original process and wargame profiles.
The new wargame path delegates entirely to attacker1ee2c92 OwnedLab/defender APIs.
The origin counts contact before validation. Actual setup errors preserve all
26 cases as19errors/7unsupported, never a mock success or missing report.

Private source54017 remains unchanged:33orchestration assertions, actual runtime
origin2/audit4 and26scheduled=19evaluated/7unsupported,0errors/0mismatches.
Latest fresh source60627 passed71orchestration assertions plus that same complete
wargame matrix. It additionally executed separate UID65533 witness and UID65532
writer, four disjoint named key/head/WAL volumes, actual shared external Log
co-signing before two counted origin effects, same-identity/head container
recreation, authentic older-head refusal and populated key-volume EACCES from a
different UID. Witness control-plane unavailability is intentionally injected
after actual process shutdown; a typed broker Unavailable closes the shared Log
with origin0 and an ambiguous durable record. No signature is fabricated.

These are component results: no full Gate capture/browser/human/AT, no external
physical-host custody independence, no persistent cross-supervisor anchor and
no runtime-binary sidecar transport. New builder emits only static musl Rust
binaries in scratch images; collector remains its own Firefox/toolchain image.
Rust unit6/Clippy currently pass; the builder also runs Linux runtime-pilot unit
and Clippy. Fresh image/Cargo/secret scans after the final wargame corrections
remain required. Source60627 contains the code at its own immutable freeze,
not later report-only/case parsing changes.

Failure lineage:50621 concurrent audit module source gap;55200 first writer setup
failure had a retry-path evidence-reuse error. Its original dirty source graph
is lost; failure report/transcript/image are55537 failure-* only.55200 now binds
the separate retry source and its empty-attach failure.60036 retains the later
injected-EOF timeout failure.60627 is a new acknowledged-pipe/broker-unavailable
attempt, never retroactive success for any failed attempt. All raw/private files
must be Specfold-attached; public summaries omit startup secrets and raw bodies.

Provider preflight is local, non-mutating and always exits 2: explicit opt-in,
owned exact hostname, bounded budget/expiry, cancellation janitor, qualified Gate
source, credentials presence, and Cloudflare Access/domain constraints are
necessary, not sufficient. No cloud API, paid resource or public Tunnel is
created. Cloudflare uses cf CLI when no Wrangler configuration exists.
