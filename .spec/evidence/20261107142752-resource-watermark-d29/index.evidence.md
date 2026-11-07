---
id: evidence:root/20261107142752
title: Watermark D29 first Clippy failure and source correction
status: active
created: 2026-11-07T14:27:52Z
updated: 2026-11-07T14:31:21Z
rel:
  depends_on: [sot:root/resource-watermark, plan:root/20261107135500]
files:
  - {path: result.json, sha256: 527abcef5b4102a245f633754dd12741b60eca5bfe9585f1466f7dab05ffa973, bytes: 2100, store: repo}
  - {path: corrected-source.json, sha256: cfc2388509e39bcf972d66bf37fdfbd8ae3d0b42d2107a5b5e051165d4d38029, bytes: 3390, store: repo}
---

# D29 first execution

Root assigned exact27f1d1d1ffe39e9cfb2d80671f4d45c0335d9701 to the exclusive
native Windows target validation-audit, jobs1/incremental0/offline/locked after
the Pages window. A separate Linux Audit GNU stream was explicitly authorized
by root; no heavy SDK/Docker/release overlap was requested. Initial admission
was82.0617% committed and30.9008GiB free, with per-gate rechecks. The source Git
archive SHA256 is51e678fa03a9a476a795ac3c77c557f7627d8419ba4f8bfd2382f56e55ba86c3,
69632000 bytes. The existing target is reused, not a cold performance experiment.

## Actual terminal outcome

Tool session35125/controller41332 ended overall exit1. fmt exited0, all-target
check exited0, strict all-target Clippy exited101. Its first error was
clippy::chunks_exact_to_as_chunks on codec/text.rs101, suggesting
as_chunks::<8>().0.iter() instead of chunks_exact(8). No lint was suppressed.
All-target unit tests were NOT_RUN because the allocation stops at first failure.
The source contains29 authored tests; this receipt never reports them as passed,
failed, ignored or filtered executions. Earlier A16 applies only to9b064dbd.

Exact commands and gate timestamps are in result.json. Raw admissions, metadata,
versions, process census, stdout/stderr and snapshots remain privately under
`.spec.local/root/evidence/20261107142752-resource-watermark-d29/`.

## Closure and limits

3571 files were recorded before and after, with0difference records:2579 full
archive files,970 selected registry source files across20 packages,21 pinned
Rust toolchain bin files and1controller binary. Ancestor/user Cargo configuration
paths were enumerated for each snapshot. Environment identities were hashed.
This does not snapshot all Windows SDK/linker/system DLLs or host OS files; no
complete native toolchain immutability claim follows. Source and dependency
versions were not modified. The immutable failed archive remains unchanged.
Target D was returned after terminal inspection and no owned compiler remained.

## Separate source correction

Under the accepted source-writing scope, the owner released/reclaimed misc with
crate paths, wrote the plan delta, and replaced only the complete-group iterator
with as_chunks::<8>().0.iter(). This preserves discarded partial-group behavior.
An existing JSON whitespace test now explicitly asserts that an extra partial
group does not prevent recovery. It is still29 authored tests, not an executed
count. Direct rustfmt is source formatting only; no correction retry ran here.
corrected-source.json binds this different package source. New exact-source
allocation is required for qualification; the first failure is not overwritten.

PNG/WASM/dependency adoption, HTTP, real resource pilots and Docker are outside
this gate and remain incomplete. The overall fourteen-file migration is unproven.
