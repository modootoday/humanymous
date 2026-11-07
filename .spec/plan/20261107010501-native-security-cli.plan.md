---
id: plan:root/20261107010501
title: Native security extraction and Rust CLI permissions
status: active
created: 2026-11-07T01:05:01Z
updated: 2026-11-07T01:05:01Z
rel:
  depends_on: [sot:root/native-security, sot:root/cli-installation]
---

- [x] Extract one real native security implementation, retaining target-scoped dependencies and audit namespace reexport.
- [x] Separate CLI orchestration, permission contexts and tests; preserve command/receipt and provisioning-only boundary.
- [x] Replace PowerShell/Unix id and shell-based test fixtures with safe native APIs.
- [x] Prove strict audit profiles cannot accept installer privileged-principal exceptions.
- [x] Test token/elevation/deny-only groups, fixed volume rejection and native ACL distinctions.
- [x] Run Windows CLI6/native-security6, workspace all-target tests and warnings-denied Clippy.
- [x] Check eight pure libraries on WASM and dependency advisories; retain existing ttf-parser maintenance warning.
- [x] Execute real CLI/runtime binaries in network-none non-root Docker, including owner-write prerequisite refusal.
- [ ] Qualify this exact new source SHA on six native tuples, including the new Unix parent-permission regression.
- [ ] Complete signed runtime/service installation and docs freshness in their later accepted slices.

No service activation, full browser/capture/admin qualification, registry publish,
final cleanup or main merge is implied by these bounded provisioning results.
