---
layout: doc
title: Installation qualification requirements
permalink: /reference/install-requirements.html
reviewed: 2026-11-07
---
# Installation qualification requirements

**Diátaxis quadrant:** Reference. **Audience:** prospective operators.

Local artifact provisioning and an explicit signed lab-service lifecycle are
implemented. A qualified full-product service installation is not. Build from the reviewed source with Rust
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
Those original provisioning commands do not register a service or escalate
privilege. The separate signed service commands have the boundary below.

## Signed laboratory service lifecycle

`service-plan`, `service-install`, `service-upgrade`, `service-doctor`,
`service-rollback` and `service-remove` operate only on owned
`humanymous-lab-` service names. Planning and dry-run do not write, lock, probe
the readiness endpoint or change native service state. Mutation requires
`--activate`; it never creates a user, elevates privilege or enables autostart.
See [command options](cli-config-policy.md#provisioning-and-signed-service-commands).

An offline strict Ed25519 signature covers `humanymous.release.v1` followed by
a NUL byte and the exact descriptor UTF-8 bytes. Whitespace changes the signed
input. The descriptor is limited to 16,384 bytes and binds version, positive
sequence, native OS/architecture, artifact digest, service name, argument tokens
and numeric loopback diagnostic address/response. Unknown or duplicate fields,
wrong target, invalid signatures and artifact mismatches fail. Keep credentials
out of descriptors and argument tokens. A valid signature does not establish
trusted key distribution, release provenance, revocation or deployment safety.

The implementation uses immutable managed generations, a private deployment
record, a create-new lock and a pending-operation record. Upgrade must exceed
the local highest accepted sequence. Explicit rollback restores only the known
signed predecessor and retains that sequence floor. Unknown service state or
mutated files block removal. Pending records and stale locks require operator
investigation; crash-atomic recovery and power-loss durability are not claimed.

| Backend in source | Qualification boundary |
| --- | --- |
| Linux systemd user runtime registration | Needs an actual usable user manager and source-bound native lifecycle pilot |
| macOS user launchd registration | Needs an actual user domain and source-bound native lifecycle pilot |
| Windows demand-start virtual service account | Needs explicitly delegated service-manager authority in a disposable test context; ordinary refusal tests do not prove positive lifecycle |

No positive complete service lifecycle is claimed here for all three backends.
Native registration and a matching loopback diagnostic remain distinct from
private runtime startup, protected custody, authenticated management/IPC,
graceful drain and full deployment qualification. The current HTTP runtime is
not silently service-compatible. Preserve failed, blocked and ignored pilots.

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
