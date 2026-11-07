---
id: evidence:root/20261107223851
title: CLI installation consolidated runtime consumer validation
status: active
created: 2026-11-07T22:38:51Z
updated: 2026-11-07T22:39:26Z
rel:
  depends_on: [sot:root/cli-installation, sot:root/structural-development]
files:
  - {path: summary.json, sha256: d6b741e549380b959648fec4af1c7837310d678eb7050cb66ffd7f92fc44d462, bytes: 3832, store: repo}
---

Immutable source8e51e55881c5e3d6389ab2bb853d6236748805ca passed the Root-allocated
Windows CLI consumer profile after Runtime consolidation. No live working-tree
source or overlays entered the archive. The consumer namespace and formatting
checkpoints areab08d065 andb4cd31ec; this receipt does not qualify later source.

First, rustfmt checked exactly17frozen consumer files with edition2024 and
skip_children=true, without formatting foreign source. The normal/build-only
dependency tree under no-default-features showed Runtime native-security only,
without Gateway. Separate locked/offline CLI all-target checks passed for
no-default-features and all-features. All-target/all-feature strict Clippy passed.
The explicitly enabled service contexts therefore compile without making Gateway
a normal CLI dependency feature.

The derived library test inventory matched the historical sourcef20/cf644 names
exactly: four control, one failure, four input and two plan tests. Only that reviewed
runtime_host:: subset ran. Exact footer:11passed/0failed/0ignored/0measured/
15filtered,0.05s. Full library/integration/service/SCM/native tests were not run;
compiling their targets does not count as execution or native qualification.

Before compiler work, source coverage asserted five local manifests,471Git-listed
Rust files and exact CLI runtime_host/input.rs, plus every canonical selected
package file including fixtures/modules. Both immediate snapshots contain543local
files,6666registry files,3root Cargo/lock/toolchain files and66compiler/targetlib
files,7278total. All546local/root files match canonical Git blob bytes in both
phases, with0missing/unexpected entries and0hash/key differences. Seventeen
environment variables also match. This selected closure is not a complete OS image
or installed-toolchain census.

Original390s work/450s total clock began2026-11-07T22:36:34.8638504Z. Work ended
22:38:23.0532810Z,108.19s; exact owned single-file marker cleanup completed
22:38:43.6293686Z,128.77s from the original start. All eight command handles
terminated exit0; no forced stop, hidden retry or source correction occurred.
The exclusive cli-runtime-consumer-v0 target was returned with marker removed.

Private evidence:root/20261107223854 indexes34raw attachments: admission, logs,
normal tree, metadata, exact pure names, before/after coverage and canonical joins,
source/environment hashes, result/cleanup and three ordered archive parts. The
retained inventory also identifies the original unsplit archive hash. Public
summary.json records source-bound accounting without native PIDs or private paths.

Native preparation remains Unsupported. No service/account/listener/SCM/process
pilot, Docker, CI or release operation was executed. Real issuer/delegation/claim/
W/transport, Ready, service lifecycle and audit drain/flush remain separate open
gates. No container shipping or product qualification follows from this result.
