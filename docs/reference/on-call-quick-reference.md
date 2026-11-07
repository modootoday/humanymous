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

## Observer-side node liveness

The audit library's `NodeRegistry` is an in-memory expected-node set, not a
signed heartbeat transport or independent monitoring deployment. Register expected
nodes through trusted configuration; unknown receipts cannot register themselves,
and duplicate registration cannot reset a deadline. Authenticate each receipt
and reject replay before calling `record_receipt`, using the observer's clock.

A node becomes missing strictly after twice its configured cadence, not at the
exact two-cadence boundary. Inputs use nanoseconds while `lastSeenMs` displays
milliseconds. A timestamp older than that node's accepted receipt or unsafe age
arithmetic returns an error instead of a reassuring liveness result. The caller
still owns persistence, trusted time and an observer outside the watched process;
a registry inside a suppressed process cannot independently report its absence.
This status does not prove request admission, audit completeness or human activity.
