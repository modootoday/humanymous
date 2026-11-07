---
layout: doc
title: Candidate retry and origin-effect lifecycle
description: Proposed durable operation states and fail-closed handling of ambiguous origin outcomes.
permalink: /standards/draft-v0/protocol/retry-effects/
draft_status: project-draft
draft_version: v0
reviewed: 2026-11-07
---
# Candidate retry and origin-effect lifecycle

**Diátaxis quadrant:** Reference. **Audience:** decision-service, adapter and origin implementers.

**Status:** proposed semantics for [idempotent delivery](profile.md#p-05-idempotent-delivery),
not implemented or frozen wire. No current implementation is qualified against
this lifecycle. Uppercase words follow the [profile's requirement language](profile.md#requirement-language).

The proposed guarantee is at most one authorized application-layer dispatch
attempt for a logical operation within its qualified enforcement domain.
It is not general exactly-once business execution. Origin/intermediary behavior,
direct bypass and distinct business operations need separate controls.

## Identity and binding

A logical operation has an immutable reference bound to independently
authenticated producer, installation, tenant and permitted origin/action scope.
Transport attempts and status queries have separate identity and freshness.
They MUST NOT replace the logical reference. Before reservation or result
disclosure, current caller authority and the immutable request commitment MUST
be checked. The commitment covers method, target, final route/origin, application
principal, selected headers/body and required evidence. Connection/stream and
freshness remain bound per attempt. Encoding, algorithm and bounds remain open.

Credential, policy-revision, process or failover changes MUST NOT reset dispatch
eligibility. Revision/epoch bindings are recovery conditions, not automatically
new deduplication namespaces. Incompatible reuse MUST reject before prior-result
disclosure or dispatch. A client MUST NOT silently mint a new reference, extend
the original admission lifetime or change origin to recover an uncertain effect.
Distinct business operations require separate intent and authorization; this
protocol cannot infer that distinct references represent the same real-world action.
Refusing an incompatible attempt MUST NOT overwrite or reject the stored
original operation. Attempt-level refusal is distinct from the terminal
operation state named Rejected below.

## Durable states

One qualified enforcing authority, including all cooperating workers, owns the
authoritative operation record and serialized dispatch claim. An in-memory mutex
or an ordinary audit append alone does not establish this cross-process property.
The following names describe semantics, not selected JSON values.

| State | Meaning and permitted progression |
| --- | --- |
| Reserved | Validated operation, once-only logical budget reservation and operation audit commitment are durable; no dispatch claim. Current authorization, freshness and exact binding are required before its first claim; otherwise reject |
| Dispatch claimed | One durable atomic claim exists before any origin bytes or handler invocation. Only its original live execution incarnation may make the single attempt. Authenticated recovery can record indeterminate status, but cannot acquire execution permission |
| Completed | A precisely authenticated observed outcome under the chosen origin contract is durable. HTTP success alone is not business-commit proof. Duplicates can obtain authorized historical status, never another dispatch capability |
| Indeterminate | Dispatch or persistence may have occurred but a trustworthy final result is unavailable. Preserve the claim; read-only status/reconciliation only, no automatic redispatch |
| Rejected | Closed without a dispatch claim. Repeating this operation cannot reopen execution |

Budget, logical audit commitment and state transitions MUST be atomic, or joined
by a specified recovery protocol that refuses processing until consistency is
established. Retries MUST NOT reserve the logical budget again, duplicate its
operation commitment or refund an uncertain reservation. Every attempt still
needs bounded admission/rate controls and attempt telemetry. Deduplication MUST
NOT hide attempt counts or make resource consumption unbounded and free.

A dispatch claim MUST be durably committed before the first origin attempt.
A persistence error, timeout or lost acknowledgement MUST NOT imply the write
did not commit. Recovery consults authenticated authoritative state; if claim
existence is uncertain, dispatch remains ineligible.

Before the first origin attempt, the claimant MUST enforce the original admission
deadline and applicable authorization/capability conditions at a specified
dispatch boundary, without renewing the operation's lifetime. A paused claimant
resuming after expiry or applicable revocation MUST send no origin bytes and
MUST NOT release its consumed claim. Exact ordering/fencing of concurrent
revocation versus dispatch is still an acceptance blocker; a local precheck
alone does not prove instantaneous revocation across processes or networks.
Revocation after dispatch starts cannot undo an already possible effect.

After a claim, owner death, lease expiry, failover, restart, disconnect, partial
forwarding or response loss MUST NOT authorize another dispatcher. A paused old
owner may still be alive; lease expiry is not effective fencing. Outbound
libraries MUST NOT silently redirect or retry the origin attempt. A crash after
claim but before the first byte can therefore leave an indeterminate operation
with no effect. This sacrifices automatic recovery to bound dispatch attempts.
A restarted or recreated worker, even with the same stable service identity,
MUST NOT resume an existing claim as a dispatch token. Recovery restores status,
not permission for a new execution incarnation to send the request.

## Historical status and safe retirement

Status retrieval MUST authenticate and authorize the exact caller/tenant/operation
before disclosure. It returns a typed historical observation without executable
ALLOW authority. A cached successful decision MUST NOT invoke the origin again.
Store minimal outcome metadata; raw response caching is not a default requirement.
Only authenticated reconciliation bound to the original operation may resolve an
indeterminate observation, without releasing its dispatch claim. A client or
collector ACK cannot establish that resolution.

Live or uncertain records MUST NOT be evicted for new admission. Capacity
exhaustion refuses admission. Retirement needs a mechanism making old operation
references permanently inadmissible even with fresh transport authentication.
Credential expiry or a fresh wall clock alone is insufficient. Restored storage,
keys or backups MUST NOT reopen a consumed claim. Without trustworthy anti-rollback
continuity, refuse readiness. Concrete lifetime/revocation, retention and
reconciliation mechanisms remain acceptance blockers, not per-implementation defaults.

References, commitments and status metadata can be linkable or personal data.
Define purpose, access, retention, backups and revocation-before-erasure handling.
Deleting live replay state while the old operation remains admissible weakens
the guarantee; retaining a hash is not anonymity.

Recovery needs distinct pending, rejected, completed, expired and indeterminate
messages and a safe read-only status path. A retry control MUST NOT secretly
create another business operation. Recovery cannot depend on speed or motor
richness; actual keyboard and assistive-technology tasks remain required evidence.

## Observable conformance cases

These are proposed obligations, not executed generic-wire vectors. Count logical
reservations, claims, outbound attempts, origin effects, attempt telemetry and
errors separately.

| Case | Required observation |
| --- | --- |
| Concurrent identical operations across workers | One logical reservation and at most one claim/dispatch attempt |
| Same reference, altered body/route/headers/evidence | Reject before prior-result disclosure or dispatch |
| Incompatible duplicate after a valid reservation/completion, then correct status query | Original state and commitment remain unchanged; refusal cannot poison another operation |
| Crash/lost acknowledgement around reservation or claim persistence | Recover authoritative state; do not infer absence from a failed return |
| Crash after claim before first origin byte | Indeterminate; no automatic redispatch after restart |
| Origin commits but response/result persistence is lost | Indeterminate; no second origin invocation |
| Completed duplicate | Authorized historical status only; no fresh dispatch permission |
| Forged/stale consumption ACK | Reject it; no counter advancement or completion inferred |
| Claimed owner pauses, then deadline expires or applicable authority is revoked | No origin bytes on resume under the qualified dispatch boundary; claim remains consumed |
| Lease expires while old owner remains paused/alive | No takeover permission or second dispatch |
| Capacity exhaustion or retry storm | No live eviction; bounded attempts and complete denominators |
| Storage/key rollback, epoch/revision change or restored backup | No reopened eligibility without authenticated continuity |
| Cross-tenant query, erasure or long-pause recovery | No result leak, unsafe replay reset or speed-dependent recovery |

## Current implementation distinction

The [experimental adapter](implementation-experiment.md) rejects duplicate
nonces; it does not implement this durable operation ledger or historical-outcome
lifecycle. Runtime session counters and collector consumption ACKs track token
progress. An ACK, including one for an error response, does not establish a
completed origin effect or permit retrying an uncertain operation. See the
[implementation map](../validation/implementation-map.md). Independent implementations,
concrete wire/schema, crash-cut vectors and deployment/recovery evidence remain pending.
