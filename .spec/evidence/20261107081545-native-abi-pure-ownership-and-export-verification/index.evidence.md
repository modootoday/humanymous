---
id: evidence:root/20261107081545
title: Native ABI pure ownership and export verification
status: active
created: 2026-11-07T08:15:45Z
updated: 2026-11-07T10:55:34Z
rel:
  depends_on: [sot:root/native-abi, plan:root/20261107072500]
files:
  - {path: summary.json, sha256: a55a59d498914d90c7ba788b663def7b105f390f14f37c71419e7273d5673f49, bytes: 2406, store: repo}
  - {path: hosted-startup-failure.json, sha256: aaef59afbb25b88e38aab3539a46c2129f6e84cb4401aff11582fe991980de59, bytes: 928, store: repo}
  - {path: hosted-813-ubuntu.json, sha256: c9e34a3a2368ccf6bd1dbf105ef4c04c5454a7d5c92d126a7189646ae1fcca0b, bytes: 1446, store: repo}
  - {path: hosted-813-windows.json, sha256: 616e7f6320c351bbcc924c6e0b29f90b1b50b5f7d93db5a1472c094d474dc4e7, bytes: 1392, store: repo}
  - {path: local-windows-artifacts.json, sha256: b982d034b1988ff12c3d7b782d855a0e24eb1919f6b59f56bbc5b32aff9941be, bytes: 951, store: repo}
  - {path: managed-jit-attempt-2.json, sha256: 99473078db1c0cb9fd6fed49e1c6f58e3bfef3e9aa2d28f7d9df872075fa8755, bytes: 822, store: repo}
---

# Native ABI pure ownership and export verification

## What was measured

Rust1.99.1/edition2024, Windows x86_64: locked all-target core16/export6/header
diagnostic-parser3 units passed, with zero failed/ignored/measured/filtered.
Same two packages passed all-target Clippy with warnings denied. Source/header
tool main programs were not executed. Root assigned target/validation-agents,
jobs1/incremental0 and the owner returned slot B immediately after checks.

Private bundle20261107081250-native-abi-local-pure-verification contains raw logs
and eighteen before/after recursive owned/shared source records. All hashes and
byte counts and the observed HEAD stayed unchanged. A raw metadata empty-array
encoding issue is preserved and corrected by an independent comparison sidecar;
no historical/raw bytes were rewritten. The public summary contains sanitized
scope and hashes, not private machine paths or inputs.

## How to read it

This working-tree component receipt does not qualify the whole workspace, an
actual dynamically loaded ABI, generated header, C/C++ compiler, managed host,
RAII/exception behavior, cross-OS runtime or product e2e. Parser fixtures are
synthetic compiler diagnostic text only. Header generation and hosted Clang layout
execution remain separate gates; approved managed generic-DLL loading will be
reported separately from the .NET facade and middleware profile.

## Separate local Windows artifact execution

The attached local-windows-artifacts.json records a later A allocation: pinned
cbindgen0.29.4 generated a4600byte header, and real MSVC C11/C++14 compilation
produced two objects. The native DLL built; passive inspection observed x64 and
the expected export. Example strict Clippy and formatting passed. Twenty captured
source files stayed unchanged across the stage. This is a worktree hash-bound
local receipt, distinct from the25unit denominator and genuine hosted layout gate.
Dynamic calls, C++11, layout equality, C/C++ callers, RAII and exceptions remain
not_run. Private raw bundle20261107104057 retains tool/source/artifact identities.
The subsequent managed JIT restore passed, but actual pilot build failed with two
SYSLIB1051 aggregate-marshalling diagnostics. The .NET owner was notified; raw
failure logs remain private. No probe or listener ran, and native header success
does not convert that managed build failure to a passing caller result.
After the .NET owner's source fix d799a126, separate attempt2 restored and built
successfully with zero warnings/errors. Seven clean committed managed inputs
stayed hash-unchanged. The attached managed-jit-attempt-2.json binds assembly and
generic DLL hashes; private raw logs retain both attempts. This is framework-
dependent JIT build evidence only. Actual generic calls, NativeAOT and package
qualification remain not_run pending the shared bounded Rust launcher.

## Separate hosted startup failure

Direct GitHub connector reads of run42354873001 and its latest-attempt jobs
confirmed sourcea26d1c3f08b365ca93545c6a51343095868edb45, completed/startup_failure
and zero jobs/tests. The attached API projection is separate from the local pure
receipt. Git ancestry confirms source checkpoints3ef6fdc and6e33131 are included;
local evidence commit7cdfb31 is not included in that run's source. Root attributed
the startup failure to the caller's Actions permission ceiling; the run/jobs API
projection itself contains no cause field. No native or product acceptance follows
from a zero-job run, and a later fixed run must not erase this failed attempt.

## Fixed-source Ubuntu partial execution

Run42355168729 targets exact8132117e7bf1445939d7dee9066fcf45ccf0915b. Its Ubuntu
native job128351926093 built the full workspace/all targets, including native
core/export. The following actor step failed after15of88required harnesses:
51passed tests,0failed tests,4raw ignored,1error and coverage_complete=false.
No native ABI harness appears in the executed list, so ABI hosted tests remain
not_run. The attached sanitized projection retains that denominator; private
bundle20261107083655-native-abi-hosted-ubuntu-attempt-813 retains original decoded
logs and the parsed workspace summary. This is not a parent-run terminal claim.

## Windows partial execution

The same813source's Windows native job128351926153 also passed workspace build
but failed the actor step:15of89required harnesses,55passed tests,0failed tests,
4raw ignored and1error. It stopped at host_process, with no native ABI harness
executed. The attached Windows projection and private
bundle20261107090311-native-abi-hosted-windows-attempt-813 preserve its separate
denominator. Local pure25 and these failed hosted partials are not interchangeable.
