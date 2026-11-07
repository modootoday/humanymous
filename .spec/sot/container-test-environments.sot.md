---
id: sot:root/container-test-environments
title: Canonical Compose lab and Cloudflare test environment profiles
status: active
created: 2026-11-06T22:41:55Z
updated: 2026-11-06T23:10:00Z
verify: manual
rel:
  governs: [deployments/lab/**, build/rust/**, test/lab/**]
  depends_on: [sot:root/cross-os-runners, sot:root/adapter-ecosystem, sot:root/automation-evidence]
---

## Rust collector component lab delta c3389b6

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

Version 0.2.0. Research and scoped Docker implementation checked 2026-11-07 KST.
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
