---
id: evidence:root/20261107121154
title: Pure prebuilt protocol and managed source scope validation
status: active
created: 2026-11-07T12:11:55Z
updated: 2026-11-07T12:11:55Z
rel:
  depends_on: [sot:root/dotnet-adapter, plan:root/20261107073000]
files:
  - {path: summary.json, sha256: 9cacc273a19769e22dc312202c406cd319af7a314dcb6c1ee5b1143d59e8211e, bytes: 2758, store: repo}
---

Root A executed eight protocol pure tests and one exact-three-CSharp scope test;
all passed with zero failed/ignored. All-target check and strict Clippy passed.
Counts and filtered denominators are separate. The runtime fixtures merely
compiled; no Kestrel, GC-await, NativeAOT, SDK, account, SCM or actor executed.

Source is immutable base0a1bb36e plus the two exact owned test overlays, not a
whole-current-HEAD build. All2369archived source files match before/after;
219actual dependency-info files are retained. Registry package bytes were not
captured before/after; Cargo.lock remains part of the captured first-party graph.
The original PowerShell comparison failed because unlike row representations
projected null fields. Its output and both byte snapshots remain intact. Parsing
both snapshots to the same schema gives zero differences; this correction is
explicit, not an erased failure or an unqualified compiler-source attestation.

The newly compiled actual IPC-abort/down fixtures were NOT_RUN. Request deadline
coverage no longer masquerades as independently expired authenticated evidence.
The helper still needs accepted launch/reader integration before Kestrel execution.
Private complete evidence: evidence:root/20261107120724.
