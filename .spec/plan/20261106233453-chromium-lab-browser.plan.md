---
id: plan:root/20261106233453
title: Rust embedded browser qualification
status: active
created: 2026-11-06T23:34:53Z
updated: 2026-11-06T23:34:53Z
rel:
  depends_on: [sot:root/chromium-lab-browser]
---

# Browser qualification

- [x] Three-agent architecture, observation and security research plus independent critique.
- [x] Preserve explicit Rust-only authored-code requirement; exclude Electron JS shell.
- [x] Clarify desired embedded-tab experience versus unproven OpenAI implementation details.
- [x] Withdraw whole Owl clone/clean-room scope; later scoped browser-embedding REA is delegated to its own project chat.
- [x] Select user-required Meari UI and record native surface/toolchain qualification gaps.
- [ ] Define bounded observation wire schema and native integration safety/MSRV policy.
- [ ] Qualify Rust supervisor external-engine pilot without claiming embedded completion.
- [ ] Qualify actual CEF native Rust UI, sandbox and packaging on supported native tuples.
- [ ] Integrate actual collector/session/ingress and independent receipt/effect evidence.
- [ ] Execute unit/native pilot/Docker product e2e and AT controls.
- [ ] Complete signed engine updates, CVE/SBOM checks and authored Pages documentation.

Browser observation helpers are separate from the user's Node/nginx-like server
adapter expansion. Every new browser implementation/qualification remains pending.
