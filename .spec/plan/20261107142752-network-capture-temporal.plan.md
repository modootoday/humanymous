---
id: plan:root/20261107142752
title: Correlation and traffic producer placement review
status: active
created: 2026-11-07T14:27:52Z
updated: 2026-11-07T14:59:00Z
rel:
  governs: [crates/network/src/lib.rs, crates/network/src/history/**, crates/network/src/traffic/**, crates/network/src/correlation/**]
  depends_on: [sot:root/rust-migration, sot:root/crate-architecture, sot:root/network-capture]
---

# Temporal network producer proposal

Version0.4.0. Root accepted pure temporal producer placement. Combined corrected
network source passed72units (58temporal plus14existing), fmt/all-target check/
strict Clippy under the exact graph/overlays in evidence:root/20261107153100.
Runtime integration, real atomic coordinator, verified epoch backfill and actual
current-source provider/product qualification remain open.
Root requested eight additional retirement rows beyond the20network rows and
subsequently assigned the two anomaly prerequisite rows to this owner.
Initial inventory inspected734c62da0e493478bb2aa4d998664f11e432945c without source
changes. The subsequent accepted source slice is recorded below. Dependencies,
scoring, retirement inventory and execution remain unchanged by this continuation.
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
- correlation/velocity.rs: bounded arrival history, reusing history::anomaly.
- history/mod.rs and history/anomaly/{mod,config,tests}.rs: ONE generic bounded
  keyed median/MAD history core, no verdict or scoring authority.
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
Root accepted ONE generic network::history::anomaly core, reused by correlation;
this supersedes the initial private correlation-specific proposal. Do not fork it
when another consumer needs keyed measurement history.

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

## Accepted placement and first pure history source slice

Root accepted network placement with one runtime composition owner, legacy text
IPv6 compatibility without a normalization delta, finite configured bounds and
explicit exhaustion Unavailable, and typed optional shared local-degraded results.
The atomic shared backend remains deferred; semantic trait alone is not fleet
coverage. Rate limiter migration is a separate Root design and not in this slice.
Runtime epoch/backfill integration remains pending and no runtime wiring is
authorized by that acceptance. At the first slice the eight original rows were
still unimplemented; their subsequently authored local source is recorded below.

Two added source rows and exact first implementation paths:

| Legacy row | Rust owner | Verification matrix |
|---|---|---|
| internal/anomaly/detector.go | crates/network/src/history/anomaly/{mod,config}.rs, exported through history/mod.rs and network/src/lib.rs | Keyed bounded sample histories; median/MAD, strict score>K before append, warmup, floor, oldest-key eviction, explicit GC; no signals/verdicts; finite input/config checks and arithmetic overflow handling |
| internal/anomaly/detector_test.go | crates/network/src/history/anomaly/tests.rs | Port all5legacy cases: stable, outlier, warmup, floor, bounded cardinality; add exact K equality, even/odd median, sample ring, TTL equality, oldest eviction/tie determinism, no-mutation regression/error, key/byte limits, reset and invalid numeric config/input |

First slice is these five Rust files only (network lib export, history/mod.rs,
anomaly/mod.rs, anomaly/config.rs, anomaly/tests.rs). Correlation/traffic modules
are reported paths for later pure implementation; they must reuse this core.
No manifests/dependencies/scoring/production defaults, runtime wiring or execution.

Caller supplies every detector Config value: window, K, warmup, MAD floor, and
Limits(max_keys,max_window,max_key_bytes,max_payload_bytes), all finite/positive
where appropriate. No Default constructor or implicit Go fallback. Correlation's
already-frozen256/6/20/1/16 profile is a later caller configuration, not a new
generic production default. Warmup may exceed window (legacy never-warm behavior).
Byte admission accounts reserved sample payload (window*f64 size) plus key bytes;
allocator/map metadata is excluded from that byte number but separately bounded
by finite key/window/individual-byte limits. Checked arithmetic before mutation.
All resource limits come from owning configuration; no invented production caps.
Correlation/traffic will additionally require configured set/session/address and
record-string bounds before their source is authorized.

Time is caller-supplied ordered Duration ticks with no wall-clock reads. Observe,
GC and reset reject now<last accepted time as explicit Unavailable BEFORE any
state change. Equal ticks are allowed. GC removes idle keys only when age>TTL;
TTL is caller supplied. Reset clears samples but retains the clock high-water
mark; a new epoch requires a new detector, not replaying older ticks into reset
state. This rejects legacy out-of-order input as an explicit compatibility delta
accepted by Root. NaN/infinite values/configs and non-finite derived arithmetic
also fail before mutation rather than generate a healthy zero score.

Oldest-key eviction is a legacy generic anomaly behavior, with lexicographic tie
break for deterministic equal timestamps (Go map ties were unspecified). The
returned measurement identifies any evicted key, so loss of history is explicit;
a future required-history caller cannot treat eviction as continuous coverage.
Exhaustion of key length/window/payload budget returns Unavailable; it does not
silently truncate samples or manufacture a normal measurement. New-key admission
and possible eviction must be preflighted together; failure preserves old state.
This generic anomaly eviction is not permission to evict correlation sets and
report clean low counts. Measurement warmup is explicitly visible in the result.

After this updated plan passes scoped checks and exact paths are reported to
Root, pure history source implementation is authorized. Every authored test is
NOT_RUN until a separate Root allocation. Qualification and retirement claims
remain separate from source implementation.

First source slice is now authored in the five reported files, with12pure tests
(the5legacy behavior cases plus7boundary/state tests). None has been executed;
fmt and diff checks only. No generic Go default is copied. Equal-time eviction
ties are lexicographic; unlike Go's empty-string sentinel bug, an empty oldest
key is actually evicted and reported, preserving the configured cardinality bound.
This is a boundedness compatibility delta alongside explicit regression refusal.
Payload accounting reserves exact window capacity plus key bytes; map allocator
overhead and transient sorting copies remain separately bounded, not included in
the byte counter. This does not claim allocator OOM recovery or a heap-RSS limit.
Root's permission for this pure source slice does not authorize runtime wiring,
new scoring, added dependencies or an actual workload. GenericMAD reuse by
correlation remains the next source stage after this common core is reviewed.

## Rate history: two additional rows awaiting contract review

Root assigned rate design after reviewing37936ee0 as ready for independent
source review, not execution proof. Source inventory at
cd06355b09e7a4ed93b52b16909572d25485c99d. Existing Authority transport Admission
is a per-instance one-second fixed-window connection gate using Instant; it is
NOT an arbitrary fingerprint-keyed rolling timestamp history, configurable
window/soft/hard classifier or TTL store. It cannot retire these two rows.

| Legacy row | Proposed Rust paths/responsibility | Required matrix |
|---|---|---|
| internal/abuse/ratelimit.go | history/rate/{mod,config}.rs: bounded keyed sliding request timestamps; observe, inclusive soft/hard classification, hot configure preserving retained hits, TTL GC scan, reset | Keep timestamps with age<window (cutoff equality expires), append current hit then count; hard>= cutoff first, then soft; configure retains all existing lists and next observe trims only that key; TTL=window*6 checked; GC age>TTL, scan at most4096; no hidden default activation or score output |
| internal/abuse/ratelimit_test.go | history/rate/tests.rs: port5legacy tests with ordered caller time, plus negative boundaries | Twelve fast hits ->hard;12spaced hits ->normal; independent keys; empty key ignored; hot configure preserves hits and immediately changes count classification. Add exact soft/hard/cutoff/TTL equality, bounded GC progress, budget failure preserving all hits/config/clock, bad config/overflow, configure regression, expansion history loss and reset high-water |

Root accepted this source-only code delta after reviewing R01-R07: add
crates/network/src/history/rate/{mod,config,tests}.rs; update history/mod.rs to
export rate and hold ONE shared Unavailable/ordered-time helper; update
history/anomaly/mod.rs and its tests to reuse that helper, retaining the public
anomaly::Unavailable re-export. No duplicate clock/error implementation, no
manifest/dependency/root source changes, no runtime or Authority edits.

R01. Every rate policy comes from caller configuration: nonzero Duration window,
soft and hard positive counts with hard>=soft, matching the existing Gate settings
validation in internal/gate/settings_runtime.go. No comments' demo60/120 defaults
or server40/80 values become generic defaults or activation. Equal soft/hard
remains valid and hard classification wins. No registry/score/hard-rule changes.
Reject formerly representable zero/inverted thresholds and zero/overflowing
window as explicit invalid-configuration deltas, never reinterpret them as disable.

R02. Propose immutable construction-time resource Limits: max_keys,
max_hits_per_key, max_key_bytes, max_window, max_retained_payload_bytes and
gc_scan_budget (positive and at most the legacy4096scan ceiling). No production
numeric defaults. Counters reserve bounded timestamp payload and key bytes;
map/deque allocator metadata is excluded but finite via cardinality bounds.
Window/TTL arithmetic and admission must be checked before mutation. An existing
key may prune expired hits and admit one hit only if the resulting state fits
all limits. No saturation at hard, dropped recent hits, fresh-key eviction or
clear-on-error, since each can lower a load-bearing rate. Budget exhaustion
returns Unavailable with the prior state/clock intact; required runtime coverage
must not become healthy from a zero count. A separate explicit GC can reclaim
expired keys. Hot configure cannot silently shrink immutable resource limits.

R03. Observe/configure/GC/reset share the accepted MAD caller Duration clock:
nondecreasing ticks, equal allowed, any regression ->Unavailable before mutation.
No wall clock, background task or internal lock. Legacy PerKeyIsolation test
inserts the second key at an earlier time than the last first-key hit; migrate
that fixture to equal/latest caller time to preserve its isolation purpose, and
add a separate rejected-regression fixture documenting the accepted delta.

R04. Empty key keeps the legacy no-count behavior as a typed IgnoredEmptyKey
outcome, not a measured normal Level0. Check time first, but an ignored key does
not create a series or advance the accepted high-water mark. Missing required
fingerprint must be handled by runtime coverage, not guessed from an empty key.
Result carries count and enum level Normal/Soft/Hard, not a verdict/signal/ban.
Standalone level(count) uses current explicit policy and does not mutate clock.

R05. Configure validates its complete candidate and caller time before committing,
updates window/soft/hard/derived TTL together, and never clears/prunes existing
hits. Tightening cannot buy an empty counter. Increasing a window cannot recover
already discarded older hits; expose that limitation in a ConfigurationEffect
with window_expanded/history_incomplete rather than claim complete retrospective
coverage. The next observation still counts the actually retained hits exactly.
Runtime must decide required-coverage readiness for expansion; this pure module
does not mint health or origin authority. No implicit replay/backfill samples.

R06. GC scans at most configured budget and never beyond4096keys per call.
Propose an ordered-map cursor with bounded key storage, advancing past scanned
keys and wrapping without visiting a key twice in one pass. This replaces Go's
random map-subset selection with explicit deterministic progress for a stable
key set; record this GC-order compatibility delta. Remove a key iff its latest
hit age>current window*6, equality survives. Report scanned/removed counts;
one bounded pass is not a claim that all stale keys were reclaimed. Deletions
must release accounted payload. Reset clears hit histories and GC cursor only,
retains policy/limits and accepted clock high-water, and requires valid now.

R07. The shared clock/error refactor must preserve the12MAD fixtures and its
public paths, with no second generic helper or default. Anomaly eviction remains
explicit legacy measurement behavior; rate exhaustion never uses that eviction
policy. These are distinct domain state semantics, not a generic eviction store.

Status: Root accepted R01-R07 and the reported paths; pure rate source and the
single clock/error refactor are now authored with12rate tests. All current24MAD/
rate fixtures are NOT_RUN for this new candidate. The original
8correlation/traffic rows and this new rate pair remain unretired. Subsequent
source/locked tests/Clippy/native and Docker qualification require their actual
separate evidence and Root allocations. Existing actual capture5 is unchanged.

The rate observation Coverage describes expansion gaps only, not general health
or completeness of request capture. Unavailable admissions must remain missing
coverage in the consuming runtime; later successes do not attest that failed
admissions were recorded. Reset retains a pending expansion gap as well as the
clock and policy. No runtime wiring or default activation was added.

## Original MAD candidate: actual selected validation

Root allocated D for immutable37936ee0, offline/jobs1/incremental0 using exclusive
target/validation-capture-b-edf700bc. The first archive setup failed due to argument
construction before any Cargo/test; it was preserved, corrected after Root retry
allocation, and not mislabeled a source failure. Fresh actual admission29.32GiB
free/82%commit. Selected history::anomaly12passed0failed0ignored14filtered; network
all-target check, strict Clippy and fmt each exited0. Entire2614source files,
48registry source directories/2316files and4toolchain binaries unchanged before/
after; no newly referenced registry roots. D returned immediately. Evidence is
registered as evidence:root/20261107145000. These results do NOT qualify the later common
clock/error refactor or rate source and do not mark all26network units executed.

## Accepted next producer source bounds

Root requested the original eight pure producers after the shared primitives.
Traffic paths are traffic/{mod,record,log,consistency,tests}.rs; correlation paths
are correlation/{mod,config,registry,velocity,shared,tests}.rs. Extend the current
lane to these paths before writing. Reuse history's one ordered clock/error;
correlation reuses the generic MAD rather than implementing median math again.
No runtime wiring, production defaults, scoring or new dependencies.

Traffic configuration supplies TTL plus positive max_sessions, max_addresses,
max_field_bytes and max_payload_bytes. The frozen per-session ring remains64.
Retained payload accounting includes duplicated map keys/record strings, not
allocator metadata or bounded snapshot copies. Record/replace/admission is
preflighted before internal mutation; exhaustion/time regression ->Unavailable.
Legacy TLS address backfill explicitly returns AddressOnly association and
unqualified coverage, never a verified epoch. Absent records/fields cannot become
healthy from an empty signal vector. Production epoch binding remains runtime's
pending owning contract, with no caller bool or fabricated proof here.

Correlation configuration supplies TTL and finite max_keys, max_fanout_keys,
max_distinct_per_set, max_field_bytes and max_payload_bytes plus explicit generic
MAD resource limits sufficient for the frozen population profile. Preserve local
subnet/session/fanout thresholds and512arrival resource behavior. Shared semantic
types have no real backend and no fleet-qualified output: externally reported
counts affect only weight0 velocity, retain explicit unqualified/degraded status,
and cannot change local Stats/proxy rotation. A named coordinator trait describes
the atomic operation without executing foreign scripts or pretending to connect.
Hash/MAC inputs stay transient; field and payload budgets precede state mutation.
Sorted deterministic set-value order replaces Go map's unspecified enumeration;
do not change set membership, counts, signal order, predicates or notes.

The accepted local producer source is now authored: traffic12tests plus
correlation20tests (17public-contract cases and3private arrival/budget cases).
Together with the later MAD12/rate12 shared-clock candidate,56temporal tests
are authored for the new source; all are NOT_RUN for this combined candidate.
The pre-existing network14tests are separate. Original37936ee0 selected MAD12
proof is historical and does not transfer to this combined source.

Traffic preserves optional field skip predicates but only reports Insufficient
or UnqualifiedHistory coverage; log backfill returns Absent/AddressOnly, never a
verified TLS epoch. Stats returns Absent explicitly rather than manufacturing a
healthy zero count. CoordinatorReportedUnqualified is not fleet health; the
atomic trait has no real implementation. Historical EVAL wire/backend testing
is therefore deferred, while local fallback/count/clamp behavior has authored
fixtures. No shared backend row is retired based on those injected counts.

Correlation uses ONE history::anomaly detector with fixed256/6/20/1 population
parameters and caller finite limits. Tests include SHA256 and RFC4231 HMAC-SHA256
known-answer vectors, strict local cutoff equality, append-before-prune512cap,
fallback retention across reset and runtime-owned concurrency serialization.
Runtime policy still must preserve missing coverage after any Unavailable;
subsequent pure observations do not prove failed admissions were recorded.
Neither the fanout API nor new history observations are connected to serving.
No dependencies, scoring/threshold/registry changes, runtime wiring, actual
coordinator or Docker execution were authored or launched in this continuation.
Source formatting/diff/spec checks are the only checks run for the new candidate.

## CA-HISTORY-LAYOUT-01 constructor correction

Independent source review identified a resource configuration gap: multiplication
that fits usize can exceed Vec's isizeMAX allocation-layout limit. For both f64
samples and Duration hits, count=floor(isizeMAX/element_size)+1 passed the former
checked multiplication while Vec::with_capacity would reject its layout.
No huge allocation or OOM experiment was performed. Both Limits and Config now
validate std::alloc::Layout::array and return InvalidConfiguration before state
construction. Two allocation-free constructor boundary fixtures cover the largest
valid count and first invalid count; neither constructs a detector/limiter or
allocates sample storage. Current temporal fixtures58plus existing14means72
authored total, NOT_RUN for this corrected candidate. Earlier source6c72a47f and
original37936ee0 proof remain distinct. This is layout validation, not ordinary
allocator OOM recovery or a heap/RSS guarantee. No policy/default/dependency change.

## Complete current L1 gate and first failures

Root allocated the lowest ready gate on e1ed50ce network plus graph173008f0,
offline/locked/jobs1/incremental0 and the exclusive warm capture target. Fmt
passed first. Compile then exposed3incorrect root signal namespace imports;
0tests ran. Own consumers were corrected to canonical observation exports and
committed0429731c, without changing shared signals. Separate immutable attempt2
passed72units/all-target check, but strict Clippy failed82missing API docs. The
documentation was supplied without suppression and a separate attempt3 passed
fmt, all72units (0failed/ignored/filtered), all-target check and strict Clippy.
Evidence:root/20261107153100 retains each failure, command and actual executed
source/overlay hashes. Each2685file tree stayed unchanged; selected registry2316,
tools50 and configuration18records stayed unchanged. Target returned at terminal.
Earlier NOT_RUN notes are historical for those source checkpoints, not the final
corrected L1 candidate. No provider/Runtime/fleet/Docker readiness is promoted.

## Accepted package baseline: prepared exact-path handoff

Following the human's explicit eight-public-package approval, Root alone
sequences network/capture into the accepted humanymous-runtime context. This
writer prepared evidence:root/20261107200910:52tracked file hashes,31owned and21
read-only context files, clean owned source, no live handles/target/Git lease.
The handoff records overlapping TLS/H2 names, canonical observation exports,
feature/dependency/visibility and unsafe-boundary hazards, old proof scope and
required new-path verification. It moves no path or ref and changes no manifest.
The existing claim remains until Root freezes and explicitly sequences this slice;
then narrow/release actual transferred paths. Real fleet, same-epoch association,
Runtime serving and Docker gates remain with their current responsibilities.

## CORE-SLICE-01 consumer references

Root assigned the mechanical Network Rust import edit, including a source loan
for residual/report/tcp/http beyond this writer's former temporal-only paths.
The verified own lane was released/reclaimed with Network src/tests/examples
scope before edits. Eight files/nine references were changed only from
humanymous_signals to humanymous_core::signals; suffixes and aliases remain.
Evidence:root/20261107201405 binds exact before/after hashes and literal-only
checks. No old matching crate references remain in this Rust source scope.
No other assigned legacy core-context names matched. No behavior/visibility,
manifest, path/ref or fixture change; no compiler/Clippy/full CI was launched.
Root owns common graph and relocation and must declare coherence before tests.
Prior72unit/5hosted proofs are not transferred to the changed consumer tuple.

## RUNTIME-SLICE-02 read-only owner disposition

Evidence:root/20261107203158 records52current file hashes and all5Network
pub(crate) declarations, each requiring Network-context-only visibility after
the move. Capture has0former pub(crate); private transport/temporal fields and
relative pub(super) remain narrow. Preserve Capture->Network direction and map
old crate:: paths to capture/network rather than gateway or root aliases.
No sibling visibility exception is needed for the inspected helpers. Root must
settle the external hosted fixture's public capture path/test target without
exposing owner constructors. Exact72pure/5actual test anchors and remaining
epoch/fleet/Runtime/Docker gates are recorded. No source/import/manifest/move or
compiler action occurred; the existing source claim stays until Root's freeze.

## Selected formatter repair after CORE-SLICE-01

Root reported the Nativeguard batch's final global fmt failure at correlation/
tests.rs::oracle_velocity_remains_weight_zero. Selected cargo fmt -p
humanymous-network -- --check reproduced exit1: the longer core-qualified lookup
needed one line wrap. cargo fmt -p humanymous-network changed ONLY
crates/network/src/correlation/tests.rs; selected fmt check then exited0.
Diff inspection and whitespace-normalized content comparison confirm no token,
string, behavior or visibility change. No other package/global formatter or
manifest was modified. No unit/compiler/Clippy rerun was performed or inferred.

Exact source SHA256 before:
b958f9195c6c23fc415e91f406a617e8e8a104c021d007371995e953b64b0298
After:
92693b32f37ff9a1418498b5eba8e0da985eff1403a61aacb59bba269a32097a

This formatting-only checkpoint preserves earlier source-specific test receipts
as historical evidence; it does not create a new provider or product result.

## PLATFORM-SLICE-02 mechanical parity-test imports

Root authorized Network Rust test consumers. Two existing imports changed from
humanymous_test_support::assert_json_equivalent to
humanymous_platform::testing::assert_json_equivalent. No windows-object-bridge
reference matched. No old matching references remain in the35Rust file scope.
Only tests/report_parity.rs and tests/tls_parity.rs changed, each one import;
fixtures, dependency categories/features, authority and behavior are unchanged.
No manifest/path move, shim or alias; no compiler/unit/Clippy/fmt/CI execution
while Root completes the common graph. Diff inspection/check passed.

| Source | SHA256 before | SHA256 after |
|---|---|---|
| crates/network/tests/tls_parity.rs | 32572b1c0a023eda9820aea849211598806f294ba1c3df07dce29cc8ef108153 | 5c83ac51ceee549b2663c869e5ce555c364e5dda807d7f7f307d5c043a76de05 |
| crates/network/tests/report_parity.rs | 9d28088b687b038d18a226951889ecb8d83518b56f3934eb02f7ee6db92008d5 | bf044891a96af1c2df94fb91492ed41bfb756666d842de5ed19944aedcb9d4c1 |

## Final fixture/target readiness before Runtime freeze

Evidence:root/20261107215446 finalizes the Network/Capture part of Root's accepted
six-context map under fixture policy662cf7a2. It binds52current files plus4owned
oracle origins/Git blobs/byte hashes/license, proposes unique network_* parity
and capture_hosted_transport targets, and identifies Delivery command/feature
mapping and extracted-package compilation gates. No extra replica is needed by
these contexts. Existing five Network visibility restrictions and private Capture
fields stand; no new exception. Own source is clean, no handles/target leases or
remaining pre-freeze source edit. Root's selected Runtime test and403diagnostic
settlement precede its freeze. No source move/common Cargo/compiler action here.

## Source custody released for Root Runtime6-to-1 freeze

At2026-11-07T21:59:13Z this owner rechecked all52handoff1ccf3b1d file hashes:
zero differences and zero dirty Network/Capture source paths. The verified own
source claim was released and reclaimed for this owner's Spec paths ONLY; no
crates/** path remains claimed. The board release records the handoff. No process
handle, target lease or Git mutex is retained. Root received the completed
transfer and accepted network_*_parity/capture_hosted_transport target names.
No further source edit or compiler run is authorized here until Root's actual
move completes and a new exact path grant is issued. Root still waits for the
other owner handoffs/Native model checkpoint; this release does not assert that
the Runtime package move or any new-source qualification has completed.
