---
id: sot:root/rust-wargame
title: Rust wargame
status: active
created: 2026-11-06T22:24:18Z
updated: 2026-11-07T07:17:45Z
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

### Legacy observation report v0.9

Root accepted four missing baseline file rows as a separate legacy reporting
context inside attacker. This imports observations; it does not run attacks,
score reports or qualify human/AT/product execution. Existing26-case native
catalog and its follow-up remain independent.

| Frozen Go baseline | Rust replacement |
|---|---|
| cmd/report/main.go | crates/attacker/src/bin/humanymous-report.rs |
| internal/report/aggregate.go | crates/attacker/src/report/legacy/mod.rs and aggregate.rs |
| internal/report/html.go | crates/attacker/src/report/legacy/html.rs |
| internal/report/report_test.go | crates/attacker/src/report/legacy/tests.rs |

One pub mod legacy declaration in report.rs exposes this context; root explicitly
approved Cargo's automatic discovery of the new humanymous-report binary. No
manifest/lock/dependency changes. All new code/comments/tests remain Rust/English.

Keep legacy array Record keys: profile,label,run,verdict,riskScore,hardRuleFired,
outcome,top,skipped,reason,error. Preserve missing/null defaults and discard
unknown fields as Go. The intentional compatibility tightening is rejection of
duplicate decoded keys through the sole shared bounded JSON decoder. Malformed
types, nonfinite numbers and aggregation overflow fail with static diagnostics.
Group by first eligible label occurrence, bot iff label starts with bot:.
Legacy botTPR counts DENY+CHALLENGE over eligible bots; humanFPR counts only DENY
over eligible declared-human rows. Keep mean risk and verdict/rule/top occurrence
counts; sort maps deterministically and top counts descending then IDs ascending.
The old fixture has four metric rows and one additional skipped camoufox row.

Expose scheduled, eligible, error, skipped and missing-label accounting separately;
retain legacy metric denominators exactly. Count error flags even on skipped rows
separately where necessary. Unsupported is not encoded by this schema: null/
not_encoded, never inferred zero or guessed from reason. Human CHALLENGE is not
ALLOW. Missing populations and incomplete/unrecorded coverage cannot yield a
product-green banner or a verified-human FPR claim.

Bounds: input8MiB,50000rows, each decoded string4096bytes, top vector128entries,
caption4096bytes, path4096bytes, rendered output16MiB. Render entirely before
publication. Escape every dynamic HTML text value, use no scripts/external assets,
and never emit raw error/reason values. Output uses bounded32collision retries,
create_new-owned temporary, write/sync-before-native-rename, and cleanup of only
that newly created temporary. Never delete a destination or foreign collision
file to make rename succeed. Windows replacement/share-denial behavior and actual
filesystem publication still need allocated native qualification; source alone
does not establish crash durability or all filesystem semantics.

Serial blue/red/SRE/privacy/evaluator critique reflected: reuse shared decoder;
retain failed denominators; reject key ambiguity and HTML injection; bound heap/
output; preserve old output before commit failures; redact free-form diagnostics;
label declared-human metrics and unknown unsupported coverage honestly. Pure
parity/rejection/escaping/determinism tests precede an allocated actual CLI file
pilot. The later allocated Windows L1 gate passed8selected pure tests, with2native
tests ignored and17other library tests filtered. All-target package check and
strict Clippy passed; the CLI test was compiled, not executed. Evidence154600
binds basee34cee8a plus exact overlays and2715source files with0differences.
Registry/tool/config closure and real file publication remain unmeasured. This
report candidate is distinct from the hosted306eedc8 generic26 execution.

## Why

Actual hosted checkpoint306eedc83801ad153bf8aed68d89a8b1dbb30847/run42387564172
passed scoped strict-byte validation:26scheduled/19evaluated/7unsupported,
0errors/skips/expectation mismatches. Evidence154100 independently recovers the
public receipt, source/raw/Lab-manifest binding and0..25indices. The owning
runtime-profile repair and shared decoder are exercised at this source. Full
product remains false and the outer gate blocked; original813failure is retained.
Private original report bytes are unavailable locally, so the original SHA256
is CI's independently checked file binding, not a local recomputation claim.

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

