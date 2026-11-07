---
id: plan:root/20261107144500
title: Draft Runtime Pass ownership and integration gap map
status: draft
created: 2026-11-07T14:45:00Z
updated: 2026-11-07T14:51:24Z
rel:
  depends_on: [sot:root/runtime-application, sot:root/pass-protocol, sot:root/rust-migration, plan:root/20261107143100]
---

# Runtime Pass integration proposal

Design/review only. Root requested the actual legacy handler and current Rust
gap map before code. Pure challenge ae57c13e and its37 component tests do not
qualify issuance, replay, event provenance, attestation, accessible UI or product.
No route, scoring/default/predicate, key, dependency or runtime process changes
are authorized by this draft. The full R01-R17 objective remains intact.

## Source reconstruction and ownership

| Responsibility | Actual legacy source | Current Rust and missing owner work |
|---|---|---|
| Page and HTTP orchestration | cmd/server/server.go registers /pass and /api/pass/{new,pow,attest,solve,kpi}; pass_handler.go orchestrates them | Runtime has no Pass route or recovery; Decision::Local documents CHALLENGE without recovery. Runtime owns new explicit handlers; accessible browser scene is a separate frontend owner/join |
| Session/instance issuance | pass_handler.go handlePassNew; pass_store.go passSession, passNonce/get | Runtime Sessions authenticates volatile profiles/RIT, but has no Pass instance/nonce/tries/crypto state. Legacy cookie ensure/get is not an authenticated Rust session |
| Alignment | internal/pass/pass.go plus its two test files | challenge::pass generate/fresh/verify/solution_offsets now exists; Runtime must reuse it, not copy an algorithm |
| Crypto preflight | handlePassPoW binds sid+"\u007cpass\u007c"+nonce; handlePassAttest binds sid/nonce and issuance budget | challenge::proof_of_work and attestation are pure primitives. Runtime owns issuance/key/nonce/budgets, actual redemption and single-use; no current handler |
| Event submission/replay | pass_proof.go passProof, realEventOK, traceDigest; pass_store.go reserveTrace | No Runtime event-proof/replay owner exists. Client trusted/motor arrays are assertions, not native provenance. Collector/session RIT proves request continuity, not humanity |
| Risk/fusion | pass_scoring.go applyPassVelocity/applyPassBehavior/passDifficulty/passSolvedSignal; handler re-scores | Shared Rust Engine already supports server l7.pass.solved on unpromoted CHALLENGE only; Runtime rejects client-forged server-plane signals and has no trusted Pass producer. No engine fork or new scoring rule |
| Replay and resource limits | passMaxTries4, maxPassSessions50000, traceTTL10min, maxTraces2^19 | Explicit bounded Runtime state and exhaustion behavior require acceptance. No copying an unstored ephemeral fallback or unbounded churn map |
| Outcomes/step-up/KPI | pass_handler.go publishPass/feedPassOutcome and optional step-up receipt; KPI risk/self-labels | No equivalent Runtime producer/receipt ownership exists. Solved puzzle is not a confirmed-human label or permission to issue external Gate step-up receipts |

Legacy sources are read-only oracles. Original SoT-36 was not found by Root;
comments referencing it do not override current hard constraints.

## P0 compatibility decisions before implementation

1. RESOLVED POLICY DISPOSITION: legacy realEventOK hard-refuses keys<3, moves<5/path<20, uniform key/pointer
   timing and insufficient raw/coalesced samples. Blind migration conflicts with
   the current no-motor-richness/no-speed-gate rule. pass_scoring.go also injects
   timing/mobile predicates; no such producer/predicate/default changes follow
   from migrating the pure puzzle. Root explicitly directed NOT to preserve
   these legacy admission conditions because the controlling current a11y
   constraint overrides them. Record this incompatibility, not silent retirement.
   Authentication/tamper/session/instance/expiry/single-use/audit checks remain.
   No fabricated benign motor trace or newly activated scoring producer follows.
2. Legacy /new advertises expiresInS=30*2, but solve calls pure Fresh with bucket
   age<=20 (up to roughly629seconds), ignoring issuedAt for that expiry decision.
   The new instance wall/monotonic TTL must be explicitly accepted; neither60s
   nor20buckets becomes an implicit default. Expiry is an expired challenge,
   not a wrong answer, bot signal, punishment or motor-speed requirement.
