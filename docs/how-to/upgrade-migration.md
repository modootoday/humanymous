---
layout: doc
title: Plan migration acceptance
permalink: /how-to/upgrade-migration.html
reviewed: 2026-11-07
---
# Plan migration acceptance

**Diátaxis quadrant:** How-to. **Audience:** operators of existing evaluations.

Do not replace a working deployment with this incomplete Rust branch. An upgrade
procedure requires a qualified candidate, configuration compatibility, key and
audit continuity, rollback and exact supported platform evidence.

Rehearse with synthetic copies in an isolated lab. Compare observable behavior,
restart/replay and failure outcomes. Confirm origin isolation and management
permissions after migration. A new language or successful build alone does not
establish compatibility. See [status](../reference/production-vs-reference.md).
