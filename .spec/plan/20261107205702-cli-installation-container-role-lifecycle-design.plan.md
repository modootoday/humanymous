---
id: plan:root/20261107205702
title: CLI installation container role lifecycle design
status: draft
created: 2026-11-07T20:57:02Z
updated: 2026-11-07T20:57:02Z
rel:
  depends_on: [sot:root/cli-installation, plan:root/20261107205157, sot:root/structural-development]
---

# Brief

Draft v0 for Root canonization, not an accepted replacement for native admission.
Replace the existing Go Core/Gate shipping roles with actual Rust deployment
compositions. CLI owns private launch input, lifecycle supervision and truthful
exit/readiness publication; Runtime owns ingress, origin effects, policy and audit;
Platform owns actual process/transport primitives. Preserve the accepted eight
public packages and container/deployment context. No new helper crate, path move,
Cargo edit, executable/source change, runtime or release action is authorized by
this design. Scoring, a11y, custody, origin and native-proof gates remain intact.

# Behavior inventory and concrete gaps

Read with [[plan:root/20261107205157]] and its shipping-role-source-map.json under
[[evidence:root/20261107052001]]. The following inventory binds inspected source,
not measured behavior or complete transitive legacy equivalence.

| ID | Legacy deployment behavior and source | Present Rust gap | Owner / required closure |
| --- | --- | --- | --- |
| C01 | Core Dockerfile serves web/WASM assets and public TLS443; lab entry8443 | Runtime is loopback/private, not demo/static asset host | Runtime+Browser composition, CLI role admission, Delivery actual image |
| C02 | Gate edge8444/admin8445; release maps443 and host-loopback admin | Configuration rejects nonloopback listen/origin; external origin container is required | Runtime typed public-ingress/origin/admin contract; no widening existing validator |
| C03 | BYO certificate pair, ACME cache/account, lab self-signed | CLI NativeSecrets is service-scoped; certificate provider is not a generic unseal key | Runtime TLS/capture contract; custody-approved material provider; CLI selector-only wiring |
| C04 | routes/max-body/HSTS/trusted-proxies and upstream cloaking | Forwarded headers cannot establish original TLS/H2; route admission must remain fail-closed | Runtime/capture policy and actual Docker origin effects |
| C05 | keystore/audit-WAL persist across restart; HMN_UNSEAL env in legacy | Rust must not inherit env/argv/plaintext unseal fallback or ephemeral production identity | Audit/native owner protected broker, recovery anchor; CLI refusal before unseal |
| C06 | Core /healthz; Gate health/ready routes and ready503 before drain | Runtime manifest is experimental-loopback; CLI diagnostic is not role-wide readiness | Runtime full role readiness/revocation API and CLI authenticated publication |
| C07 | SIGINT/TERM drains public+admin then seals keystore | Legacy ignores Shutdown errors; private Rust main retains unused sender | CLI independent watchdog/signal owner plus actual Runtime drain/flush outcome, not copied legacy success |
| C08 | release nonroot65532, read_only, cap_drop ALL, no-new-privileges, volumes/tmpfs | Default read permissions/mount presence do not prove private endpoint/custody owner | Platform actual namespace/process/mount evidence and bounded cleanup |
| C09 | stop_grace_period30s, Gate shutdown-grace25s | No container stop budget or independent parent proof accepted | Root deployment budget, CLI monotonic deadline, Docker actual TERM/KILL outcomes |
| C10 | separate bearer/mTLS admin, settings persistence and optional trust inputs | DATA-only signature/scope check grants no management/kill/erasure authority | Runtime authN/authZ/dual-control and separate status vs stop capabilities |
| C11 | model/signature/shadow/canary, Redis fleet/projections, agent/PAT/WebAuthn, USB and lab receipts | CLI cannot silently omit advertised features or reinterpret frozen score policy | Owning behavior inventories classify each as measured/explicit refusal/accepted retirement |
| C12 | structured logs/version/receipts and separate shipping image/license/CVE gates | stdout used as readiness pipe can block or mix with public logs | CLI dedicated bounded lifecycle channel, Runtime redacted telemetry, Delivery separate required gates |

The top-level source inventory has27Core flags and35Gate flags. This records every
declared flag name found in their current main.go files; env-only variables and
registered handlers still require Runtime's transitive role inventory.

Core: addr, max-conns, web, rit, acme-domain, acme-cache, acme-email, tls-cert,
tls-key, log-level, log-console-format, log-console-stream, log-plain-file,
log-jsonl-file, external-input-receipt-dir, ops-token, ml-bundle, ml-fp-budget,
ml-trace-dir, ml-pubkey, ml-bundle-sig, ml-bundle-digest, ml-shadow-bundle,
ml-canary, ml-canary-max-fp, ml-canary-probation, redis.