3. RESOLVED CLAIM DISPOSITION, PRODUCER REVIEW STILL OPEN: public alignment, PoW payment, JSON trusted=true and a self-issued rate-limited
   HMAC token do not prove a human. The legacy feedPassOutcome confirmed-human
   statement and weak risk/self-reported KPI labels cannot become ground truth.
   Root classified the result as server-verified interactive challenge completion,
   NOT confirmed human; model oracle labels remain ReviewRequired/Quarantined.
   Authority must independently review accepted producer trust, expiry and replay
   before wire or existing server l7.pass.solved input. Required independent planes
   remain unavailable rather than satisfied by a completion assertion.
4. Global quantized motor-trace replay gating cannot require richer AT input:
   empty/regular traces can coincide across legitimate switch/keyboard users.
   Legacy quantization also admits perturbation across bins and is not universal
   replay prevention. Preserve accepted anti-replay semantics or approve a stated
   compatibility delta; do not quietly replace them with instance-only replay
   and claim equivalent cross-instance coverage. Authority review must separate
   one-use signed instance redemption from the weaker historical cross-instance
   motor-trace heuristic; neither requires richer input or proves humanity.
5. Legacy JA4|subnet issuance identity is not currently an authenticated Runtime
   context: loopback/normalized forward headers and a client report cannot mint
   that identity. Capture/shared-provider ownership and fail-closed missing
   context are required for cross-session attestation budgets. Session resource
   quota alone is not proof of equivalent fingerprint budgeting.

## Proposed authenticated lifetime and bounded state

All production Pass mutations use explicit POST and the existing authenticated
Runtime profile, canonical request binding, one-flight/counter semantics and
actual-status browser ACK. No cookie-created session, ambient identity or
client-supplied session/instance is accepted. Proposed browser internal route
names and explicit opt-in PassPolicy still need Root acceptance; existing route
defaults and NativeFull ACK behavior are unchanged. No enabled-by-default Pass.

Application is the sole serialized Pass owner; bounded per-session state is
subordinate to the real Sessions epoch and scope/policy. Restart, expiry,
resource-setting session invalidation, observer loss or poisoned audit invalidates
Pass state too; no retained solved state or silent session resurrection. Missing
session must refuse rather than allocate an ephemeral unstored challenge.

An instance binds scope, actual session epoch, checked monotonic instance number,
fresh OS-CSPRNG nonce, issue bucket/time, explicit expiry, tries and crypto/event
status. Preserving legacy18-byte lowercase-hex nonce is a proposal, not a new
security theorem. Counter overflow, entropy failure, invalid/backward clock and
capacity exhaustion refuse; do not wrap/reseed or evict a live instance to fit.
An authenticated new-instance action invalidates the previous instance atomically;
it cannot reset issuance quotas or persistent controls.

Pure challenge/PoW/attestation need a distinct server-only zeroizing key. The RIT
key is also known to the client and MUST NOT be reused to authenticate server
attestation. Do not extract/reuse audit custody keys. A fresh explicit volatile
Application key is a candidate consistent with restart-invalidated sessions;
provisioning/domain separation/lifetime must be accepted before code. No new
key-store backend or persistent key creation is inferred.

Keep the existing Runtime64KiB outer body limit and propose narrower bounded
request schemas (legacy4KiB pow/attest and32KiB solve are review inputs). Reject
unknown/duplicate fields, nonfinite values, excessive arrays/depth and trailing
JSON before allocation/amplification. No truncating event streams to fit. Bound
pending instances by existing session capacity, retries by accepted policy and
replay/issuance maps by finite trusted limits; exact sizes and GC budget remain
pending. Saturation is operational unavailable, not a bot label or permanent ban.

## Audit, consumption and effects

