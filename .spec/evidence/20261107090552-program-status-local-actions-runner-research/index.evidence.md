---
id: evidence:root/20261107090552
title: Program status local Actions runner research
status: active
created: 2026-11-07T09:05:52Z
updated: 2026-11-07T09:08:16Z
rel:
  depends_on: [sot:root/program-roadmap]
files:
  - {path: findings.md, sha256: f1daa7770c584a043d598585a870a5c06d7e0f90e00fd0003acafb3c896d0865, bytes: 7197, store: repo}
---

# Program status local Actions runner research

## What was measured

Three research agents surveyed primary sources for Rust Actions execution,
established emulation and Rust CI orchestration/security tools. Root verified
wrkflw main README at 553ecf535f45a59d035ec87ee19b43702e130b3b through the
GitHub API because cached views disagreed. The attached findings distinguish
documented capability, observed maintenance and unexecuted adoption gates.

## How to read it

wrkflw is the first Rust-specific evaluation candidate, not a certified complete
GitHub runner replacement. Local CI's Rust path is experimental, gha-runner is
limited, and Velnor is a self-hosted runner rather than a local scheduler.
act is a non-Rust reference. Task runners and static analyzers do not execute
Actions semantics. No researched candidate was installed or benchmarked here;
the separate direct Cargo comparison does not qualify any emulator.
