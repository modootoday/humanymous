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
