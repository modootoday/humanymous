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

## Source-only kill state assessment

Working runtime library source adds `KillTarget` and `KillStatus::assess`; these
are not an installed kill command or a qualified startup/admission gate. The
target binds the installation and node to the versioned one-way killed state.
Changing tenant, origin or policy revision does not rearm that target.

Assessment joins a freshly authenticated retained kill intent with the sealed
audit history and independent witness coverage. It requires exact scope,
authority identity and target binding, and refuses other consumed kill targets.
The source distinguishes these outcomes:

| Retained intent | Matching audit effect | Assessment |
| --- | --- | --- |
| Absent | Absent | `Clear`, only for this serialized inspection |
| Consumed | Independently covered | `Killed` |
| Consumed | Absent or beyond witness coverage | `Pending`; remain unavailable |
| Absent | Present, covered or ambiguous | Error; resolve the inconsistency |

Keep startup, admission and issuance serialized throughout the join. A cached
`Clear` value must not become a reusable healthy capability. Preserve a pending
or inconsistent state for controlled recovery; expiry of the original approval
does not clear consumed intent. The assessment does not apply an effect, issue
a grant or prove that the runtime currently enforces the result. Current native
restart, crash, witness-loss and concurrent-issuance pilots remain outstanding.
