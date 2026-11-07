---
id: sot:root/rust-wargame
title: Rust wargame
status: active
created: 2026-11-06T22:24:18Z
updated: 2026-11-07T06:59:31Z
verify:
  - cargo test --locked -j1 -p humanymous-attacker -p humanymous-defender -- --test-threads=1
rel:
  governs: [crates/attacker, crates/defender, test/rust-wargame]
  depends_on: [sot:root/rust-migration]
---

# Rust wargame

## Rule

Two separate evidence modes exist: deterministic **pure-engine simulation** and
the bounded **owned-process network pilot** below. Neither qualifies full Gate,
browser, Pass, real human/AT, remote installation identity or all-product e2e.
The attacker crate owns an extensible, versioned catalog of synthetic observations
and replays the existing frozen Go-oracle fixtures. The defender invokes the
existing Rust `humanymous_scoring::engine::Engine::score_with_trace` unchanged.
Neither crate implements detection policy or alters weights, thresholds or rules.
Registry constructors supply scores for synthetic observations. Frozen reports
are read directly from scoring's canonical fixtures, never copied or regenerated.

Each scheduled case has a stable ID, tier T0–T4/control, seed, provenance, input,
expected regression outcome and exactly one terminal status: evaluated, error,
skipped or unsupported. A seed deterministically orders the catalog; it does not
claim stochastic coverage. The report includes actual score and complete trace,
input reports, attempted/evaluated/error/skip/unsupported denominators, and counts
ALLOW, CHALLENGE and DENY separately by class. Errors never become ALLOW or a
successful block. Regression mismatches fail the CLI; known unsupported runtime
cases remain visible and make runtime coverage incomplete. No percentage may
drop errors or unavailable cases from the scheduled denominator.

Human controls count explicit ALLOW separately from CHALLENGE and DENY. The
zero-motor baseline and its extended observation window exercise accessibility
regression risks without claiming measured human false-positive rates. A T4
coherent-observation replay is a ceiling control, never a detected bot success.
Pass token issuance/verification, actual accessibility/browser behavior, replay
storage, HTTP enforcement and observed network fingerprints require runtime
integration and must remain unsupported here.

### Isolation and installation binding

The simulation CLI has no target/URL option and opens no sockets. The engine pilot
runs as an unprivileged read-only Docker container with `network_mode: none`, no
published ports, no Docker socket and no host mounts. Build dependency downloads
are outside the execution sandbox. Both packages are `publish = false`; attacker
sources and binaries must never enter normal installations, npm packages or
public defender runtime images. The local pilot image is explicitly lab-only.

A future runtime runner MUST use an internal Docker network with no gateway to
the internet and only fixed service names. Operator-provided arbitrary URLs,
redirects, DNS rebinding and externally published ports are forbidden. Before
any probe it MUST authenticate the target's explicit installation identity using
a per-installation pinned public key and a fresh signed nonce. A matching string,
private IP or container hostname alone is insufficient authorization. Identity
failure, missing identity, route escape, redirect or egress-test failure aborts
before attacks. Remote target execution remains unimplemented. No unauthenticated
pseudo-binding API or arbitrary URL/port option is supplied.

### Owned-process network pilot v0.2

This differs from remotely identifying an existing target. The trusted lab
supervisor owns preprovisioned sealed custody, configuration, origin, runtime
child, private startup/readiness pipes and authenticated audit recovery. It may
send real HTTP to only its own freshly spawned child in the same network-none
Docker namespace. This is not remote signed identity, a browser run or an
installation trust decision.

The defender's OwnedRuntime spawns only the fixed sibling humanymous-runtime,
without a shell or inherited environment. Before spawning, require Linux with
only loopback interfaces and no non-loopback routes. Validate shared runtime
configuration, private-pipe readiness profile, exact loopback listener, live
child executable hash and network namespace. Repeat namespace/liveness checks
before requests. Bound I/O by size and overall deadline, never follow redirects,
and expose no caller-provided target, PID, executable or socket attachment API.
Drop kills and reaps the owned child.

The client calls production session::canonical, seed and compute_token; no
signer, detection or challenge verifier is copied. Keys/session capabilities have
no Debug/Serialize surface and are absent from case reports. This is a lab
client, not a browser bootstrap API.

The attacker owns scheduled cases and complete case/step ledgers. Before/after
each child case, the lab supplies a snapshot constructed from the authenticated
audit::Log and independently counted owned origin. Require unchanged chain
prefix, contiguous sequence and nondecreasing origin count. Read CHALLENGE/DENY
from actual audit, never infer from HTTP403 alone. Retain every scheduled case
when setup, transport, audit or expectations fail. Cover fixture ALLOW/DENY,
zero-motor HR-12 CHALLENGE, missing evidence, forged server Pass, RIT mutations,
replay/restart and required-capability refusal. Assertions remain synthetic.

Real browser/human, AT, accessible Pass and signed remote identity cases stay
unsupported until their actual producers/participants exist. Preserve T4 and
existing HR-12; CHALLENGE is not successful human access. Receipts bind actual
supervisor/runtime binaries; outer Docker evidence also binds source, image and
resolved topology. A Rust verifier checks scheduled IDs and observations. Seed
reproduces ordering, not session entropy, clocks or audit hashes.

