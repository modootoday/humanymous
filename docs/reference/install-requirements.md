---
layout: doc
title: Installation qualification requirements
permalink: /reference/install-requirements.html
reviewed: 2026-11-07
---
# Installation qualification requirements

**Diátaxis quadrant:** Reference. **Audience:** prospective operators.

There is no qualified Rust installation procedure on this branch yet. Do not
infer downloadable binaries, service commands or supported OS versions from
the migration goals.

A releasable installer needs an exact OS/architecture/runtime matrix, verified
artifact provenance and a doctor report for topology and capability readiness.
Its dry run must leave files, services, privileges and networking unchanged.
Installation must report managed changes, preserve operator-owned files and
demonstrate safe repeat execution and rollback.

Management requires local binding, authentication and restrictive service
permissions. Public container mappings must never expose it. Review
[release status](production-vs-reference.md) before selecting artifacts.