Root accepted the design direction of a server-issued HMAC instance bound to
the actual authenticated session/epoch and RIT-authenticated response, one-time
consumption and audit before a server completion observation. Proposed signed
instance context includes explicit domain/version, scope/session-epoch binding,
fresh nonce/checked instance, issue/expiry/difficulty and eligible request context.
It is PUBLIC authorization metadata, not a secret answer or human attestation.
The eligible context itself stays PRIVATE: no app Authorization, body, raw
serialization or unkeyed low-entropy request hash may enter the public token.
Use a fresh opaque context ID backed by private state, or a separately reviewed
keyed purpose commitment. A plain SHA256 of a password/body is not privacy.
The concrete shared HMAC encoder/key/domain API must be jointly accepted; do not
reuse the client-known RIT key, ambiguous delimiter joins, wall-clock-only proof
or the pure attestation helper as an undeclared new wire protocol. HMAC verification
does not replace retained current-instance/single-use state or actual RIT checks.

An authenticated completion may contain no motor stream at all. Event fields,
if retained for accepted non-gating analysis, remain bounded client assertions;
trusted=true, event count/richness and speed cannot promote their provenance.
PoW/identity-budget tokens remain their stated cost/issuance evidence, not human
proof. Completion observation and a trusted Engine input are distinct typed
producer decisions; no existing Engine predicate, weight or band is changed and
no unreviewed producer is newly activated. Accessible automation/T4 is an explicit
ceiling, not a reason to restore the rejected legacy gates.

Outer parse/resource413 remains pre-consumption with no ACK. Authenticated
handlers must define which validation/resource outcomes consume RIT; after
consumption return a valid browser ACK for the ACTUAL status even when refused.
Do not infer consumption from status or add BrowserAuthOnly ACK to NativeFull.
Never reset counters or automatically replay an ambiguous solve.

Candidate state transitions are Issued -> CryptoBound -> SubmissionValidated ->
Consumed, with explicit Expired/Invalidated/PendingUnavailable outcomes. This is
not an accepted wire schema. A solve must match the CURRENT issued nonce/session/
instance/expiry. An old verify result cannot commit after a concurrent new issue,
resource update, loss or cancellation. Application serialization plus the eventual
same epoch/effect gate must establish the commit boundary.

Validate all required crypto/event/fusion inputs and exact alignment before
reserving scarce cross-instance replay state; bad/expired inputs cannot poison
that registry. Reserve replay and instance consumption together, retaining a
pending/poisoned state across uncertain audit failure. No rollback to reusable
intent after an append may have happened. Durably audited issuance/crypto/solve/
consumption precedes publishing success or enabling a trust input; required
witness failure never becomes writer-only success. Cancellation after possible
mutation preserves ambiguity; fresh recovery does not replay effects.

Audit bounded event types runtime.pass.* separately from permanent kill/control.
Commit scoped request/instance/outcome identifiers without raw event streams,
motor timestamps, IP/fingerprint, RIT/session key, PoW solution or attestation
token. Purpose, retention and pseudonym threat limits need explicit review.
No public anonymous KPI or caller risk/red-team label becomes evaluation truth.

## Recovery and frozen Engine boundary

Pass is eligible only for an actual server CHALLENGE context; it must not clear
DENY or a fired hard-rule/promotion. Existing Engine semantics remain unchanged.
Store a private eligible request/context commitment rather than accepting a
client verdict. Any eventual server l7.pass.solved is created by the accepted
fusion producer and never accepted from a client report or a solved offset alone.

Proposed recovery is scoped/short-lived/single-use for the original protected
application request identity (method/target/body/content-type/app Authorization
and scope, excluding changing RIT counter/MAC fields). The caller still sends a
fresh authenticated request and Engine re-scores; solving never directly contacts
origin or automatically retries a mutation. This is an explicit pending delta
from legacy session-wide trust and repeated alreadySolved step-up renewal, not
claimed compatibility. No external step-up ticket is issued until its separate
Gate owner accepts binding, lifetime, redemption and effect semantics.

The public scene needs keyboard/switch/screen-reader support and a zero-movement
solution when keys already align. No minimum duration, timing variance, path
length, pressure, movement count or sensor richness may be a Pass gate. Any
accepted computational cost/identity budget is not human proof. Actual ordinary
browser/AT cases, honest expiry/new-puzzle recovery and incomplete/unsupported
paths must be separately measured; synthetic event dispatch is not that evidence.

## Slice sequence and evidence gates

S1: Root reviews this gap map and resolves P0 compatibility, TTL, provenance,
key, resource and recovery-scope choices. Record accepted changes in SoT before
reachable code. Independent frontend/capture/audit review is required; the five
persona check below is single-author analysis, not their approval.

