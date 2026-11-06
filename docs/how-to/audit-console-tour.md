---
layout: doc
title: Review an audit console candidate
permalink: /how-to/audit-console-tour.html
reviewed: 2026-11-07
---
# Review an audit console candidate

**Diátaxis quadrant:** How-to. **Audience:** audit and accessibility reviewers.

Rust console operation remains unqualified. Use synthetic audit records and
local authenticated access when evaluating a candidate. Verify that read-only
inspection cannot perform administrative mutations.

Check proof scope, missing-data warnings, redaction and keyboard navigation.
Keep credentials and subject data out of screenshots. Destructive controls must
still require independent authorization; hiding a button is not enforcement.
See [administrative roles](../reference/rbac-separation-of-duties.md).
