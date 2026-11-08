---
id: sot:root/clean-room-usage-matrix
title: Docker clean-room usage-condition matrix completeness gates
status: accepted
created: 2026-11-08T18:35:00Z
updated: 2026-11-08T19:10:00Z
verify: manual
rel:
  depends_on: [sot:root/container-test-environments, sot:root/delivery, sot:root/program-roadmap]
  governs: [.spec.local/root/evidence/*clean-room-matrix*/**, test/lab/runner/src/clean_room_matrix.rs]
---

# Clean-room usage-condition matrix completeness

Path-released to cursor-integration after Cap-ready GRANT + matching
candidateSha256 create-only place. Cap=0 vocabulary refresh from matrix
completeness. Detection freeze. product completion still requires Docker
clean-room `pass_product_clean_room` cells — this SoT does not invent product
green.

## Authority

- Usage-condition surface: deployments/lab/ENVIRONMENTS.md
- Cap=0 completeness gate: humanymous-lab-runner matrix-completeness
- Inventory matrix: .spec.local/root/evidence/20261108164500-cursor-clean-room-matrix/
- Only status that counts as product: pass_product_clean_room

## Honest blockers (fail-closed)

| Blocker | Meaning |
| --- | --- |
| root_grant_g2 | G2 GRANT.json + topology TRANSFER required (spent Cap=1 component only) |
| root_grant_g5 | G5 GRANT.json + topology TRANSFER required (spent Cap=1 component only) |
| root_auth_handoff | AUTH-HANDOFF for N06 owner join |
| root_at_handoff | AT-HANDOFF for BrowserReady / interactive AT |
| cap_free_false | Cap free conserved false until Cap realloc after live topology |
| root_structural_g1_g9 | required_unavailable until Root structural delta |
| measured_component_hold | component/fixture measured; never promote to product |
| blocked_hosted_linux | hosted Linux allocation still required |
| fail_needs_structural_delta | failed gate needs Root-reviewed delta |

## Cap realloc

Cap realloc schemas for G2/G5/CR require live Cap topology TRANSFER first.
Cursor must not invent Cap free from Cap-ready structural feeders alone.
G2+G5+CR Cap=1 component burns already spent at tip `10de63c1`; Cap free=false.

## Tip bind

Matrix `head_sha` must bind the settled integration tip before completeness
claims. Missing SoT-required scenarios fail closed. Tip bind `10de63c1`:
G2+G5+CR cells may hold `pass_component_desktop_cap_topology` only — never
`pass_product_clean_room`. CR Cap=1 binds G2+G5 Cap=1 holds and Cap=0 AUTH/AT
refuse; AUTH/AT handoffs remain Root and are never forged.