S2: Runtime pure bounded instance/issuance/nonce/expiry/consumption state with
no signal or public-success constructor until the fusion input is accepted.
Reuse challenge APIs. Tests cover wrong session/scope/epoch/instance, issue/solve
races, cap/GC/clock/overflow, repeated/expired solve, empty/regular AT data,
replay saturation and all-failure no-upgrade behavior. Independent Go byte vectors
remain a Root challenge-module gate, not a new Runtime algorithm oracle.

S3: Authenticated HTTP/RIT/ACK plus actual audit/witness-before-effect composition;
test invalid MAC/preconsume413/missing required plane, postconsume refusals,
append/flush/witness/cancel ambiguity, restart/new epoch and no origin mutation
from solve. Native component passes do not qualify ordinary-browser event proof.

S4: Actual Rust-authored accessible browser scene/collector session/one-flight/
abort integration and admitted evidence fusion. Genuine human keyboard/switch/
screen-reader cases, zero-motion alignment and expired refresh must be observed,
not generated as executable foreign strings or counted as synthetic Fetch passes.

S5: Docker-only full topology, actual required producers/witness, independent
origin bytes/effects, replay/parallel/new-instance/fault cases, source/image
bindings and exact denominators. Positive TLS ALLOW and typed native startup
remain independent required gates, not supplied by this Pass migration.

## Single-author critique reflected

Blue: one Application/Sessions/audit owner and shared Engine; no parallel trust
engine. Red: public solution/client trusted flag/self-issued token and perturbed
trace replay cannot prove humanity; bind actual instance/counter and preserve
ambiguity. SRE: finite capacities, conserved clocks and no ephemeral fallback or
quota reset. DPO: raw motor/fingerprint retention is not anonymous; audit bounded
commitments only. Evaluator:37 pure challenge tests and proposed source checks do
not qualify handlers, browser/AT or product. Adversarial verdict NEEDS-WORK:
P0 policy/provenance/TTL/identity/replay/recovery choices above block reachable
implementation. No blocker is waived as a production residual by this draft.

## Independent Authority review v0.2

Authority reviewed exact v0.1 SHA256
6dc16e80be7358333097a70d0f5ca6972de1f441b7b495989c6a4f4b9f42db65 and returned
NEEDS-WORK before wire. The following requirements incorporate all six findings;
this revision is not an accepted protocol or implementation approval.

### One absolute lifetime, pending explicit policy acceptance

PassPolicy must supply ONE explicit finite instance TTL; no implicit60-second or
20-bucket default is selected here. Retain issue wall time, checked wall expiry
and local monotonic deadline. Refuse when wall_now>=expiry OR mono_now>=deadline;
the pure challenge Fresh check is an ADDITIONAL AND condition, never an alternative
lifetime that extends acceptance. Preserve backward-clock failure separately.
The effective bound is also no later than session and required observer lifetime.
Root must freeze the numerical TTL/caps and public expiry fields before wire.

PoW, attestation, failed/malformed solve, status polling, a new RIT counter or
bucket movement cannot renew the instance or its recovery artifact. Server-key
epoch or session-epoch change invalidates ALL related instance/crypto/completion/
recovery artifacts; no cached token survives restart. Explicit new-instance
issuance uses a fresh ID/nonce and the accepted quota, not a renewal of the old
instance. Expiry is not a wrong answer and never creates a speed-derived signal.

### Proposed exact consumption/outcome table for review

This table proposes NEW explicit Runtime route behavior, not legacy wire parity.
All four actions are authenticated POST. Browser ACK is present only after valid
RIT consumption and binds actual status; NativeFull gains no browser ACK.

