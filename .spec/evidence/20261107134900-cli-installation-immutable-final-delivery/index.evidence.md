---
id: evidence:root/20261107134900
title: Immutable final-delivery watchdog scoped validation
status: active
created: 2026-11-07T13:49:00Z
updated: 2026-11-07T13:49:00Z
rel:
  depends_on: [sot:root/cli-installation]
---

Root granted one D renewal using immutable Git archive
792a2fab05bf21d3bc6e9e9f165a103ca88a66c6, with runtime6c02bd22 ancestry verified.
It contains the original-deadline validation/completion checks, final report
delivery ordering and removal of hard-expiry stderr output. No shared working
tree source was used. Archive SHA256 is
1fc473614af80a8220228218f419345967e7bde69c18884f3d6e3356fc93ecc2.

Locked offline all-target CLI check, reviewed pure runtime_host tests, all-target
Clippy with warnings denied and rustfmt check of both changed Rust files passed.
Exact pure result is9passed/0failed/0ignored/15filtered. Rust is1.99.1, edition2024,
Windows x86_64-pc-windows-msvc. Exclusive target/validation-cli, jobs1/inc0 were
used. Final admission was31.73GiB free/80.31percent committed. No native service,
account, network or blocked-output boundary fixture was executed.

Before/after closure observations:

| Input | Inventory | Equal manifest SHA256 |
|---|---|---|
| Entire extracted archive | 2508files | c994557fd04e80e573a58110a2dbc131eb4ef5418aac0d3ea11de6665f7d593b |
| Actual registry source files in resolved CLI dependency graph | 6670files,142resolved nodes including local packages | 0ffba0f99183393ceb4ea0b0ace1d31331276b2457104f34992f08952eb1183e |
| Rust toolchain bin/lib/libexec input trees | 155files | 19b6649c4767bb33eed8117d1d13f68baa9d8a9224d4574907f41e72ddff00cb |
| Exact rustc/cargo/rustfmt/cargo-clippy/clippy-driver binaries | 5files | 385a73014f2eec5f884cbff874f8c10fdfb722e353313fd29d4f13d34a9e0a9c |
| User/workspace Cargo configuration candidates | 6absent files | 88609967f008bbe7d97156fdacb0aba9cc84b51580b42070b309a6bc5691c407 |

Additional root/target/archive Cargo config candidates were absent. Relevant
tool/environment values were hashed privately; explicit target/jobs/incremental
settings are recorded above. This inventory does not attest Windows system DLLs,
the external SDK/linker installation or loaded executable pages. Registry graph
features are a metadata superset, not a claim that every node ran. Unused toolchain
share/docs are excluded from actual tool inputs.

The first full-sysroot inventory accidentally included unused documentation. Its
verified owned collector process was cancelled, not a validation or foreign
process. The scope correction note and initial passing checks remain private.
After actual tool-input inventory, the same frozen check/pure9/Clippy set was
repeated once within D; all returned zero and complete final input hashes matched.
No source edit or automatic source-drift rerun loop occurred. D was returned and
all owned handles were terminal before evidence work.

Private bundle20261107134200 retains the archive, metadata, all raw attempts,
configuration/tool/source inventories and scope note with attachment hashes.
The6401148byte source archive exceeds Specfold's per-file attachment limit and
is retained as two sequential binary parts in the private attachment manifest;
the original archive is also retained unchanged. No evidence-store policy changed.
The snapshot and scoped pass supersede neither the earlier mixed-dependency
observation nor actual native proof. Blocked stdout/stderr termination, SCM
lifecycle, protected observer/witness bootstrap and full product qualification
remain NOT_RUN or incomplete. No unconditional deadline-liveness claim follows
from pure expiry injection or removal of a logging call.
