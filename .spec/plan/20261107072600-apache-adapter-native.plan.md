---
id: plan:root/20261107072600
title: Apache native module implementation and hosted qualification
status: active
created: 2026-11-07T07:26:00Z
updated: 2026-11-07T07:26:00Z
rel:
  governs: [crates/apache-adapter/**, adapters/apache/**]
  depends_on: [sot:root/apache-adapter]
---

# Apache native module implementation

Goal active; lane adapter-apache session rust-apache-goal claimed after root
071ccef authorization. Previous turn researched native hook ordering and shared
contracts; no implementation existed then. This checkpoint authors safe Rust
child/request lifecycle, meaningful pure tests, integration and pilot contract.

- [x] Read coordinate-sessions/design-sot/implement-sot-slice and current canon.
- [x] Read current authority/framework shared APIs without editing their owners.
- [x] Adversarial bypass/fork/pool/body findings reflected in owning SoT.
- [x] Safe child generation/MPM admission, complete bounded body and cancellation.
- [ ] Root workspace registration and local Cargo slot; pure units/fmt/Clippy.
- [ ] Apache integrity/authority/framework profile owned by corresponding owners.
- [ ] Single-consumption authority response and exact final request handoff.
- [ ] Approved minimal unsafe native ABI, generated binding provenance.
- [ ] Actual native module hooks/input replay/pool cleanup/child initialization.
- [ ] Source-bound free hosted Docker MPM and outage/cancellation/reload pilots.
- [ ] Spec check, owned attributed commit and integration evidence.

Current tests are NOT_RUN. No actual httpd, Docker, IPC or origin pilot ran locally.
No dependency lock, workspace, workflow or public docs changes are owned here.
Native ABI is blocked pending explicit exception approval; independent safe Rust
work continues. No foreign executable source/string/template workaround.
