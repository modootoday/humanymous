---
id: evidence:root/20261107201200
title: Audit key custody eight package path hash handoff
status: active
created: 2026-11-07T20:12:00Z
updated: 2026-11-07T20:15:18Z
files:
  - {path: paths.json, sha256: 7cf4bf356bd686bf9e95be02f65c3820d631c1646a9657097943a52e9af9350e, bytes: 29853, store: repo}
  - {path: handoff.json, sha256: 0f34eb7f23e604b07f08e125cc9caf462a184330d1f60d6e471eef586906b544, bytes: 975, store: repo}
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
