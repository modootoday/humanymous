---
id: evidence:root/20261107081545
title: Native ABI pure ownership and export verification
status: active
created: 2026-11-07T08:15:45Z
updated: 2026-11-07T08:16:39Z
rel:
  depends_on: [sot:root/native-abi, plan:root/20261107072500]
files:
  - {path: summary.json, sha256: a55a59d498914d90c7ba788b663def7b105f390f14f37c71419e7273d5673f49, bytes: 2406, store: repo}
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