### Terminal execution accounting v0.8

An evaluated case completed its request sequence without transport errors; a
regression mismatch is distinct from a failed request. Never relabel a transport
failure as evaluated/mismatched. A skipped case was not attempted and therefore
has no process, step or effect observations. Failed execution may retain an
honest step prefix, including complete HTTP observations followed by an audit
failure. Its first transport failure must terminate that prefix, have a nonblank
safe error and have no response verdict. These are the existing executor's
terminal-state invariants, not changes to scoring or HTTP expectations.

V2 fields and the26-case catalog remain unchanged. Source audit preserves all
seven missing participant/producer cases as unsupported, including ordinary
human, AT, accessible Pass and coherent T4 controls. Automated browser or frozen
fixture activity never supplies human/AT participation. New pure negative
fixtures construct failed ledgers only; no runtime success is manufactured.
Root-assigned28pure checks and scoped strict Clippy passed on the recorded graph
(evidence120401). Owned284d24cf bytes match; newer audit dependencies and2doc-only
drifts are retained explicitly. Actual product validation remains pending.

### Standalone service readiness v0.7

Expected child capability metadata comes from the runtime-owned
service::standalone_profile(&Configuration), shared with ServiceHost's builder.
Configuration::readiness() is only the core capability view and is not the full
service envelope. Retain exact whole-object equality, private owned-child/image/
namespace binding and all frozen enforcement. Reject extra fields, false witness
claims and changed scope/listeners. The actual813hosted receipt (evidence083903)
preserved26cases but measured19errors/0evaluated/7unsupported; the source-proven
profile mismatch is repaired separately and requires fresh validation. No raw
private case reasons were recovered, so the individual failure causes are not
asserted as directly observed. Missing producer/human/AT qualifications stay open.

The743494c1 consumer/runtime709b0e2 repair passed27selected pure checks and scoped
all-target Clippy under an allocated slot (evidence085126). Captured all-crate and
root build-metadata bytes stayed equal before/after; Git history changes outside
that scope are recorded. This does not replace the pending fresh hosted repair.

### Shared syntax boundary v0.6

Root-approved extraction moves the sole recursive decoded-key visitor to
humanymous-test-support::json::decode_strict_json(bytes,max_bytes). Attacker uses
one report adapter with its unchanged4194304-byte budget and safe LabError
messages. The common decoder rejects oversize input before parse and retains
duplicate-decoded-key, default depth, finite-number and trailing-input rejection.
Schema/fieldset/replay/qualification remain specific to each consumer. No
duplicated visitor is kept in attacker. Root integrates the approved dependency
graph. New allocated28pure checks and scoped Clippy passed (evidence075756).
Actual13-crate snapshot and before/after raw hashes expose3changes outside the
selected runtime configuration; whole graph was not immutable. Consumer/shared
decoder/lock remained stable. Source-bound hosted qualification remains pending.
Historical v0.5 results do not qualify this changed graph.

### Step attribution acceptance v0.5

The network verifier compares step identities, ordering and roles against the
owning catalog, including setup and synthetic controls. Evaluated rows require
the complete ordered plan; execution errors may preserve any genuine prefix,
including an empty prefix when installation/session setup failed. Relabelling a
replay control as an attack, reordering steps or appending steps fails report
acceptance independently of HTTP status and policy expectations. This strengthens
evidence attribution without changing frozen detection or the v2 wire fieldset.
New pure regression fixtures remain failed ledgers, never fabricated successful
runtime observations. V0.5 passed26pure tests and scoped Clippy on first attempt
under an allocated slot (evidence:root/20261107073058). Observed HEAD7cda20c
included75d7c82; other dependency edits were dirty and no immutable whole graph
was captured. Fresh source-bound hosted integration remains pending.

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
