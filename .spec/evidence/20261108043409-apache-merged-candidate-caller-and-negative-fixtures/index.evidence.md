---
id: evidence:root/20261108043409
title: Apache merged candidate caller and negative fixtures
status: active
created: 2026-11-08T04:34:09Z
updated: 2026-11-08T04:42:53Z
rel:
  governs: [crates/apache-adapter]
files:
  - {path: handoff.json, sha256: fb40a10e3693593b6eb60db7c5ebc785bc5294d8c914bffb64c685d1557857b3, bytes: 5995, store: repo}
  - {path: delta.patch, sha256: 3c2fd5a20060c8ff2b7b24d077b38ceb1a64afc6357930b56adf0be4bd8c4777, bytes: 30511, store: repo}
---

# Apache merged candidate caller and negative fixtures

## What was measured

Apache-only preimage-bound candidates are authored under private bundle
.spec.local/root/evidence/20261108102000-apache-merged-candidate. Input is root's
READ-ONLY tools-integration-5f1010d4 isolated eight-package materialization.
handoff.json binds exact raw SHA256 preimages, replacement bytes and unchanged
manifest/build/header-resource dependencies. No file there was edited. Branch
was checked as codex/rust before writes; no branches/worktrees/refs were created,
switched, pushed or deleted. Source application remains root-owned.

The candidate replaces old Apache Cargo selectors with humanymous-adapters and
exact Apache/qualification/export features. A merged rlib-only library is built
as a cdylib explicitly with cargo rustc; production/probe/pilot use separate
target directories. Public humanymous.so artifacts are proposed under pinned
Apache version/target/production-or-probe directories, not a universal native
ABI or a shared overwrite destination. LoadModule, independent copies/hashes,
contract paths and ABI-receipt caller move together. This path layout is a
root-review proposal, not independently accepted support. No global native
header/profile, Cargo manifest/lock, export symbol or unsafe scope changed.

Five meaningful Rust negative tests are authored: cross-role/target/probe artifact
substitution; packaged Apache header/closure/receipt identity substitution;
late pool cancellation versus replacement request; expired complete request
permit retention; wrong-generation admission versus another live permit.
The first two consume the actual package-local contract/upstream-lock resources,
using synthetic receipts only as parser fixtures, never SDK/runtime evidence.
Exact45-case accounting and old failed/historical receipts are preserved.

Formatting the partial overlay initially could not resolve unchanged child
modules; per-file rustfmt with skip_children=true passed afterward. actionlint
initially caught an unquoted YAML command ending in apache::; quoting it fixed
the candidate and actionlint passed. These are source-preparation checks only.
The workflow proposal is manual-only: no deleted validation branch is recreated
and applying it does not silently start an unallocated heavy push workload.

## How to read it

NOT EXECUTED: compiler, tests, Clippy, SDK, OS/native processes, Docker/browser,
or hosted jobs. Root must review exact preimages and layout/policy, apply the
overlay atomically, then allocate scoped pure test/check/Clippy and actual
generated-header/module/three-MPM/artifact/cleanup validation. Five new tests
are not five passes. The historical b40045/45 tuple does not qualify this code.
No main merge, publication or migration-goal completion is implied. Common
manifest/features, native ABI header/profile and other owner sources remain
outside this candidate's scope.
