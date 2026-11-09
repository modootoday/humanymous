---
id: sot:root/clean-room-usage-matrix
title: Docker clean-room usage-condition matrix completeness gates
status: accepted
created: 2026-11-08T18:35:00Z
updated: 2026-11-09T23:15:00Z
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

## LIVE measured status board (Cap=0 tip-bind)

Measured 2026-11-09T23:15:00Z on branch `codex/rust`:

| Field | Measured |
| --- | --- |
| tip / matrix `head_sha` | `4c198f30771d937673525dd739cb7f7dcc7910c5` |
| decision | B |
| product / `pass_product_clean_room` | 0 / 0 |
| Cap free | false |
| Cap burn this board | 0 |
| BrowserReady | false |
| AUTH-HANDOFF / AT-HANDOFF | absent / absent |
| required / clean_room_required / product cells | 84 / 47 / 0 |
| product_matrix_open_cells | 16 (need AUTH/AT/BrowserReady or Root structural / Cap free) |
| missing SoT scenario | `red-blue-product` (1) |
| Unlock A / B / C | true / true / false |
| CEF Cap-day durable bind | present (`.spec.local/lab/cef-cap-day-bind.json`) ≠ BrowserReady |
| `HUMANYMOUS_CEF_PILOT_BUNDLE` process env | unset |
| `crates/apps/humanymous-browser` | absent |
| `crates/browser` | present |
| Cap0∥ remaining / Cap1 | none / HOLD |
| goal_complete / `full_product_qualified` | false / false |

Exact next closer (unchanged Decision B): Root AT-HANDOFF
(`humanymous.root-interactive-at-windows-tuple/1`) + AUTH-HANDOFF
(`humanymous.root-n06-owner-join-auth-handoff/1`) → Cap0 apply BrowserReady/N06
→ documented Cap realloc (NEW non-twin feeder, not spent G2/G5/CR/G6/RB/PM twin)
→ Cap free=true → Cap=1 product path toward `pass_product_clean_room`.

Receipts: `.spec.local/root/evidence/20261110080500-cursor-cap0-cap-free-restore/`,
`.spec.local/root/evidence/20261110080900-cursor-sot-code-sync/`.

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

Cap realloc schemas for G2/G5/CR/G6/RB/PM require live Cap topology TRANSFER first.
Cursor must not invent Cap free from Cap-ready structural feeders alone.
Measured spent Cap=1 component / feeder burns (Cap free remains false):

| Spend | Historical tip note | Status |
| --- | --- | --- |
| G2+G5+CR | tip `10de63c1` component topology | spent_forbidden twin |
| G6 | later Cap1 component | spent_forbidden twin |
| RB | red-blue Cap realloc | spent_forbidden twin |
| PM | `--clean-room-product-matrix --require-full` EXIT2 feeder | spent_forbidden twin |

NEW non-twin GRANT→TRANSFER matching LIVE preimages is required before Cap free
may honestly become true. Unlock C remains blocked on 6 digest mismatches.

## Tip bind

Matrix `head_sha` must bind the settled integration tip before completeness
claims. Missing SoT-required scenarios fail closed. Historical tip bind
`10de63c1`: G2+G5+CR cells may hold `pass_component_desktop_cap_topology` only —
never `pass_product_clean_room`. LIVE tip bind is the status board tip above.
CR Cap=1 binds G2+G5 Cap=1 holds and Cap=0 AUTH/AT refuse; AUTH/AT handoffs
remain Root and are never forged. tip_bind.ok=true ≠ product green.
