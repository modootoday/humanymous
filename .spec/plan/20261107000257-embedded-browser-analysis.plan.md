---
id: plan:root/20261107000257
title: Embedded browser analysis
status: active
created: 2026-11-07T00:02:57Z
updated: 2026-11-07T00:02:57Z
rel:
  depends_on: [sot:root/embedded-browser-analysis]
---

# Embedded browser analysis

## Goal

Explain the shipped browser guest, attach, profile and control boundaries using
bounded static evidence. No Owl clone, full runtime reconstruction or UI implementation.

## Steps

- [x] Claim dedicated sot-plan paths and read coordination/REA instructions.
- [x] Recheck ASAR identity and extract only selected shipped members privately.
- [x] Separate sidebar guest attachment from download-only WebContentsView use.
- [x] Trace route/session ownership, persistent app partition, preload and CDP/native-pipe paths.
- [x] Parse five bounded derived slices with REA and retain full private evidence.
- [x] Distinguish official contract, dated third-party inference, static observation and unknown.
- [x] Record Rust-only/Meari/CEF transferable principles without source reuse.
- [ ] Runtime qualification of native attachment, complete IPC/CDP policy and profile persistence; requires a separately scoped task.
- [ ] Qualify the Rust UI/engine pair; belongs to the browser implementation roadmap.

## Record

Static research complete; runtime qualification is explicitly not_run. Main
integration is owned by the root chat. No branch switch, main merge, push or tag.
The older roadmap's broad REA withdrawal is not silently edited by this bounded
continuation; root should reconcile the scope wording when integrating.

Validation: scoped spec check for the three public research documents passes.
The full spec check additionally reports S3 for raw files under the explicitly
authorized analysis directory (including the inherited owl directory), and P2 for
another session's local node-adapter evidence. These were not repaired outside
this lane. No product code changed, so Go/Rust/e2e execution is not applicable.
