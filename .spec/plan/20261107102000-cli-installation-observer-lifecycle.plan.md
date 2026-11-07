---
id: plan:root/20261107102000
title: Draft observer-host lifecycle and qualification matrix
status: draft
created: 2026-11-07T10:20:00Z
updated: 2026-11-07T10:20:00Z
rel:
  implements: [sot:root/cli-installation]
---

# Scope and approval boundary

Version0.1.0; draft, not an implementation or accepted security contract. Root
approved only the topology: three roles and two persistent process identities.
The transient authorized provisioner configures protected trust. The long-lived
observer is also the bootstrap broker/receiver. The independently managed service
host sends bootstrap material. Separate broker-to-observer delegation and
forwarded ACK are unsupported. Authority owns the shared exchange specification
and API; native-security must review noncircular OS authentication and process
reuse defenses before implementation. Existing CLI has no observer-host command.

Proposed CLI entry is `humanymous observer-host --plan PATH --plan-sha256 DIGEST`.
Names remain draft until the shared API and command contract are accepted. The
plan is bounded nonsecret protected configuration; it must not contain a bearer
credential, unseal passphrase or arbitrary process identity accepted as proof.
Its authenticated installation, service, observer role, purpose and epoch intent
must be joined to actual native process evidence using shared primitives. A hash
alone does not establish plan issuer trust or OS identity. Provisioner authority,
protected trust establishment and initial service/observer discovery remain
mandatory shared security decisions, not CLI JSON validation shortcuts.

# Lifecycle and ownership

1. Provisioner prepares explicitly authorized trust and role/epoch configuration;
   it exits without becoming ManagementPolicy.peer. Creating accounts or changing
   SCM permissions is outside ordinary CLI startup. The hosted controller owns
   temporary delegated rights and restores them independently of CLI success.
2. Observer validates its protected plan and actual process identity, enters its
   bounded startup supervisor, and binds the shared authenticated receiver. Native
   principal plus live process instance and epoch must be verified, not inferred
   from PID text, an executable path, environment or a readiness file. The actual
   observer is the eventual management client. Broker=observer is verified equality.
3. Service enters its native manager lifecycle and verifies signed launch metadata.
   Mutual authentication joins actual service and observer identities to the
   authorized installation/service/purpose/epoch. Wrong peer, changed instance,
   missing receiver or unavailable provider fails closed. No installer credential
   or previous process token is reused. The service pins the actual observer peer
   through the shared ManagementPolicy boundary.
4. If external witness is required, observer's authenticated witness capability and
   independently pinned witness key must be available before opening service audit
   writer state. Shared WitnessServiceClient/ExternalWitness typing alone is not
   evidence of native peer authentication, independence or bounded I/O. Existing
   writer uses allow_new=false; missing witness never creates replacement custody.
5. Service binds its management endpoint with a fresh Credential, then transfers
   that credential through the one-use scoped shared exchange. Observer ACK means
   the actual intended observer accepted the exact session and credential scope;
   it must not wait for public service readiness. This separates bootstrap receipt
   from RuntimeReady and prevents a readiness/ACK circular wait. Root/shared design
   must define authority for every initial endpoint/pin without trusting the ACK
   that it is supposed to authenticate. Credential payloads remain zeroizing and
   never enter argv, environment, public reports, arbitrary files or error text.
6. Only the completed authenticated exchange and required witness setup allow the
   service to publish readiness. Its independent startup watchdog stays armed
   across synchronous input, witness and ACK operations through publication.
   Observer readiness must distinguish receiver bound, bootstrap received and
   service ready; receiver existence alone proves none of the later phases.
7. For normal observer stop, an authenticated authorized stop request is sent to
   the service; observer retains witness/client availability until actual service
   drain and final audit flush are proven. Native SCM owns service lifetime; the
   observer may not terminate it as though it were a spawned disposable child.
   Then observer cancels and joins its own I/O workers, closes its clients and
   stops/reaps only children it actually owns, within independent native bounds.
