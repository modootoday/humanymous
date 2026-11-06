---
id: plan:root/20261107080000
title: CLI provisioning slice
status: active
created: 2026-11-07T00:00:00Z
updated: 2026-11-07T00:00:00Z
rel:
  implements: [sot:root/cli-installation]
---

Implement cohesive provisioning module and command parser with SHA-256 artifact
boundary. Verify in an isolated workspace until root integrates crates/cli.
Record Windows fixture pilot and negative tests privately. Root owns membership,
lockfile and full workspace acceptance. No Go, docs, service or scoring edits.

Implemented doctor/install/rollback with read-only plan, caller SHA-256, explicit
local-management acknowledgement, native private parent checks, path/reparse
rejection, create-new collision prevention and failure cleanup. Six Windows unit
tests and a real CLI temp-fixture pilot passed in an isolated workspace. Evidence:
private CLI evidence bundle 20261107080001. Public integration request is crates/cli/INTEGRATION.md.
Root integrated membership; actual workspace package tests (6) and Clippy with
warnings denied and --locked passed. Full workspace checks, Linux/macOS pilots, real server local management/IPC,
release artifact provenance and privileged/service registration remain open.


