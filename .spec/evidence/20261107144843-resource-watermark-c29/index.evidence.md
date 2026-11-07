---
id: evidence:root/20261107144843
title: Corrected watermark C29 native component verification
status: active
created: 2026-11-07T14:48:43Z
updated: 2026-11-07T14:51:42Z
rel:
  depends_on: [sot:root/resource-watermark, plan:root/20261107135500, evidence:root/20261107142752]
files:
  - {path: result.json, sha256: 291ca8b10414d7279cd0177dce92af381fb8b2f555c26dc32e464ab344e172e3, bytes: 2442, store: repo}
---

# Corrected C29 component result

Root renewed the component allocation for exact corrected source
2eb2069a7851324b90311d54ffe663d8eb9f75cf, using an isolated immutable Git archive
and exclusive target validation-audit. Native Windows x86_64-pc-windows-msvc,
Rust1.99.1, jobs1/incremental0/offline/locked. Archive SHA256 is
5aa38b004ff7fe877a7caaeabe8362f27663f223cf02fdba6e6000aef20a8bfd,
69775360 bytes. Initial admission82.8414% committed/29.5894GiB free and fresh
per-gate checks satisfied root's caps. Other explicitly authorized workloads used
separate targets. This reused target is not a cold benchmark or speedup claim.

## Actual results

Tool session45831/controller5712 ended exit0. fmt, all-target check, strict
all-target Clippy and all-target units each returned0. Actual test footer:
29passed;0failed;0ignored;0measured;0filtered out. The executable was
humanymous_watermark-620887680b1a5a84.exe. Earlier27f1d1d1 Clippy101 and tests
NOT_RUN remain preserved in evidence:root/20261107142752; this corrected run
does not change that original outcome.

Commands and timestamps are recorded in result.json. The29 cases comprise
16payload/ledger,5frame,2opaque,2strict MIME,3text and1base32 test. They cover the
corrected partial-group behavior, CRC/RFC4648 vectors, frame/candidate bounds,
ambiguity, explicit fallback refusal, NUL/UTF-8 and original ledger invariants.
They do not validate complete source grammars or actual resource semantics.

## Recorded input closure

3589 files were hashed before/after with0difference records:2597 full archive
files,970 registry source files across20 conservatively resolved packages,
21pinned toolchain bin files and1controller binary. All recorded configuration
paths were checked; no additional present config file contributed to these
counts. Environment/config identities, process censuses, version outputs,
admissions, metadata and raw stdout/stderr remain privately under
`.spec.local/root/evidence/20261107144843-resource-watermark-c29/`.

This selected tool-bin/config closure excludes the complete Windows SDK,
linker/system DLL and host OS trees. It is not proof of every native system input
remaining unchanged or a clean shared checkout. File counts and difference-record
counts are separate. Source/dependencies were unchanged throughout this run.
Target C/validation-audit was returned to root after terminal/footer inspection;
no owned compiler/test process remained. No source edits or broad reruns followed.

## Qualification boundary

This is a source-specific pure component pass for the corrected low-level
payload/ledger/frame/MIME/text/opaque implementation. The original fourteen-file
scope remains intact. WASM validator/codec adoption, PNG metadata/LSB/alpha/memory/
cancellation, full injection/recovery/trace composition, original complete format
tests, valid-input syntax checking, real resource/browser/HTTP pilots and Docker
product acceptance remain incomplete. There is no scoring/ban/human-proof or
feature-loss-free full migration claim.
