---
layout: doc
title: Security evidence and audit status
permalink: /reference/security-audit.html
reviewed: 2026-11-07
---
# Security evidence and audit status

**Diátaxis quadrant:** Reference. **Audience:** security reviewers and adopters.

No completed independent audit or production certification of the Rust replacement
is claimed. Unit checks, dependency advisory scans and deployment pilots answer
different questions and must be reported separately.

A reviewable report names the exact source and artifact, covered interfaces,
threat assumptions, tools/database dates, findings and unresolved conditions.
Unexecuted checks remain pending. A clean advisory scan only means that its
database found no matching known advisory in the scanned dependency set.

Required boundaries include missing evidence, origin bypass, local management,
replay, audit storage failure and destructive-action approval. Report sensitive
findings through [security disclosure](security-disclosure.md).