Gate: addr, admin-addr, upstream, node, monitor, origin-key, keystore, tls-cert,
tls-key, acme-domain, acme-cache, acme-email, acme-directory, routes, audit-wal,
audit-verify, audit-redis, audit-clickhouse, redis, trusted-proxies, agent-keys,
anomaly-shadow, pat-issuers, webauthn-creds, shutdown-grace, max-body, hsts,
admin-mtls-ca, settings-dir, virtual-usb-results-dir, log-level,
log-console-format, log-console-stream, log-plain-file, log-jsonl-file.

# Proposed launch and ownership contract

## Role and supervisor composition

Keep image.core.demo and image.gate.edge as distinct required roles. A proposed
CLI container command selects an explicit typed role; its final command spelling
is Root-owned and is not an alias for the private Runtime binary. Existing native
service HostPlan remains v1; do not smuggle a container flag into its service/SID
fields. Introduce a distinct, strict, versioned container launch schema only after
Root accepts the contract. Reuse bounded parsing primitives, not duplicated crypto.

Preferred supervision candidate: O is PID1/reaper and owns S plus subordinate W;
finite P is separately registered by the accepted container provider and original
I delegates to it. P is not a permanent third main service, and readiness does not
depend on P exiting. This maps the established finite-P/two-persistent-owner
semantics into a container only if Authority/Platform accept and measure it.
Alternatives such as an external authenticated supervisor must preserve the same
retained ownership and private channels. Docker labels, UID65532, PID1, a mounted
socket or a passed boolean are never sufficient native authorization.

Do not invent Windows service SIDs for Linux namespace principals or reinterpret
the Windows-targeted v2 wire. Native owner must accept a versioned provider and
issuer/process-start/namespace/claim bindings first. Unsupported remains refusal
before unseal, origin admission or application Ready. The CLI cannot mint proof.

## Private configuration and sealed custody

Proposed nonsecret launch DATA binds role, installation/generation, exact artifact,
BASE/configuration digests, typed endpoint selectors, origin/route/certificate
policy commitments and explicit startup/stop budgets. Reject unknown fields,
unsupported versions, replay, changed raw bytes and excessive input before secret
access. Initial limits may reuse32KiB launch/64KiB configuration ceilings; Root
must approve the schema and exact bounded endpoint representation.

Read bounded nonsecret configuration through an already-open handle from the
accepted provider, verify descriptor type/peer/locality and exact digest, then
retain that same object through native preparation. A pathname or stdin merely
being nonterminal is not protected-input proof. No shell entrypoint, sourced env,
configuration echo, exec string, fd-number credential or secret argv/env fallback.
Descriptive handle selectors may be public; kernel ownership must be verified.

Configuration carries empty unseal material. Native protected broker fills only
the existing zeroizing owned secret after native admission, exact scope and W
requirements; missing/locked/unqualified provider fails closed. Docker secret
files are not automatically an authorized replacement for this broker. TLS key,
admin/stop credentials, origin-auth material and audit-unseal keys have separate
purposes and permissions; CLI does not consolidate or log them.

Custody/audit reside in a verified nonroot-owned persistent child, with readonly
image and distinct certificate cache. Verify opened handles against mount/owner/
link policy without chown/ACL repair or host account changes. Normal restart must
recover the same identity, sequence floor and trusted head; lost/wrong/readonly
volume fails before Ready. First identity creation is a separate explicit I action,
not an allow_new default. Namespace/root adversary and orchestrator compromise
remain threat-boundary limits, never independent-W qualification by assertion.

## Readiness and lifecycle publication

Required phases: Received -> InputBound -> TransportPrepared -> NativePrepared ->
CustodyRecovered -> RuntimePrepared -> AppReady -> Draining -> Stopped/Failed.
This is a proposed adapter model, not a public native capability constructor.
TransportPrepared/discovery cannot enable application ingress, unseal or origin
effects. AppReady needs Runtime's complete role manifest, native receipt/ACK,
required capture/browser/TLS-H2 planes and recovery/W state. No subset comparison,
cached JSON, socket bind or adapter status credential can substitute.

Retain public health/readiness compatibility routes through Runtime's reserved
control plane, not a new CLI public listener. Healthy means only process/control
liveness; readiness is503through startup/drain/failure. Publish nonsecret identity,
role/source/config digests, actual capabilities and degraded/unsupported facts.
No raw tokens, private paths, principals or audit payloads in probe responses.
Product qualification stays false until the role's actual acceptance gates pass.

