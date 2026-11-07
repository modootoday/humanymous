---
id: plan:root/20261107050329
title: Real newtab and measured CEF internal URL support
status: active
created: 2026-11-07T05:03:29Z
updated: 2026-11-07T05:36:25Z
rel:
  governs: [crates/browser/**]
  depends_on: [sot:root/browser-ui, sot:root/chromium-lab-browser]
---

# Real newtab and measured CEF internal URL support

## Goal

Implement a complete app-owned newtab in actual CEF and apply a measured exact
internal-page policy. Rust-only and mandatory sandbox remain; no external browsing,
scoring change, fake tabs or generic native bridge. One current real CEF tab.

## Steps

- [x] Claim shared browser paths and inspect pinned CEF/Meari contracts.
- [x] Read-only blue/red/SRE/privacy/evaluator and upstream CEF source review;
  custom-scheme allprocess registration, unsafe pre-throttle URLs, chrome-urls
  mutation, builtin JS/assets and receipt/privacy requirements become UI-17..22.
- [x] Implement strict routes, typed newtab and upstream serialization/CSP.
- [x] Wire owner scheme, native address controls and CEF navigation/receipt guards.
- [x] Test parser/assets/IME/unknown-route boundaries and actual native safe-page matrix.
- [x] Verify native newtab rendering/form/links/return/close/resize without Hanja replay.
- [x] Record exact support versus policy-denied/unverified, artifact/source hashes,
  fmt/Clippy/tests/build and coordinated attributed integration.

## Record

Do not visit crash/hang/quit/restart or mutate flags/settings to test policy.
Core preflight denial suffices for forbidden URL tests; actual safe-page probes
must identify content, not just commit. Full sandbox/remote navigation/Chrome
parity/multi-tab/new-layout IME/high-DPI/AT remain separate gates.


## Verified delta and residual work

Evidence:root/20261107053625: native five-page matrix plus newtab/return succeeds;
core24/native13 and feature1/4negative flags, fmt/Clippy pass. Actual native
address focus failure was fixed with scoped parent SetFocus/readback; keyboard
Enter, form, links, reload, home,96-DPI resize/close observed. Initial credits
4MiB budget exhaustion retained;8MiB replay identifies actual credits. Final
artifact includes download denial and passes a12-second matrix/cleanup exit0;
interactive export denial is unexercised. Canonical source/working source and
GUI/final artifact identities are explicitly separate. Root accepted the exact
optional dependency lock delta; root alone owns shared Cargo/lock integration.

This plan remains active for integration/checkpoint and the residual qualification
contract. It does not complete the broader browser or SDK program.

| Order | Remaining acceptance | Owner / prerequisite |
|---|---|---|
| 1 | Independently attest utility/network/GPU/job/mitigation policies; investigate WebUI Network Service Not Sandboxed and transient GPU exit78; reject external navigation until complete | Browser/native-security coordination; no sandbox-weakening overrides |
| 2 | Native shell AX bounds and OS screen-reader actions; real new-layout Korean composition/candidate/selection; high-DPI/multi-monitor/Snap/keyboard resize | Browser, actual supported OS/AT/IME; preserve predecessor receipts |
| 3 | Native export-denial callbacks, fixture regression and page asset fidelity including observed broken version logo | Browser; exact final CEF tuple and privacy-safe receipts |
| 4 | B0 detailed observation schema; B1 supervisor/engine lifecycle/crash deadlines; B2 external navigation/history/multi-tab only after full required premises | Browser and root agreed wire/API contracts before code |
| 5 | B3 real collector/session/ingress receipts, B4 owned controls/AT/failure streams and actual pinned Chrome comparisons | Owning adapters/collector plus browser; no third-party probing/scoring change |
| 6 | B5 signed clean-machine install/update/CVE/SBOM and native tuple distribution | Root/delivery/native runner; exact artifact hashes and standard-user runners |
| 7 | Four native SDK crates, stable DLL ABI, framework adapters and Rust scaffolding CLI | Separate native-embedding plan; root serial workspace integration; current bootstrap is process-owning, not in-process SDK |
