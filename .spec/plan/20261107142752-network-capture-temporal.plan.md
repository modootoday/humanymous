---
id: plan:root/20261107142752
title: Correlation and traffic producer placement review
status: draft
created: 2026-11-07T14:27:52Z
updated: 2026-11-07T14:27:52Z
rel:
  depends_on: [sot:root/rust-migration, sot:root/crate-architecture, sot:root/network-capture]
---

# Temporal network producer proposal

Version0.1.0. Design/source inventory only, pending Root placement review.
Root requested eight additional retirement rows beyond the20network rows.
Source inspected at734c62da0e493478bb2aa4d998664f11e432945c; no Rust source,
dependencies, scoring, retirement inventory or execution was changed here.
Existing capture actual5 proof remains evidence:root/20261107122900; this proposal
does not extend its scope. No subagents or test processes were started.

## Brief and placement

Registry definitions, residual mappings and promotion consumers do not produce
correlation or traffic history. The Rust network crate currently contains no
correlation registry, traffic store/checker or streaming MAD detector. Preserve
L5 temporal observation semantics; scoring remains in the existing frozen engine.
Do not merge client populations under a coarse server JA4 key.

Propose network domain modules, without a new crate:

- correlation/mod.rs: public typed observations, outcomes and lifecycle API.
- correlation/registry.rs: per-key subnet/session and JA4 fanout state, stats/GC/reset.
- correlation/velocity.rs: bounded arrival history and private population MAD core.
- correlation/shared.rs: semantic coordinator input/result and explicit degraded state.
- correlation/tests/{registry,velocity,shared}.rs: parity and negative boundaries.
- traffic/mod.rs plus record.rs, log.rs, consistency.rs: representation, bounded
  history/backfill and pure frozen checks; tests/{record,log,consistency}.rs.

Runtime owns exactly one application-scoped instance, synchronization, actual
connection/session admission, clock, GC scheduling and authenticated composition.
This implements the architecture's shared application owner requirement, rather
than placing a second security engine or a global singleton in network. Proposed
domain objects use exclusive mutable access; runtime controls concurrency. A
coordinator call must not retain a state lock while doing I/O. Ordering of
concurrent population samples must be defined by the owning runtime before use.
Network must not depend on runtime or capture; runtime supplies observations from
the actual retained capture owner. No new dependency/manifest change is proposed.
Existing sha2/hex suffice for hashes and a narrowly reviewed standard HMAC-SHA256
construction if needed; known-answer vectors are mandatory, no ad-hoc MAC.

## Exact eight-row responsibilities and test matrix

| Legacy row | Actual responsibility and proposed Rust owner | Required source-bound verification |
|---|---|---|
| internal/correlation/correlation.go | registry/velocity/shared: fingerprint-keyed subnet/session sets; JA4 fanout; per-key first-new-subnet arrivals; population MAD; optional atomic fleet velocity; stats, TTL GC, reset, fallback count and coordinator digest | Proxy rotation at3subnets; else shared fingerprint at5sessions; proxy precedence; fanout requires3fingerprints AND3subnets; no coarse-JA4 replacement; repeat subnet never new; empty key or pipe-only key ignored; Stats preserved for Gate credential cap; exact TTL equality/expired boundary; reset semantics; resource failure explicit |
| internal/correlation/correlation_test.go | registry parity fixtures (5existing tests), not a replacement producer | Port rotation/shared/single-session/empty-key/fanout examples; add2vs3subnets,4vs5sessions, each fanout half below threshold, duplicate IDs, empty subnet and exact emitted IDs/value/verdict/confidence/notes |
| internal/correlation/correlation_shared_test.go | shared contract fixtures (6existing tests): atomic globally-new subnet+velocity, digest, fallback observability and concurrency | Preserve semantic two-key atomic contract,40organic thenfleet50burst, outage and malformed reply local fallback, count20and8x50degradations, SHA/HMAC deterministic fixed64hex; fleet count must NEVER enter load-bearing proxy_rotation/Stats; typed unavailable required when no actual coordinator exists |
| internal/correlation/velocity_test.go | velocity parity fixtures (3existing tests) plus private MAD prerequisite | Registry and emitted ip_velocity weight0,40organic population then30subnetburst,100slow arrivals30s apart remain quiet; strict floor8/MAD score>6, warmup20/pre-append scoring, even/odd median, bounded256population/512arrivals, equality cutoffs and saturated-ring behavior |
| internal/trafficguard/consistency.go | traffic/consistency: pure intra-session observed-engine/JA4/UA/subnet/protocol checks; no history storage or authorization | Fewer than2records emits none; engine>1, stable-JA4>2, UA>1, subnet>1; mixed exact protocol strings1.1and2; stable JA4 retains a_b before second underscore; full/resumed pair2must remain quiet; c-only differences quiet; permutation/header-order/alpn_flap remain unproduced |
| internal/trafficguard/log.go | traffic/log: latest TLS record by address, HTTP TLS backfill, oldest-first copied session history,64record ring, seen timestamps, TTL GC and reset | Append65retains newest64; backfill JA3/JA4/engine/ALPN/version/ext-order but not HTTP identity; address overwrite and no match; returned snapshot mutation isolated; GC strictly age>TTL for sessions/address records; boundary equality survives; reset clears all three maps; connection-reuse/epoch negative contract |
| internal/trafficguard/record.go | traffic/record: timestamp/session/address and optional protocol metadata; IsTLS/IsHTTP; short SHA256 UA/order and legacy subnet key | Empty UA/list -> empty; first6digestbytes ->12hex; order hashes comma-joined names exactly; duplicate/order sensitivity; TLS iff JA4nonempty AND methodempty; HTTP iff methodnonempty; IPv4port, IPv6compressed/bracketed/port and malformed legacy string fixtures before any normalization decision |
| internal/trafficguard/trafficguard_test.go | traffic parity fixture suite (7existing tests), not a producer | Engine/UA rotation, stable session, c-segment permutation, TLS backfill, varying header-order no signal/no panic, IPv4subnet; add two stable JA4 resumption values then third, ring/TTL/hash/epoch/missing-input tests above |

