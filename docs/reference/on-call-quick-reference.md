---
layout: doc
title: Incident review quick reference
permalink: /reference/on-call-quick-reference.html
reviewed: 2026-11-07
---
# Incident review quick reference

**Diátaxis quadrant:** Runbook. **Audience:** incident responders.

No production Rust operating procedure is qualified yet. Prepare this information
for an owned pilot before relying on a service.

| Need | Record |
| --- | --- |
| Identify the deployment | Artifact digest, configuration revision and topology |
| Assess impact | Attempts, outcomes, errors, affected routes and time range |
| Check safety boundaries | Audit durability, origin isolation and management binding |
| Recover | Reviewed rollback plan, authorized operators and approval requirements |
| Help visitors | Accessible alternative and support reference |

Preserve minimal evidence without copying secrets or visitor bodies. Avoid a
blanket allow workaround when required safety conditions cannot be established.
Continue with [incident preparation](../runbooks/incident-runbooks.md).
