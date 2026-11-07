---
layout: doc
title: Administrative roles and dual control
permalink: /reference/rbac-separation-of-duties.html
reviewed: 2026-11-07
---
# Administrative roles and dual control

**Diátaxis quadrant:** Reference. **Audience:** service owners and security reviewers.

Administration must use a local, authenticated channel distinct from public
traffic and adapter decision transport. Locality alone does not authorize an
operation. Forwarded client headers cannot establish administrative trust.

Give service identities only the permissions needed for their role. Keep
read-only investigation separate from policy changes and destructive actions.
Erasure, kill-switch changes and permanent bans require independent authorized
approval. Record the request, approvals and outcome without leaking credentials.

The Rust management implementation still needs real unauthorized-peer, replay,
restart and dual-control pilots. See [destructive-action preparation](../runbooks/kill-switch-and-bans.md).

## Typed approval consumption boundary

The audit library exposes a signed proposal/approval policy and durable control
authority separately from runtime execution. `ApprovalRequest` carries the exact
proposal, independent signatures and trusted observer time. `authorize_action`
consumes approval through that authority before returning an `ActionPermit`.
The permit cannot be cloned or deserialized through this API.

`ActionPermit::execute` consumes the permit and checks the exact installation/node
scope, action, target commitment and validity window before invoking its callback.
The callback still owns durable runtime state and outcome audit. A consumption
receipt proves neither that the action completed nor that a failed callback had
no effects. Do not present this primitive as an installed administrative router,
exactly-once business operation or complete crash-recovery service. Trusted time,
independent administrator provisioning, external latest anchors and actual runtime
peer/restart/dual-control pilots remain separate consumer responsibilities.

## Policy and durable ledger scope

The current policy provisions 2–128 distinct administrator identities with
separate nonweak Ed25519 keys. Aliases sharing a key cannot form quorum. Every
supplied approval is checked; an unknown, duplicate or invalid extra is not
ignored after two valid signatures. Distinct provisioned keys do not by
themselves establish two independent human operators or organizations.

Signatures bind the exact installation/node, action, target commitment, operation
ID and issued/expiry times. The configured lifetime is bounded to one day and
may be shorter. Trusted UTC seconds are required here; do not substitute request
claims or the millisecond time units of another protocol. Target bytes and any
object/generation schema remain the consuming operation's responsibility.

The signed consumed-operation ledger has an explicit finite capacity and does
not evict spent IDs to accept new destructive operations. Existing-state reopen
requires the exact independently retained latest anchor; a signed local file
alone does not establish freshness against directory rollback. Preserve the new
anchor outside that local state before acknowledging the operational effect.
Storage failure can leave an ambiguous committed or staged transition; it does
not authorize resetting the ledger or replaying an operation blindly.

Clock regression poisons the current issuer. Successful consumption records its
time durably, while rejected observations do not independently advance a durable
trusted clock. Provisioning across restart and latest-anchor delivery still need
operational evidence. Explicit erasure recovery is limited to the same consumed
proposal and subject, with valid approval signatures and time window checked
again; it cannot approve a new subject or bypass expiry. A blocked recovery is
an incident to resolve under the owning policy, not permission to discard spent
state, widen scope or fabricate approval.