There are14existing correlation tests and7traffic tests in these eight rows.
These are inventoried, NOT_RUN. New boundary fixtures are a matrix, not an
assertion of authored/executed Rust counts. This plan does not mark any row retired.

## Frozen details that implementation must preserve

T01. Local correlation maps keep distinct sets, refresh updated on observations,
and expire only when GC runs and age>TTL. Stats does not itself expire entries.
Sessions include the supplied session ID even if empty in the compatibility core;
production admission separately requires a real session. Empty subnet adds no
arrival. The fp|stableJA4 key remains a client fingerprint component plus server
observation; it is not an identity or original-hop attestation. JA4 fanout exists
as an API but the inspected server collect path does not call it: exposing its
producer is not permission to newly enable that predicate in serving.

T02. Local arrival list appends a first-seen subnet only while length<512 BEFORE
pruning times strictly before now-2minutes. Preserve the saturation quirk in
parity fixtures; do not silently optimize away a boundary that changes output.
Shared legacy removal is inclusive at the cutoff, unlike the local strict cutoff.
Timestamp units and this asymmetry must be explicit. Local input time ordering
is assumed by the Go prefix-prune; out-of-order/clock-regression policy is a
placement-review decision, not permission to sort or drop observations silently.

T03. MAD has one per-process population series named pop: window256, K6,
warmup20, floor1, maxkeys16. Score absolute deviation against median/MAD BEFORE
appending the new value; anomalous iff score>6, emit velocity only if count>=8
and observation is a new subnet. The code's warmup20 overrides its stale comment
claiming12. Generic anomaly defaults are not the correlation configuration.
Velocity is weight0; no added hard rule, verdict promotion or scoring adjustment.
Dependency gap: internal/anomaly/detector.go implements this math and detector
GC; there is no Rust counterpart found. Root must assign its canonical owner.
Propose a private correlation-specific core now, not a duplicated public generic
anomaly service; future generic migration must move/reuse it rather than fork.

T04. Coordinator aggregation affects ONLY new-subnet/velocity inputs. Local
subnet/session/fanout counts and Gate Stats remain local. Go clamps velocity to
0..i32MAX and recognizes added exactly1; malformed short replies/outage increment
fallback count and use already-computed local values. Local-only has no fleet
health claim and zero fallback count. Reset clears maps/population, but retains
the coordinator configuration/key and lifetime fallback counter. Preserve these
observable distinctions. A cumulative zero fallback count never proves healthy
coverage or that a coordinator observation actually occurred.

T05. Go's EVAL embeds executable Lua. Do not port that string or author another
foreign script workaround. A Rust semantic coordinator trait can describe one
atomic operation over a hashed key and subnet, deadline and typed response.
Until an actual Rust-owned backend exists, shared capability is unavailable;
an injected/test response is not a fleet producer. SHA256 or HMAC-SHA256 remains
64hex, never raw fingerprint/JA4 at coordinator rest; hashes are pseudonyms.
Live shared qualification needs a separately accepted atomic Rust backend and
real owned coordinator pilot, not two non-atomic commands or a mock-only claim.

