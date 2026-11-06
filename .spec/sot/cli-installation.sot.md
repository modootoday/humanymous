---
id: sot:root/cli-installation
title: Explicit CLI provisioning boundary
status: active
created: 2026-11-07T00:00:00Z
updated: 2026-11-07T00:00:00Z
verify:
  - cargo test -p humanymous-cli
rel:
  governs: [crates/cli/**]
---

# Contract

The CLI provisions a caller-supplied regular artifact after verifying its SHA-256
against an independently obtained digest. It does not claim that this artifact is
a working Core/Gate server. It never executes the artifact. Doctor reports native
OS and checks the same prerequisites as installation. No network operations occur.

Install requires an absolute destination directly below an existing private,
caller-controlled parent, a regular non-linked artifact, its expected digest and
explicit acknowledgement that local management binding and IPC ACLs of the artifact
were independently verified. This acknowledgement is a deployment precondition,
not proof of runtime conformance. Windows UNC, device paths and non-fixed drives are rejected before filesystem reads. Unix mounts are an operator-controlled local-filesystem prerequisite; the portable implementation cannot prove mount locality. No default server configuration is invented.
Windows requires Windows PowerShell/.NET Framework for read-only local drive and ACL inspection without module loading; unavailable
inspection fails closed. Linux/macOS require parent owner matching the process and mode with no group/other
access. Windows requires protected DACL with allow entries restricted to current
user, SYSTEM and Administrators; elevated tokens are refused. Other OSes fail closed.

Dry-run performs reads and prerequisite diagnostics only: no mkdir, write, lock,
network, privilege changes or service changes. Actual provisioning creates only
the destination, artifact and fixed receipt. Existing destinations are accepted
only when their complete content matches the requested installation. Files are
never overwritten. Rollback removes only an exact verified managed installation;
changed/extra files block deletion. Installation failure removes only files created
by this invocation and reports cleanup failures. A crash may leave a partial tree;
it is deliberately not accepted as a completed install.

Threat review: private parent control is mandatory because portable path checks
cannot defeat concurrent hostile replacement of directories. No symlink/reparse
point in any traversed path is allowed. Concurrent commands in a trusted parent
may fail safely; create-new and create-dir arbitrate ownership. Rollback assumes
exclusive administrative access during its transaction. Crash-atomic removal and
power-loss durable installation are not provided. System service registration and
privilege elevation are not implemented and are rejected, including explicit flags.

## Acceptance and residuals

Unit tests cover digest mismatch, containment, collision, mutation, repeat install,
rollback and partial write cleanup. Windows temp-fixture pilot must show unchanged
dry-run snapshots and repeat/rollback. Linux/macOS pilots, runtime binding/IPC
conformance, signed artifact release provenance and actual workspace checks remain
integration gates; a local fixture is not deployment/e2e authority.

Official implementation references: [Rust filesystem race guidance](https://doc.rust-lang.org/stable/std/fs/),
[Get-Acl](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-acl?view=powershell-7.4),
[ACL inheritance rules](https://learn.microsoft.com/dotnet/api/System.Security.AccessControl.FileSystemAccessRule).

