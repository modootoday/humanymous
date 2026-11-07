---
id: evidence:root/20261107201200
title: Audit key custody eight package path hash handoff
status: active
created: 2026-11-07T20:12:00Z
updated: 2026-11-07T22:01:18Z
files:
  - {path: paths.json, sha256: 7cf4bf356bd686bf9e95be02f65c3820d631c1646a9657097943a52e9af9350e, bytes: 29853, store: repo}
  - {path: handoff.json, sha256: 0f34eb7f23e604b07f08e125cc9caf462a184330d1f60d6e471eef586906b544, bytes: 975, store: repo}
  - {path: core-slice-01.json, sha256: 5547877df4916f6503f5b6a1b8a939a8fb6a0805facabfab5a9267e950315b67, bytes: 2588, store: repo}
  - {path: platform-slice-02.json, sha256: b735ccd6c208a65a423a40723356262721c13bb940df861c2639b9b0fe54c688, bytes: 3686, store: repo}
  - {path: runtime-slice-02-fixtures.json, sha256: 50caa76818a8c99800e958e552680cfe56769c8ec42e22952b5e3bbb2fef23f0, bytes: 8551, store: repo}
  - {path: runtime-slice-02-source-freeze.json, sha256: 98268e41a937309ae5acf07914a12adce9b752e5ad5a89e6dc0c56aa8aac6609, bytes: 27220, store: repo}
---

# Audit key custody eight package path hash handoff

## What was measured

Current owned Audit handoff for the human-approved eight-public-package baseline.
paths.json records109 tracked crates/audit files and five existing canon files,
114 total. Every worktree byte sequence matches its canonical Git blob at
repository HEADa65dac2feebdd0666c78707ccb99d0e17b868e3a. Latest Audit source
commit99317a5065e6cb2bd1d456678dc0c23124fb73ed. SHA256/length/Git blob are
recorded per file; manifestSHA256
7cf4bf356bd686bf9e95be02f65c3820d631c1646a9657097943a52e9af9350e.

## How to read it

handoff.json records accepted runtime ownership direction, no dirty source,
no owned process/target/path-moving operations and no publication/main-merge
authorization. The new packet itself is outside its baseline hash inventory to
avoid self-reference. This is a pre-move snapshot, not a freeze or qualification.
Root must recheck hashes, freeze an exact group, perform atomic integration and
run new source-bound tests. Full actionable hazards/gates are in
[[plan:root/20261107200953]]. No source move/compiler/WSL/container was launched.

Root subsequently authorized CORE-SLICE-01's four mechanical Audit src import
changes. core-slice-01.json records each before/after hash and proves inverse
replacement reproduces the original bytes. It supersedes only those four source
rows for current handoff; original paths.json/handoff.json remain the immutable
pre-core snapshot. Source-only/fmt-check result, no compiler/Clippy/tests until
Root's coherent-workspace release. No manifests, paths or behavioral code changed.

PLATFORM-SLICE-02 adds platform-slice-02.json: eight source/test files and ten
ordinary test-support reference replacements, each inverse-verified by SHA256.
No Windows object bridge matches. All fixture JSON/source commitments remain
unchanged; graph/category/feature edits belong to Root. No compiler/test/Clippy/
fmt/CI execution took place during the transitional graph. This additive delta
does not qualify current or historical native behavior.

runtime-slice-02-fixtures.json supplies the final read-only Audit contribution
under Root fixture canon662cf7a2: seven exact fixture hashes/provenance, eight
context-qualified test aliases and two independent bin aliases, with remaining
visibility/consumer/packaged-test blockers and no owned leases. These are Root
freeze proposals, not performed moves or executed tests. Original data stays exact.

Root then requested actual source writer release. runtime-slice-02-source-freeze.
json binds106 src/test inputs at8d362f49, actual/tracked inventory equality and
canonical Git blob matches, plus the two ancillary note/license preservation
hashes. Cargo graph is excluded as Root-owned. Audit performs no source moves and
releases source coverage after committing this final packet; only own Specfold
coordination remains. This is not migrated-source or runtime qualification.
