---
layout: doc
title: Operator guide
permalink: /start-here/operator.html
reviewed: 2026-11-07
---
# Operator guide

**Diátaxis quadrant:** How-to. **Audience:** service operators.

The Rust runtime has not completed deployment qualification. Do not expose a
migration build to public traffic based on these preparation notes.

Prepare a dedicated lab with an origin that rejects direct bypass, a local
management channel, scoped service identities and bounded storage. Document
certificate ownership, audit retention, recovery contacts and rollback authority.
Local management still requires authentication; forwarded headers do not prove
that an administrative caller is local.

Before deployment, require startup, restart, audit replay, storage-outage,
unauthorized-management and origin-bypass pilots against the actual artifact.
Record the exact configuration revision and tested topology. An incomplete or
skipped required case blocks qualification.

Use the [incident preparation guide](../runbooks/incident-runbooks.md) and
[requirements](../reference/install-requirements.md) to prepare the evidence.
