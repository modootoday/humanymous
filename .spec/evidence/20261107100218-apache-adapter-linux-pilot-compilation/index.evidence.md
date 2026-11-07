---
id: evidence:root/20261107100218
title: Apache adapter Linux pilot compilation
status: active
created: 2026-11-07T10:02:18Z
updated: 2026-11-07T10:03:09Z
rel:
  governs: [crates/apache-adapter]
files:
  - {path: result.json, sha256: 342eabd0dc6f00c3014e4330903d8534b27b24a5df5e5088093b48735985679e, bytes: 894, store: repo}
---

# Apache adapter Linux pilot compilation

## What was measured

Root allocated B for Linux pilot-feature compilation and pure tests only.
Actual WSL REA-Ubuntu Rust1.99.1 used an immutable Git archive of
271b14e1c46355bd71cce11c4e80efbcb6945150 with the isolated native filesystem
target, jobs1 and incremental0. The pilot-feature bin's ten pure tests passed;
all-target check and warnings-denied Clippy passed. Full extracted-file SHA256
inventories before and after matched. The slot was returned with no live child.

The initial source 2ffa4335df49c111685a764a81f8e4b414685cd4 passed ten tests but
Clippy rejected four findings in Linux-only coordinator/origin code. Raw logs
remain private. The attributed fix 1e5c011a propagates the nested pilot outcome
and resolves the redundant closures and nested conditional. Validation used a
new immutable archive rather than editing the original snapshot. Its initial
before/after inventory also matched. During validation, observed host memory
was31.69GiBfree and82.91percent committed; this is a sampled observation, not a
peak or pre-admission measurement.

## How to read it

These results compile Linux-only coordinator and pilot code, but run only pure
tests. No selected httpd/bindgen build, Apache process, IPC, network or Docker
pilot ran. All45native scenarios remain NOT_RUN. No qualified host/ABI tuple
or full product success is claimed. Windows pilot-feature pure10/check/Clippy
also passed earlier; its missing pre-command hash receipt is recorded in the
owning plan and is not promoted to an exact before/after source receipt.
