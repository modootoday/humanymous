---
id: plan:root/20261107033358
title: Browser UI research scope and Chromium rendering shell
status: active
created: 2026-11-07T03:33:58Z
updated: 2026-11-07T04:17:05Z
rel:
  governs: [crates/browser/**]
  depends_on: [sot:root/browser-ui, sot:root/chromium-lab-browser]
---

# Browser UI research scope and Chromium rendering shell

## Goal

## Steps

## Record

## Accepted brief

Expand the browser into an engine-observation research instrument for user-directed
nonintegrated sites. Keep passive research separate from owned active experiments,
visibility planes distinct, observer contamination explicit and scoring frozen.
Use actual Chromium content, Chrome rendering comparisons with pinned cohorts,
and compact top tabs/address plus large content and collapsible right/bottom dock.
Do not claim delivered external navigation, universal automation detection or
Chrome parity from a static local pilot. Rust-only/Meari/sandbox remain mandatory.

## Work and acceptance

- [x] User confirmed browser-first layout and actual Chrome-level rendering goal.
- [x] Root granted exact Chromium laboratory SoT path; shared Cargo/roadmap remain root-owned.
- [x] Parallel read-only blue/red, privacy/SRE, evaluator/render-fidelity panel; P0/P1 reflected in RSRCH/UI requirements.
- [x] Preserve previous Hanja changes/evidence without replaying the stopped keyboard routine (b65e2f0).
- [x] Define research scopes, visibility/provenance, capabilities, privacy/loss/budgets, observer controls and Chrome parity matrix in canonical SoTs.
- [x] Implement compact native controls and shared viewport/dock geometry.
- [x] Core/native library tests, Clippy/fmt/build; no unrelated source edits.
- [x] Bounded compact-shell previews; later user-approved borderless/resize scope follows plan:root/20261107041001. No Hanja replay.
- [x] Record actual observed page allocation/dock behavior and residuals, evidence and attributed commit.

## Critique and remaining slices

P0 scope/authority leaks, fake site detection and fake sandbox readiness are
reflected in RSRCH-02..07; contamination/ground-truth leakage and false AT
attribution in03/04/10/11; data/model export and cleanup in12..14; rendering
fidelity and cohort mismatches in15; actual page/inspector geometry in UI-06..12.

External passive navigation admission/network policy, dynamic website renderer
comparisons, multi-tab/history/address receipts, complete feature registry and
observers, data retention/export controls, full AX/text zoom/DPI/cross-OS and
complete sandbox/signing/product Docker gates remain later slices. The compact
shell must visibly retain those unavailable states. Baseline fixture remains local.
Native preview delta: the single initial run established right/bottom docking
and native size readback, then exposed the inherited1000-DIP initial window
height exceeding the desktop and a missing reload glyph. Replace default height
with720, use readable Reload text, and remove the native-only unattached placeholder
AX announcement. One bounded corrective UI preview is justified by these defects;
no Hanja/native typing replay is part of it.


## Latest evidence and handover

Evidence:root/20261107034226 records final core18/native11, explicit native
security-override1 test covering4 flags, warning-denied Clippy and targeted fmt.
Actual Windows96-DPI resize changed1116x759 ->1036x759 ->1036x699 ->996x669
->976x649, with6 CEF size readbacks. OS cursor identities included horizontal,
vertical, both diagonals, text and pointer. Custom close exit0 with CEF before-close
and host-closed receipts. Earlier minimized/closed polling and fast-queued press
sampling failures are retained separately. Five bounded window runs covered those
fixes and new resize scope; no Hanja replay, external browsing or scoring change.
GUI-tested source snapshot/artifact hashes are preserved. Final source additionally fixes maximized-edge arrow selection and DPI-scaled resize minima/capture cancellation on DPI change; these branches pass unit tests, Clippy and build but were not replayed in native GUI. Normal96-DPI gesture evidence refers to the preserved source snapshot and DLL.

Next: SDK ABI/lifecycle gates remain planned/not_run; internal URL capability
qualification, external navigation, Chrome rendering parity, complete sandbox,
keyboard resize/Snap, high-DPI/multi-monitor/screen-reader/cross-OS and new-geometry
IME qualification remain open. Read AGENTS.md, rust-migration, browser-ui,
chromium-lab-browser and native-embedding SoTs before continuing. Shared Cargo/
roadmap integration belongs to the root session. No detection scoring changed.
