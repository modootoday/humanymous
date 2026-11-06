---
layout: doc
title: Local administration and configuration revisions
permalink: /explanation/control-plane-and-bundle.html
reviewed: 2026-11-07
---
# Local administration and configuration revisions

**Diátaxis quadrant:** Explanation. **Audience:** operators and security reviewers.

Administrative authority is separate from request processing. A decision channel
credential must not grant configuration or erasure access. Public listeners
never alias local management routes.

Configuration revisions let a decision name the policy that authorized it.
Loading, validating and activating a revision need explicit error and rollback
behavior. A bad revision cannot silently weaken required capabilities. Local
binding still needs credentials, authorization and protection against hostile
local processes or browser requests.

The Rust control plane awaits operational qualification, including restart and
dual-control tests. See [administrative roles](../reference/rbac-separation-of-duties.md).