Use a dedicated bounded private supervisor lifecycle channel, separate from public
stdout/stderr logs. Delivery flush is under the existing independent watchdog;
successful publication/ACK never renews its original deadline. A blocked pipe,
lost O/W or stale receipt irreversibly revokes Ready/admission. Do not recover
in-place from first loss using a fresh token or heartbeat.

## Signal, drain, flush and exit contract

Arm signal handling and independent startup watchdog before potentially blocking
input/custody operations. First SIGTERM/SIGINT or accepted native stop request
atomically revokes readiness and new admission, then uses the same owned Runtime
shutdown channel. Already-linearized effects may be partial/Unknown; no new origin
effect after revocation linearization. Runtime owns cancellation, request tracking,
admin stop, actual drain and audit flush; CLI consumes the actual typed outcome.

Validate drain + supervisor reserve < actual container-manager stop ceiling.
Legacy25s/30s is inventory, not an automatically approved Rust budget. Example
20s drain +5s reserve <30s is only a candidate requiring actual Docker evidence.
Use original monotonic minimum deadline; repeated signals never extend it or
turn emergency termination into success. A second signal may request earlier
owned termination but reports failure/Unknown, never drained/flushed by inference.

Keep O/W and lifecycle channel alive through actual final flush/report delivery.
Only actual stopped AND drained AND audit_flushed, accepted final delivery and
retained owner cleanup can commit Stopped/exit0. Failures preserve independent
drain/flush booleans and partial/Unknown facts. Watchdog expiry/noninterruptible
output uses the existing fail exit74 contract without blocking diagnostic output.
Reaping/owned process-group or cgroup cleanup needs Platform proof; same-process
watchdog cannot promise preemption of every blocked kernel syscall. Host-side
kill ceiling remains separate. Docker SIGKILL/exit137 is not a successful stop.

# Root decisions and implementation slices

| Slice / decision | Concrete acceptance needed | CLI work after lower cluster acceptance | Verification |
| --- | --- | --- | --- |
| D1 | Accept O-as-PID1/S/W and finite-P provider or equivalent retained external supervisor; target Linux identities and wire | CLC1 typed lifecycle/config budget model, no native proof constructor | Pure phase/event/deadline tables, wrong-role/version/overflow/replay and no-secret-call negatives |
| D2 | Runtime public role schemas, original TLS/H2, origin/auth/admin/browser contract and explicit unavailable-feature disposition | CLC2 exact shared role/config/manifest binding; no cloned parser or scoring | Shared valid fixture + each digest/scope/cap/role/capability mismatch; origin remains untouched on refusal |
| D3 | Protected Linux container input/secret/custody/W provider, first-creation and restart recovery ownership | CLC3 consume opaque opened-input/native preparation API, zeroizing secret and same resource owner | Native provider isolated Docker positive/denials, tamper/symlink/mount/UID/namespace/PID reuse/replay/key-loss cases |
| D4 | Runtime shutdown/revocation/actual report API, status vs stop auth, real manager budgets | CLC4 signal/PID1/reaper/bounded private publication adapter using shared Platform primitives | Pure stop races plus actual blocked config/output/flush, owner loss, orphan/reap and TERM/deadline tests |
| D5 | Retained Core/Gate external behavior matrix, readiness endpoints, deployment command/secret provisioning and certificate cache contract | CLC5 release role argument validation and readiness/exit serialization | Actual Docker Core/Gate TLS/H2/browser/origin/admin/custody/restart/drain; required jobs remain required |

CLC1's pure model and negative boundary tests are independently implementable
after Root canonizes D1/schema/budgets and lower module handoff; no native provider
is needed to test refusal. CLC2 can follow Runtime's accepted public role DATA API.
CLC3/4 require actual native and shutdown producers, not fabricated placeholders.
CLC5/Delivery recipe replacement requires accepted behavior inventory plus CLC2-4;
image build/scan alone cannot close the role. Root sequences modules in existing
humanymous-cli, humanymous-runtime and humanymous-platform packages; source paths
are assigned at each handoff, never independent new crate creation.

Cross-owner decisions go through Root. No feature becomes optional, role removed,
score threshold changed or test deleted by CLI initiative. Unimplemented advertised
features block role equivalence unless Root explicitly accepts their disposition.

# Verification and retirement gates

Source-only plan has no compile/test pass. Future authored implementation is Rust
1.99.1/edition2024. Require fmt/check/strict Clippy and scoped pure tests under
Root allocation, exact selected local+registry/tool/config/env before/after,
canonical source joins and actual inventory denominators. Historical f20/cf64468c
or old watchdog proof does not qualify a new graph or container behavior.