8. Lost observer, broken ACK, service failure or unfinished I/O produces failed or
   unknown completion, never graceful success. Exact service failclosed response
   to observer loss must be specified by runtime before implementation. A forced
   deadline exit retains the unproven74 convention. Concurrent/repeated stop may
   not extend deadlines. Observer must not erase persistent production custody.
9. Restart requires explicit authorization of a new observer process instance and
   epoch, a fresh credential and a new service management binding. PID reuse,
   stale receiver state or secret possession alone does not authorize reconnect.
   No hot rebind, implicit fallback or split-broker mode is assumed in this slice.

# Verification matrix

All entries are pending; prior standalone lifecycle and pure7 results do not
cover these new requirements. Authored code/tests are Rust1.99.1/edition2024.

| Gate | Required observations | Scope and authority |
|---|---|---|
| Pure contract | Strict plans; independent purposes/epochs; unknown/duplicate fields; non-forwardable ACK; replay and phase-order refusal; monotonic stop bounds | Pure data/state checks only; no OS identity claim |
| Shared native authentication | Correct principals and live instances on both sides; wrong UID/SID/PID; same-PID replacement; wrong endpoint owner; reparse/link; unavailable capability; expired bootstrap | Native-security-owned actual OS cases; missing capabilities remain blocked |
| Ordinary owned process fixture | Real observer receiver and service host, fresh exact-scope credential, witness before writer bind, ACK before service readiness, independent readiness correlation | Allocated ordinary-user native component; no SCM/account changes; no hosted attestation from arbitrary inherited Write |
| Failure and cancellation | Missing observer/ACK/witness; wrong scope/purpose; replay; peer exits during exchange; blocked read/write; drain/flush failure; stop during startup; cancellation joins or nonzero unknown timeout | Actual bounded process tests, separate cause/outcome and cleanup facts |
| Genuine hosted SCM | Independent virtual-service sender and ordinary observer process; actual peer evidence; install/upgrade/rollback/replay/stop/remove; full readiness manifest; secret/state cleanup | Disposable real hosted actor with externally bound source/run and restored controller delegation; local ACT labels insufficient |
| Restart and process replacement | Fresh authorized observer epoch and service binding; old token/ACK/instance rejected; same-PID reuse denied; no custody reset | Actual instance evidence, not mocked PID strings |
| Observer/service shutdown | Witness usable through actual final audit flush; service controlled through authenticated stop/native manager; own workers joined; owned children reaped; no foreign termination | Distinct service and observer exit facts; no successful report inferred from loopback closure |
| Linux/macOS | Actual protected unattended provider, user-manager/domain and ordinary principals, same protocol/lifetime invariants | Each real OS profile independently qualified; Unsupported never counted as pass |
| Product integration | Real Core/Gate protected input, authenticated observer management and independent witness through final flush | Docker-only product e2e; a native component or fixture does not replace this gate |
| Evidence | Immutable source/run/actor binding; required and executed denominator; private raw hashes; sanitized public facts; controller/account/SCM/scratch cleanup | No blanket exit2 acceptance, global pass or ignored-test credit |

# Review findings and implementation blockers

Blue boundary: use authority/native-security shared exchange and scoped identity
types; no duplicate transport/ACL logic in CLI. Red boundary: an ephemeral
installer PID or forwarded ACK must never authorize a later observer. SRE:
separate ACK from public readiness and keep the witness alive through finalflush.
Privacy: keep secret payloads and raw native identifiers in appropriate private
evidence, not public receipts. Evaluation: preserve native component, genuine
hosted and Docker product qualification as separate claims.

Adversarial verdict: needs-work. P0: protected trust/discovery and native process
instance proof are not yet specified by the shared owner. P0: observer-loss
enforcement and bounded witness exchange/cancel/join must be specified and proven.
P1: new observer plan, command, readiness schema and typed shared API are not yet
accepted or implemented. These are required preconditions, not accepted residual
risks. Native-security independent critique and root final review must precede
implementation. This draft changes no scoring, verdict or accessibility rule.
