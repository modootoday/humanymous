---
id: evidence:root/20261107191156
title: CLI installation immutable shared DATA validation
status: active
created: 2026-11-07T19:11:56Z
updated: 2026-11-07T19:16:43Z
rel:
  depends_on: [sot:root/cli-installation]
files:
  - {path: summary.json, sha256: 1712e9f380c6fa99122a8e0922d937ba56a221ef9c8c0069c34952ab8f8c0ff8, bytes: 3269, store: repo}
---

Qualification is HOLD after independent audit CA-CLI-INPUT-01 found missing local
build-source coverage. The initial d395a302 qualification statement is withdrawn;
actual successful commands and immutable archive provenance remain separate facts.

The archive for `f20db16ed32cae78c39057fa35319a142ccc78e3` was used for scoped
Windows DATA-consumer commands with Rust1.99.1/edition2024. It includes Authority
`a898d91a` and Runtime `aba7934d`; no live working-tree files were imported.
CLI retains exact BASE/service/artifact/cap checks and moves the same configuration
and checked authorization through Runtime's shared pre-unseal DATA validation.

Package fmt ran first. Locked offline all-feature/all-target check and strict
Clippy passed. The `--lib runtime_host:: -- --list` inventory was exactly11tests;
the actual run footer was11passed/0failed/0ignored/0measured/15filtered. Reviewed
selected bodies exercise byte parsing, synthetic secret-source calls, plans,
error projection and state transitions, without native/service/account/SCM or
listener execution. The new conversion method is type-checked; this receipt does
not claim an additional end-to-end signed fixture test of that method.

The selected Windows metadata graph contained143packages. Before/after hashes
covered6674files with0difference rows, but the absolute-path exclusion of target
directories also excluded every local package tree because the extracted archive
itself lives below target. Only registry files, five compiler binaries and three
common local files were captured. Local Rust source and package manifests were
NOT captured before/after. Seventeen recorded environment variables matched;
these facts cannot establish the actual compiled local source's stable closure.
Archive and manifest hashes are in summary.json; raw logs, metadata,
environment, admission and split immutable archive are retained privately under
evidence:root/20261107191209. The original private source.zip hash remains in the
raw inventory; its three ordered parts reconstruct the same archive.
Independent audit verified all24attachments and the concatenated archive against
literal Git f20 bytes. This does not retroactively supply missing extracted-source
snapshots. No immediate before/after local snapshot exists elsewhere. A future
allocated run must exclude relative package build directories, assert local
source/manifest coverage and retain both snapshots. No rerun was allocated here.

The original600s work/60s cleanup clock began before preparation I/O at
2026-11-07T19:09:33.5899807Z and ended at19:11:27.4833426Z. Initial admission was
33.534GiB free/67percent committed. Jobs1/incremental0 and a distinct CreateNew
target lease were used. All six command handles terminated with exit0 and the
actual lease marker was removed; no target or artifact-write lease remains held.
No source retry or correction occurred within this run.

The historical `a026bfc6` parser splice defect remains an invalid source checkpoint;
`d79560e1` corrected it before this immutable run. Earlier passes are not applied
to the defective source. Current pure regression proves supplied bytes are parsed
without reopening an invalid-NUL path and rejects tampering before secret reads.

Native preparation remains Unsupported. Real P/S/O identity, protected issuer,
atomic claim, W, transport, Ready, actual service lifecycle and audit drain/flush
remain open. No local services/accounts were changed. Product qualification is
false; the dedicated genuine-hosted service component is a separate gate.