T06. Traffic retains max64records/session and overwrites the last TLS entry by
address; sessions/address map cardinalities and string sizes in Go are unbounded.
Require explicit finite resource budgets from the owning configuration for keys,
sessions, addresses, distinct-set entries and byte lengths. Numeric budget
defaults remain unapproved. Exhaustion must report unavailable/degraded coverage,
not evict evidence and return a clean empty result. Do not cap distinct counts in
a way that changes a frozen predicate. Required missing/stale TLS, peer/session
or history coverage must prevent runtime health/allow; no new signal ID needed.
Separate observation outcomes from the existing signal vector so an empty vector
cannot be mistaken for evidence of adequate coverage. Optional weight0 fleet
fallback remains local-degraded, never masquerades as fleet-healthy; a route that
requires fleet coverage must refuse while that coverage is unavailable.

T07. Preserve the historical textual subnet/hash transformations in the pure
compatibility core. Go Subnet24 is NOT a correct general IPv6 /48 parser: it
truncates textual groups, mishandles compressed/bracketed IPv6+ports and returns
unparsed strings for other input. Replacing it with std::net normalization can
change distinct-subnet counts and verdicts, so needs explicit Root decision and
separate accepted delta. Keep valid peer provenance distinct from subnet key
compatibility; arbitrary supplied/XFF strings cannot become authenticated peers.
Header set/order hashes are not authority to enable disabled order predicates.

T08. Legacy address-only TLS backfill can cross connection epochs after address
reuse and uses clientIP in the server wrapper, collapsing proxy/client addresses.
Production Rust must consume same-owner TLS metadata and runtime's accepted
connection/session epoch. Do not add a public mark_verified or caller bool. Root
and runtime must approve whether the production key is epoch+peer while a pure
legacy model retains address-only parity; absent that contract, backfill remains
unavailable for authorization. Do not import proxy hints as original-client TLS.
Actual negotiated ALPN/version must come from capture owner, not the first offer
or an empty legacy wrapper field. Mutable records never mint a serving capability.

## Five-role review and critique reflected

Local review used the blue-architect, red-attacker, sre-ops, compliance-dpo and
evaluator role instructions; no delegated agents were spawned. Blue requires
one runtime owner and unchanged predicates. Red identified address reuse, coarse
JA4 population merging and coordinator injection. SRE requires finite budgets,
explicit fallback and deterministic GC/clock semantics. DPO requires short-lived
pseudonyms, no raw coordinator fingerprints, and no anonymity/erasure claims.
Evaluator separates producer source, unit proof, actual coordinator/transport
pilots and Docker topology. Existing traffic memory includes addresses/path and
session IDs; do not persist/export these as sanitized public evidence by default.

Adversarial verdict: needs-work before implementation. P0: a registry/residual
mapping cannot retire a producer; same-address stale TLS must not become serving
evidence (T08); missing data cannot produce a healthy observation (T06). P1:
MAD owner, atomic shared backend, resource bounds, clock/cutoff quirks and IPv6
compatibility decision (T02-T07). These become review gates, not silent choices.

## Sequence and verification

1. Root accepts module placement, MAD owner, resource-budget source, subnet
   compatibility and runtime epoch contract. Extend the work-lane paths before
   any network implementation; its current lane remains capture/spec only.
2. Author local record/check/history and correlation/MAD Rust components with
   source-bound table fixtures; no new dependencies/thresholds/predicates.
3. Root-allocated scoped locked tests, fmt and Clippy; report actual denominators.
   No Go execution or new foreign fixture-generation glue in this plan.
4. Runtime owner integrates into the one shared application lifecycle and checks
   missing input/epoch/GC/reset/budget failure before audit and origin effects.
5. If shared coordinator is required, qualify the accepted real atomic backend
   separately. Native transport/epoch pilot needs actual owner and joined cleanup.
6. Docker product gate needs exact source/images/topology, original peer scope,
   required signals, audit/origin witnesses and bypass negatives. Unit or capture
   actual5 success cannot qualify this. Do not repeat capture5 without new need.
7. Root reviews exact eight-row retirement mappings only after each real producer
   and its tests/integration are present and independently qualified. No producer
   source file is removed or declared retired by this design-only continuation.
