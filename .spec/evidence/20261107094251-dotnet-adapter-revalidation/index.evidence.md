---
id: evidence:root/20261107094251
title: Latest Rust dotnet host runner typecheck and pure validation
status: active
created: 2026-11-07T09:42:51Z
updated: 2026-11-07T09:42:51Z
rel:
  depends_on: [sot:root/dotnet-adapter, plan:root/20261107073000]
files:
  - {path: summary.json, sha256: 863fa7fc88ad1e326146b40941b99af501a5ad30ea84508d81dc38777a2d61e6, bytes: 5705, store: repo}
---

Latest source after20544bf6 passes both packages all-target check/Clippy and
12core+1export pure tests with0failed/ignored/measured/filtered. Root allocated
default target/jobs1/incremental0; A returned after commands.259recursive selected
source/configuration hashes are identical before/after. summary.json preserves
captured owned/common graph hashes and actual command timings. Whole shared
worktree immutability is not claimed; the observed HEAD is provenance context.

The new package/support/host fixture Rust source type-checks. None of its ignored
actual SDK/host/package tests executed. No C# compilation, dynamic PInvoke,
Kestrel, managed GC, NativeAOT, clean NuGet install or Docker product qualification
is established. managed_scope was compiled only. Actual host tuples remain empty.
Complete private raw logs: evidence:root/20261107094127.
