---
id: evidence:root/20261107113926
title: Rust generic caller helper and public projection preflight
status: active
created: 2026-11-07T11:39:26Z
updated: 2026-11-07T11:39:26Z
rel:
  depends_on: [sot:root/dotnet-adapter, plan:root/20261107073000]
files:
  - {path: summary.json, sha256: c559b8769e577102040af6140c5ac2b3e8cba8caa4a1dedec2b101d3da232e8d, bytes: 8182, store: repo}
---

Root D compiled the hosted Rust test executable in28.404s, executed8pure tests
with0failed/ignored and passed scoped release all-target Clippy in18.853s. The
exact projection/guard filters and their filtered counts are preserved separately.
No SDK or managed/native profile ran in this batch.

One context document changed across268captured paths; this is NOT zero drift.
Root accepted a reviewed classification:267other inputs unchanged, path set stable,
and195retained compiler dependency-info files have no reference to the changed
audit integration document. The initial snapshot guard failure and corrected
premature claim remain explicit. No whole shared workspace immutability is claimed.

The immutable helper SHA256 is24b782d1e5ad4c844833bbfba5ccdb9e26f7864760de3d5fcffcded58ba8f821.
A separate sole execution by the ABI owner later produced its narrow managed
generic DLL caller receipt at commitd143f4f1; this preflight does not duplicate it
or promote it to Kestrel/GC-await/NativeAOT/package/product qualification.
Raw complete logs, dependency archive and helper: evidence:root/20261107112318.
