---
id: sot:root/native-security
title: Native OS security boundary and CLI integration
status: active
created: 2026-11-07T01:05:00Z
updated: 2026-11-08T16:08:00Z
verify:
  - cargo test -p humanymous-runtime --features native-security --locked
  - cargo test -p humanymous-cli --locked
rel:
  governs: [crates/sdk/humanymous-runtime/src/native_security/**]
  depends_on: [sot:root/cli-installation, sot:root/adapter-authority, sot:root/crate-architecture]
---

# Native OS security boundary

## Live package layout (measured 2026-11-08)

There is no workspace package `humanymous-native-security`. The live owner is
`humanymous-runtime` feature `native-security`, module
`humanymous_runtime::native_security`. CLI consumes that module without a
separate native-security crate dependency. Empty `crates/native-security/**` is
an untracked ghost skeleton and is not an implementation root. Historical
standalone-crate proposal text below remains chronology; live wiring uses the
runtime paths above. Evidence:
`.spec.local/root/evidence/20261108155700-cursor-spec-gap-audit/`.
Pilot/e2e remain distinct gates; no product e2e claim follows from this path fix.

Create one context-bound native-security boundary for OS token, volume and
handle privacy checks. It is not a detector, generic core or service manager.
Move the existing audit Windows verifier into this context without retaining a
second implementation. Audit storage reexports the same API for current callers.
CLI uses this lower layer without acquiring an audit/crypto dependency. Pure
WASM libraries do not gain native Windows dependencies on their WASM targets.

Windows safe dependencies are winsafe0.0.29(kernel/advapi,MIT,MSRV1.87) and the
already pinned windows-permissions0.2.4. Authored code retains unsafe forbid.
There is no embedded PowerShell, shell fallback or runtime helper executable.
Use native process token user/elevation/groups and native GetDriveType.
Reject any impersonated thread or an unknown thread-token query rather than
substituting a process identity for an effective thread identity. Unknown
token query or volume kind fails closed. Only a local drive-letter root reporting
DRIVE_FIXED is eligible; this is an OS classification, not physical provenance.
Reject elevated or enabled-Administrators tokens. A deny-only administrator SID
does not confer enabled administrator authority; preserve the prior filtered-
token policy rather than inventing a new elevation bypass. Unix uses geteuid; CLI installation parents require mode0700and a non-root
effective UID. No write probe or automatic permission repair is permitted.

| Profile | Allowed allow-ACE principals | Required conditions |
|---|---|---|
| Audit/custody file | exact owner | regular non-reparse file, nonempty DACL |
| Custody directory | exact owner | native child file/directory inheritance |
| Authority directory | exact owner | protected DACL |
| Installation parent | current token user, SYSTEM, Administrators | owner exact user, protected DACL, full self/child owner access with no propagation restriction |
| Installation directory/file | same explicit installation principals | owner exact user, full self owner access, correct native object type |

No installation exception may weaken authority/custody profiles. Null/empty,
foreign, deny or unsupported ACE forms fail closed. Verify the descriptor on
the actual opened non-reparse handle; do not repair ACLs during inspection.
Installer policy recognition does not authenticate an artifact, activate a
service or prove local admin/IPC conformance. Preserve existing dry-run,
digest/collision/rollback and trusted-private-parent conditions. Canonical tests
must prove installer privileged principals never pass strict audit profiles,
foreign grants fail, read-only owner grants fail installation, unsafe volumes
fail, and token elevation/enabled-group classification is explicit.

Critique reflected: avoid crypto dependency inversion, duplicated ACL parsers,
policy booleans/custom allowlists, hidden shell code, null-DACL permission and
filtered-token confusion. Native same-owner/path-replacement premises remain;
these checks are not a sandbox against the directory owner. Pilot/e2e and actual
six-tuple verification remain distinct gates. No scoring change is authorized.

Primary references: Microsoft
[GetDriveType](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdrivetypew),
[GetTokenInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation),
[SID attributes](https://learn.microsoft.com/en-us/windows/win32/secauthz/sid-attributes-in-an-access-token),
and pinned upstream dependency sources from Cargo.
