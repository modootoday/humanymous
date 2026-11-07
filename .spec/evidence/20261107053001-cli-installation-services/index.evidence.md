---
id: evidence:root/20261107053001
title: Signed CLI service component qualification summary
status: active
created: 2026-11-07T05:30:01Z
updated: 2026-11-07T05:39:01Z
rel:
  depends_on: [sot:root/cli-installation]
files:
  - {path: summary.json, sha256: 2359b3518572b43f2eff40bfd1e344dc7c270f7893a8a4f83514817f9d2690e9, bytes: 2762, store: repo}
---

# Applied source and measured scope

CLI source commit83397fd5d7289a326b890aa547a8f9e1a04fe03f. Actual Windows x64
Rust1.99.1 package checks passed16unit tests and2process pilots. One positive
native-service lifecycle case was ignored and is unqualified. Mock native-manager
state-machine tests are separate from actual process/refusal evidence. Zero-test
fixture/doctest harnesses are not counted as behavior cases.

The actual CLI verified signed exact-byte descriptors, dry-run snapshots of
names/content/mtime/DACL, a running native Rust fixture, and real ordinary-actor
SCM creation refusal with staging cleanup. No privileged host changes were made.
This is not successful SCM deployment, Linux/macOS qualification, runtime private
configuration/credential integration or full product/Docker e2e.

Warnings-denied package Clippy, fmt and default release build passed. Default
release emitted only humanymous with no enabled features, excluding the service
fixture. Source manifest before/after these checks matched. All17CLI input files
matched canonical Git bytes after the attributed commit. Common Cargo and two
native-security files were validated working inputs pending their owners' commits;
their exact hashes are in the attached sanitized summary. This is not a claim
that an entirely clean repository commit was tested.

Raw attempts/source/environment observations are private, including earlier path,
Clippy and shared-lock failures. CLI scoped Specfold checks passed. The global
private graph check still failed on other in-progress paths and is not waived or
reported green. Remaining native service/controller, six-tuple, runtime/credential,
release trust and recovery gates are explicitly open in CLI SoT/plan/INTEGRATION.
