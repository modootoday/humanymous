---
layout: doc
title: Installation qualification requirements
permalink: /reference/install-requirements.html
reviewed: 2026-11-07
---
# Installation qualification requirements

**Diátaxis quadrant:** Reference. **Audience:** prospective operators.

The local provisioning CLI is implemented; a signed, activated and qualified
full-service installation is not. Build from the reviewed source with Rust
1.99.1 and its locked dependencies. No published binary availability is implied.

`humanymous doctor` validates an explicit regular artifact, absolute destination,
lowercase SHA-256 and the operator's local-management verification acknowledgement.
The acknowledgement records a precondition; it does not perform a runtime IPC
or management qualification. `install --dry-run` and `doctor` remain read-only.
Installation and repeat/rollback behavior are limited to the managed artifact
and receipt, with refusal to remove a mutated installation.

Existing private user-owned parents are required. Unix pilots require a non-root
user and owner-private directory permissions. Windows checks actual native
identity, fixed-drive/reparse boundaries and the installer access-control policy;
it refuses elevated/administrator contexts rather than repairing permissions.
There is no privilege escalation or service-registration option.

A releasable installer needs an exact OS/architecture/runtime matrix, verified
artifact provenance and a doctor report for topology and capability readiness.
Its dry run must leave files, services, privileges and networking unchanged.
Installation must report managed changes, preserve operator-owned files and
demonstrate safe repeat execution and rollback.

Management requires local binding, authentication and restrictive service
permissions. Public container mappings must never expose it. Review
[release status](production-vs-reference.md) before selecting artifacts.

The separate native integration planner has a [read-only tutorial](../tutorials/quickstart-monitor-mode.md).
Creating its manifest does not install an SDK, load CEF or authorize a service.
