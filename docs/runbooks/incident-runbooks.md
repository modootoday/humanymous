---
layout: doc
title: Prepare an incident runbook
permalink: /runbooks/incident-runbooks.html
reviewed: 2026-11-07
---
# Prepare an incident runbook

**Diátaxis quadrant:** Runbook. **Audience:** service operators.

The Rust replacement has no qualified production runbook yet. Prepare and test
these steps in an owned lab before rollout.

1. Identify the artifact, configuration and topology; establish the affected interval.
2. Separate input, transport, audit-storage and origin failures from policy outcomes.
3. Preserve minimal redacted evidence and verify that management remains local.
4. Apply only a reviewed recovery or rollback plan through authorized administration.
5. Check origin isolation, durable audit recovery and accessible visitor support.
6. Record the outcome and unresolved conditions before reopening protected traffic.

Do not replace missing safety evidence with a blanket allow. Destructive changes
retain [dual control](kill-switch-and-bans.md).
