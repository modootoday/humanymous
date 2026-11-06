---
layout: doc
title: Prepare destructive-action approval
permalink: /runbooks/kill-switch-and-bans.html
reviewed: 2026-11-07
---
# Prepare destructive-action approval

**Diátaxis quadrant:** Runbook. **Audience:** authorized administrators.

Kill-switch changes and permanent bans require independent authorized approval.
The Rust management path still needs real local-peer and dual-control pilots;
no operational command is qualified by this page.

Before a pilot, define the requested action, scope, expiry, approver separation
and rollback boundary. Verify authentication, local transport and authorization
for both participants. Reject replay, mismatched scope and expired requests.
Record the final outcome without storing approval credentials.

Exercise refusal paths as well as successful approval. A public proxy or adapter
credential must never reach these operations. See [roles](../reference/rbac-separation-of-duties.md).