| Condition/action | Proposed HTTP | RIT consumed / browser ACK | Instance/tries/quota disposition |
|---|---|---|---|
| Outer or narrower accepted body/predecode resource limit |413|No / none|No victim state, tries or nonce mutation |
| Unsupported required producer/profile or missing trusted budget identity |422|No / none|No substitute fingerprint/session quota; unavailable before issue |
| Invalid MAC/session/scope/epoch or conflicting request counter |Existing403/409|No / none|Cannot burn another session's nonce, tries or budget |
| Valid RIT, then malformed/unknown/duplicate semantic JSON |400|Yes / valid400 ACK|No new instance/crypto/recovery; no wrong-answer try; debit only accepted endpoint resource quota |
| Valid RIT, endpoint issuance/attempt resource cap |429|Yes / valid429 ACK|No instance created/replaced; fresh nonce cannot reset quota |
| new with no eligible server CHALLENGE, or stale/wrong/already-consumed instance |409|Yes / valid409 ACK|No trust or receipt refresh; no nonce/try mutation on an unrelated instance |
| new accepted |200|Yes / valid200 ACK|Stage fresh instance, audit, then commit publication; replace previous instance only at accepted transition |
| pow/attest accepted |200|Yes / valid200 ACK|Audit before marking exact instance crypto evidence; original expiry unchanged |
| pow/attest invalid proof |422|Yes / valid422 ACK|No crypto promotion or TTL reset; no solve wrong-answer try; accepted endpoint quota still applies |
| solve wrong alignment for exact live instance |422|Yes / valid422 ACK|One accepted wrong-answer try, audited; no replay reservation or trust; cap policy pending |
| Any post-consumption instance expiry |410|Yes / valid410 ACK|Expired, not wrong answer; no renewal, event penalty or new artifact |
| solve valid under all accepted requirements |200|Yes / valid200 ACK|Atomic Pending reservation, audit/witness, then one-use completion/recovery commit |
| Post-consumption audit/witness/cancel/loss/unknown append |503 if no response started|Yes / best-effort valid503 ACK|Retain Pending/burned/quarantined state; missing ACK closes client, never reusable rollback |
| Success response/ACK lost after commit |No invented delivery status|Counter and instance remain consumed|No reissued ticket, automatic solve retry or regenerated quota; client invalidates ambiguity |

Exact410/422 schemas, solve cap, quota charging and semantic/predecode division
still need Root/Authority acceptance. This table does not add detection rules.
Authentication checks precede instance ownership changes; signed nonce/instance
verification precedes decrementing that instance's try count. Body/decoder limits
are resource failures, not secret answer or timing oracles.

### Atomic reservation and distinct replay properties

After validation but BEFORE any authoritative append, reserve instance redemption,
accepted replay identity and recovery slot together as Pending under the one
serialized owner/epoch. No second solve/new issue can steal, release or duplicate
that reservation. Append/witness/cancel uncertainty, loss or ACK delivery failure
retains consumed or quarantined ownership, never rollback to reusable state.
No valid-prefix local fsync substitutes for required independent witness coverage.

One-use HMAC instance redemption prevents exact instance/token replay. It is not
the legacy cross-instance quantized motor-trace heuristic. Distinct valid
instances with empty or regular switch/AT traces MUST NOT be hard-denied for
sharing that motor representation. Copying cryptographic artifacts to a different
nonce/session/instance is rejected by binding. Cross-instance behavioral replay
and perturbation residuals must remain separately described; Root must explicitly
dispose the legacy trace-gate behavior before any replacement is reachable.

Recovery reservation/consumption must be one transaction with the matching fresh
request RIT, current Engine decision, same live epoch/effect gate and required
audit. Method/target/body/content-type/app Authorization/scope binding stays
private/keyed. A current DENY or hard promotion cannot be upgraded. A partial/
submitted origin request remains a possible effect after loss; no automatic
origin replay, receipt reissue or renewed recovery deadline is permitted.

### Accessible scene requirements, owner still to be designated

Browser owner independently required semantic names/roles/states/values and
usable bounds; complete keyboard-only and switch/AT operation; predictable focus
entry, visible focus/traversal/restoration and status/error announcements. Already
aligned zero-motion completion must work. IME composition/confirmation must not
accidentally submit or navigate. No minimum event count, motor richness, speed,
pointer-only operation or timeout advantage may determine eligibility.

Record native input, accessibility action and programmatic-change provenance
separately; DOM trusted flags/synthetic receipts do not qualify human or AT use.
Root must designate the Rust Pass scene owner and host accessibility/focus adapter
owner. The browser lane can supply CEF/Meari integration observations, not assumed
ownership of a new web scene. Actual OS keyboard/screen-reader checks are distinct
from pure challenge and Docker HTTP topology qualification. No UI was implemented
or workload launched by this review.
