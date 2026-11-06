---
id: plan:root/20261106224156
title: Canonical container test environment rollout
status: active
created: 2026-11-06T22:41:56Z
updated: 2026-11-06T22:41:56Z
rel:
  depends_on: [sot:root/container-test-environments]
---

# Container environment research and rollout

- [x] Three subagents researched managed Containers, Tunnel/Access/Browser Run and repo Compose gaps.
- [x] Official sources and actual repository boundaries synthesized in canonical SoT.
- [x] Separate engine/mock/Rust product/provider-local/provider-remote evidence types.
- [x] Explicit no-public-admin, no-origin-bypass, no-egress and provider provenance requirements.
- [ ] Complete actual Rust runtime, independent witness/state and browser paths.
- [ ] Implement canonical contextual Compose with Rust image digests and isolated networks.
- [ ] Run full Docker lab with real adapters and human/a11y controls.
- [ ] Implement cf-compatible provider-local overlay without copying Compose security assumptions.
- [ ] Confirm dedicated account/domain/credentials/budget before remote resource creation.
- [ ] Run bounded owned remote compatibility tests and verify TTL cleanup.

Cloudflare deployments and paid provisioning were not performed by this research.
The existing Go lab is reference evidence only; its publicly published admin port
must not be copied into the final Rust deployment design.
