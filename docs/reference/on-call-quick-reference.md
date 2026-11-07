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

## Experimental native status service

The runtime now has source for a separate local observer listener. It accepts
only `POST /management/v0.1/status`, authenticated with the configured native
peer and a separately provisioned management credential. Application sessions,
adapter credentials and browser headers do not grant management access. Keep
the credential out of public logs and application configuration exports.

The request uses `Host: humanymous.management.local` and exactly
`Content-Type: application/json`. Its JSON body, bounded to 8192 bytes, must
match the configured installation, tenant, origin, revision and authority.
Unknown or duplicate scope fields and duplicate header names reject. Browser
context headers, including Cookie, Origin, Referer and `Sec-` headers, reject.
Query variants and mutation routes are not accepted status requests.

The response identifies the observer role and runtime profile, with
`settings_writable` and `destructive_controls` both false. It is not a heartbeat
receipt, a management write API or evidence that a deployed service is healthy.
Source joins application and management listener shutdown before releasing
service custody; actual native IPC, permissions, cancellation and lock-release
pilots remain required. The new listener and policy fixtures have not yet been
executed in the current qualification run.
