---
id: evidence:root/20261107100735
title: Source-bound Windows Rust dotnet release artifacts
status: active
created: 2026-11-07T10:07:35Z
updated: 2026-11-07T10:07:35Z
rel:
  depends_on: [sot:root/dotnet-adapter, plan:root/20261107073000]
files:
  - {path: summary.json, sha256: 3c0181a1b170aa04f998455358462e5e036c6d035800c70c956fd8cb6266f55f, bytes: 5914, store: repo}
---

Root-allocated C built the release Rust facade and hosted Rust qualification
test executable from the standalone clean committed source
2ffa4335df49c111685a764a81f8e4b414685cd4. Actual commands returned0 in76.032s
and12.616s.261recursive source/configuration hashes are unchanged and source
remained clean. Cargo JSON identifies the exact DLL and test executable; their
hashes, sizes, common graph hashes and compiler/target are in summary.json.

The long-path clone recovery and initial failed admission guard are retained as
preparation failures. A fresh passing resource measurement allowed the build;
no guard override was used. Slot C returned after terminal compilation.

This is compilation only. The harness used --no-run; no tests, actual PInvoke
load, SDK compilation, Kestrel, managed GC, NativeAOT, NuGet install or Docker
product executed. Host-qualified tuples remain empty. Raw complete logs and
strict next-stage provenance: evidence:root/20261107100408.
