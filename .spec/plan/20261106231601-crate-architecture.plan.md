---
id: plan:root/20261106231601
title: Crate architecture review follow through
status: active
created: 2026-11-06T23:16:01Z
updated: 2026-11-06T23:16:01Z
rel:
  depends_on: [sot:root/crate-architecture]
---

# Crate review follow-through

- [x] Three read-only agents reviewed Cargo/naming, actual responsibilities and target/release boundaries.
- [x] Captured actual 13-package metadata and distinguished normal/dev dependency edges.
- [x] Canonical findings separate Rust requirements, project conventions and optional core consolidation.
- [x] Recorded retain-current-context recommendation plus concrete shared application/secret/provenance gaps.
- [ ] Consume/zeroize authority startup secrets and consolidate decoded key custody.
- [ ] Add typed validated provenance getter and remove redundant authority JSON parsing.
- [ ] Complete shared session/application state and actual Core/Gate/browser composition.
- [ ] Decide public package/test-harness closure and complete metadata/trust/version readiness.
- [ ] Extend final Go retirement inventory to both demo generator tools.
- [ ] Evaluate a core/SDK consolidation only with a concrete consumer/target reason and complete impact plan.

No naming/directory/package consolidation was implemented by this research.
Existing gates remain fail-closed; no publication flag was changed. Concurrent
key/vault dependency integration is a separate implementation checkpoint.
