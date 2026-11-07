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