Adversarial review: P0 arbitrary targets/host execution are removed; remote
binding remains unavailable. P1 private readiness is not attestation and full
product qualification stays false. P1 collect forgery is not Pass success and
HTTP403 must not hide human CHALLENGE. Missing origin/audit evidence is an error,
not a successful block. Independent durable witness/cross-network capture remain
separate gates. Blue reuses production owners, red retains denominators, SRE
owns cleanup, privacy omits credentials, evaluator preserves human/T4 limits.

### Design review and residuals

Blue: reuse the sole verdict engine. Red: synthetic inputs do not prove collection
or enforcement; expose unsupported cases. SRE: network-none pilot cannot prove
the future internal topology or installation binding; require signed-nonce and
egress negative pilots before runtime enablement. Privacy: only frozen/synthetic
reports, no live identifiers. Evaluator: distinguish challenge from human allow,
retain missing coverage and T4. Adversarial critique: runtime identity and escape
tests remain P0 blockers for enabling a remote network runner. The newly scoped
owned-child pilot has the different trust contract above. No motor speed/richness
requirement is added.

## Why

The migration requires replayable red/blue evidence while full Rust Gate is not
available. Success-only mock numbers would hide missing runtime responsibilities.

## Verification

Unit tests exercise actual engine replay, deterministic seeds, accounting,
negative CLI options, and human/ceiling distinctions. Run Cargo tests and Clippy
in a temporary isolated workspace until the root owner integrates membership.
Run the lab-only Docker engine pilot with no network and retain raw JSON privately.
Full runtime e2e remains subject to the existing Docker-only authority.

The v0.4 exact exported APIs, JSON fields, exit codes, accepted Compose/runtime
ownership agreement and implementation/validation state are preserved in
crates/attacker/INTEGRATION.md. Network schema is humanymous.wargame-network.v2;
v1 simulation remains compatible except Cargo invocation now specifies --bin.
No existing scoring, RIT bytes or runtime wire contract changes.

V0.3 also exports verification::verify_bytes: v1 input is checked by actually
replaying the current pure Engine/catalog, v2 by validating the actual measured
ledger. Verifier output retains its v1 schema and adds explicit mode. Equivalent
integer/float JSON numbers are accepted without discarding exact integer
comparisons. Unknown schema, malformed/oversize input and source-replay mismatch
fail closed with safe errors. Remote/product qualification cannot be enabled by
this verifier. Historical reports are not automatically claims about a changed
engine. The old Python helper is not invoked; retirement belongs to its owner.

Isolation evidence requires the complete IPv4 route header as well as no routes;
empty or malformed evidence is not no-egress proof. Startup reader allocation
failure is a typed error that drops/reaps the already-owned child. Concurrent
execution and audit-snapshot failures are both preserved in the error reason.
These additions passed25pure tests and scoped Clippy with the v0.4 decoder;
fresh hosted runtime qualification remains pending. Execution policy is
defined only by [[sot:root/program-roadmap]] and [[sot:root/delivery]]: local
classified pure fastchecks are resource-bounded; actual child/network/Docker/
browser/Cross-OS work moves to standard hosted execution. No local runtime run
is resumed. No prior green qualifies this source; unavailable participants and
full product requirements remain explicit until directly measured.

### Report decoder acceptance v0.4

Before Value construction or schema selection, reject duplicate decoded object
keys at every depth. Escaped aliases are the same key; dynamic dictionary keys
are not exempt. V2 owned report DTOs deny unknown fields; V1's complete replay
fieldset comparison rejects unknown/missing fields without altering production
signal DTOs.47499a5 did not enforce these two properties; its typed v2 decode
only caught duplicate known struct fields, while v1 lost duplicates. This
explicitly tightens report acceptance and leaves detection/RIT unchanged.

Do not call the result canonical bytes/JCS: whitespace, property ordering,
equivalent string escapes and compatible mathematical JSON numeric spellings
remain accepted. Seed requires an unsigned integer. V1 is current-engine replay
agreement; V2 is internal consistency with the expected catalog and minimized
observations. The verifier cannot authenticate arbitrary supplied hashes or
verify a full WAL signature chain from its projection, nor prove actual runtime,
human/AT or historical-source execution. External source/image/generation
evidence remains required. Unknown-schema, duplicate-key, unknown-field and
replay failures use safe static errors. No production policy or signal changes.

Official [Rust1.99.1 TcpStream](https://doc.rust-lang.org/std/net/struct.TcpStream.html)
documents bounded connect/read/write calls. Per-call timeouts alone do not bound
slow-drip total duration, so the client recomputes an overall deadline each I/O.
Linux [proc documentation](https://docs.kernel.org/filesystems/proc.html) describes
local process observations; these do not establish remote attestation or
resistance to a hostile host administrator.

Docker's [none driver](https://docs.docker.com/engine/network/drivers/none/)
creates only loopback; the [Compose service contract](https://docs.docker.com/reference/compose-file/services/#network_mode)
provides `network_mode: none`. A normal [bridge](https://docs.docker.com/engine/network/drivers/bridge/)
provides external access by default, so a bridge alone is never isolation evidence.
These official references were checked on 2026-11-07 (Asia/Seoul).
