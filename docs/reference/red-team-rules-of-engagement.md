---
layout: doc
title: Defensive evaluation rules
permalink: /reference/red-team-rules-of-engagement.html
reviewed: 2026-11-07
---
# Defensive evaluation rules

**Diátaxis quadrant:** Reference. **Audience:** authorized lab evaluators.

The attack catalog targets only this project's owned, isolated deployment.
Third-party probing is outside scope. Keep attack images and private traces out
of normal runtime packages and public deployment images.

Use synthetic accounts and bounded workloads. Record the intended target,
network isolation, stopping conditions and cleanup before a run. Include human
controls and record failed or skipped attempts without removing them from the
case ledger.

Full enforcement evaluation runs in Docker so the lab topology and isolation are
part of the result. A host runner does not qualify the replacement. See
[catalog evidence](red-team-catalog.md).