Actual Linux container tests are Docker-only on the intended Desktop engine,
separate disposable namespaces/volumes, no local SCM/accounts or host privilege
changes. Exercise PID1 signal/reap, missing/malformed/oversized input, wrong
native identity, absent W, partial startup, broken lifecycle reader, persistent
recovery/replay, disk/full/readonly/corrupt volume, final-output/flush deadlines,
SIGKILL and original manager ceiling. Keep each positive/negative/Unknown result
separate and verify no unexpected origin effect on refusal.

Full-role acceptance additionally needs public TLS and original H2 observations,
browser asset/collector/Pass session flow, authenticated admin allowed/denied and
dual-control mutation, shared frozen scoring outputs, durable audit-before-effect,
restart identity and actual drain/flush. Required capability absence fails closed;
motor richness/speed never gates Pass/CHALLENGE. Include complete binary/source/
license inventory and actual image/CVE reports per Delivery plan. Only then may
Root replace required image jobs/Compose consumers and remove active Go build/
loader/entrypoints together. Preserve Git-only981d historical oracle and source-
bound old evidence. No publish/tag/main merge or deployment is authorized here.

# Critique reflected

Single-author blue/red/SRE/DPO/evaluator lens review, not an independent panel.
Adversarial verdict: needs-work until D1-D5 owning contracts are canonized.

- P0: UID/PID1/container labels could forge native authority. Require opaque real
  provider and versioned principal/claim binding; no Windows-SID fiction or bool.
- P0: public topology could bypass loopback/original capture gates. Leave existing
  Runtime validator unchanged; require separate owning public-role composition.
- P0: readiness may race signal/owner loss and allow a post-revocation origin
  effect. Require one Runtime linearization and monotonic irreversible loss.
- P1: normal stdout or final flush can block PID1 until SIGKILL. Separate bounded
  control channel, independent original deadline and exact failure/Unknown facts.
- P1: bind-mounted secrets or empty volume may regenerate identity silently.
  Require protected producer, explicit first creation and anchored restart refusal.
- P1: legacy Shutdown ignored errors and logs completion anyway. Consume actual
  Rust drain/flush/cleanup reports; do not migrate that false-success behavior.
- P1: removing Go jobs before optional behavior disposition erases required scope.
  Require complete owned inventory and positive replacement gates before cleanup.
- P2: container co-location cannot establish W independence or defeat host root;
  keep noncoverage visible and qualify independence separately with evidence.

# Source binding

Inspected path SHA256 values below are observations, not a complete build closure.
Legacy code is a read-only behavior oracle; no new Go implementation is proposed.

| Path | SHA256 |
| --- | --- |
| cmd/gate/main.go | 29464bb31c8df7ccef2f11769525345def1503ac931072cbfe9093bf0816c6e3 |
| cmd/server/main.go | cdd8010a55b92343e573ee467cbba90400bb69d3531e170f427c916245e9ab56 |
| cmd/server/server.go | e89b067fab0b83aa483fe9e82e1a1feaea68eaf1a433e7bb39f5ebcb7f576034 |
| cmd/server/ops.go | 60d30e7def70279bc6aa037d0b0258cc499c1e78f0ec839c0b367cba98974b63 |
| internal/gate/control.go | fb82ba05def759b11ef9d8d12d440b1070ce7da69f8367eb3ab943ef281cd71c |
| build/core.Dockerfile | 21c178708a3757df14569078898199be64d907499a8a1b388f249cb571d4fe8f |
| build/gate.Dockerfile | 5e66d0b83be2d4b186ab919e3f181f16704d8dbf25615fbbf950d2f8a2b3511c |
| deployments/compose.release.yaml | 4fec68fdafbdca09557caeeff3b06253916a718667a3aa29ffd644adf605258b |
| deployments/compose/defenders.yaml | e926e16061f2d6e93b33d47bdfdc78b1bf828dbf8fa580db37a15c9d669ba65c |
| crates/runtime/src/main.rs | ce657a2bd2758ffb3a712626359c43e2bdefe19817accad3173ce209fc746797 |
| crates/runtime/src/configuration.rs | 8c5c3f1b8502a74e759d5da0e2e4a6bf570faaa39957a2e258f4285f4e09178e |
| crates/runtime/src/service/readiness.rs | 588e2973865ad469d46a9bb9faeab13c5f95b2d9cb989e1022977d27ccb40fc5 |
| crates/cli/src/runtime_host/input.rs | 241b01120bffc81c9418a3e02d275d929155d611ffca801352421b1d83e44df7 |
| crates/cli/src/runtime_host/platform.rs | fb611870c133341cd0beaffe8101ae9b3af9f97ecefce88027e271b6f577d25c |
| crates/cli/src/runtime_host/control.rs | c9cf54c2ad318eb1f5852bbad6640146fbe73368011ed284e27afae6c68ed160 |
