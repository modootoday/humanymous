---
id: evidence:root/20261107094859
title: Official private Windows dotnet SDK preparation
status: active
created: 2026-11-07T09:48:59Z
updated: 2026-11-07T09:48:59Z
rel:
  depends_on: [sot:root/dotnet-adapter, plan:root/20261107073000]
files:
  - {path: summary.json, sha256: 084073d84628cfe9a7a15fbf539ecfad56839a507ad7c1a5a016e787795c506d, bytes: 1402, store: repo}
---

Root-allocated SDK preparation verified the official SDK10.0.403Windowsx64 ZIP
against its official SHA512 metadata, extracted into a fresh owner-only protected
directory and executed only --version and --info. Both returned0 and reported
SDK10.0.403/runtime10.0.14/win-x64. No global PATH or admin installer was used;
private CLI home/package cache, opted-out telemetry and suppressed ASP.NET
certificate generation were explicit process environment settings.

This is tool readiness only: no managed build, PInvoke load, Kestrel, managed GC,
NativeAOT or package installation executed. Actual host tuples remain empty.
Raw identity/ACL/local paths and complete logs: evidence:root/20261107094726.
