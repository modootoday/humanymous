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

The runtime now has source for a separate native management listener. Its observer
role accepts `POST /management/v0.1/status` and `POST /management/v0.1/settings`,
authenticated with the configured native
peer and a separately provisioned management credential. Application sessions,
adapter credentials and browser headers do not grant management access. Keep
the credential out of public logs and application configuration exports.

The request uses `Host: humanymous.management.local` and exactly
`Content-Type: application/json`. Its JSON body, bounded to 8192 bytes, must
match the configured installation, tenant, origin, revision and authority.
Unknown or duplicate scope fields and duplicate header names reject. Browser
context headers, including Cookie, Origin, Referer and `Sec-` headers, reject.
Query variants and mutation routes are not accepted observer requests.

The status response identifies the observer role and runtime profile, with
`settings_writable` and `destructive_controls` both false. A settings read returns
the operational snapshot/revision; `operator_available` reports separately whether
an operator was provisioned. The observer credential remains read-only. These
responses are not heartbeat receipts or evidence of a healthy deployed service.
Source joins application and management listener shutdown before releasing
service custody. An ordinary-user Unix component run exercised native IPC,
observer/operator credential separation and graceful drain before custody release.
These Unix-only lifecycle cases were not compiled in the Windows component run.
Protected Windows service operation, broader cancellation/permission cases and
deployment-specific lock-release evidence remain separate qualification gates.

## Experimental resource operator

New library source exposes `ServiceHost::bind_with_operator`, requiring a
management policy, a different operator credential, durable settings and an
external witness. Missing prerequisites or reused observer/operator credentials
refuse startup. The current standalone runtime entrypoint does not provision
this role. A Unix component fixture has executed; this does not establish a
qualified deployment or operating procedure.

On the native management endpoint, the operator accepts only
`POST /management/v0.1/settings/commit`. Its strict JSON carries `scope`,
`expected_revision` and `settings`, under the same header, peer and body limits.
It cannot use the observer credential or change scoring, credentials, erasure,
kill switches or permanent bans. Only these resource fields are accepted:

| Field | Bound |
| --- | --- |
| `session_capacity` | 1–4096 |
| `session_ttl_ms` | 1000–3600000 ms |
| `timeout_ms` | 1–30000 ms |
| `rate_per_second` | 1–10000 |
| `session_rate_per_second` | 1–10000 |

The source queues one change at a time for the application owner between requests.
It checks the expected operational revision, stages exact snapshot bytes, records
the authenticated journal commitment and finalizes before applying resources.
Application invalidates existing volatile sessions; this is not rollback of
earlier business effects. Cancellation, deadlines and stop state are rechecked.
An ambiguous post-staging failure poisons the serving epoch and returns an
unknown-outcome error. A timeout or 503 does not prove the change was absent;
inspect the authenticated persisted revision before any recovery decision.
Automatic mutation retry is not provided.

At a frozen runtime source revision, the Unix component fixture observed a
witnessed compare-and-swap update, stale-revision refusal, old-session invalidation,
the updated capacity limit, graceful drain and trusted-head restart restoring the
persisted revision. Its actual signer and durable witness storage ran in the same
process; this is not independent witness deployment. Broader crash/cancellation/
permission, protected Windows service and independent witness/custody qualification
remain outstanding. These results do not qualify TLS, browser or full-product use.
