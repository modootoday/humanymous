---
id: plan:root/20261109041000-cap0-product-gate-enumeration-hard-stop
title: Cap=0 tip-bound product-gate enumeration + hard-stop after G2/G5/CR/G6 Cap spent
status: active
created: 2026-11-09T03:45:00Z
updated: 2026-11-09T03:45:00Z
rel:
  depends_on:
    - plan:root/20261108233000-post-unlock-c-matrix-honesty
    - plan:root/20261109033000-matrix-completeness
    - sot:root/program-roadmap
    - sot:root/structural-development
  governs: []
---

# Cap=0 tip-bound product-gate enumeration + hard-stop

Brief: After G2/G5/CR/G6 Cap=1 component holds (tip `809e2290`), Cap free=**false**,
AUTH/AT absent, and G7/G8/G9 remain `root_only` / not Cap-ready. Cursor lands
Cap=0 tip-bound `matrix-completeness` + Cap=0 honesty CLIs + exact remaining
product-closer enumeration, then **hard_stop_root_ask**. `product=0`.
`pass_product_clean_room=0`. Never invent Cap free / BrowserReady / AUTH/AT /
`pass_product_clean_room`. No G2/G5/CR/G6 Cap twin spam.

## 1. Settled Cap wave (component holds ≠ product)

| Cap cell | Cap=1 | counts_as | product |
| --- | --- | --- | --- |
| G2 `--g2-collector-product-binding` | EXIT0 | `pass_component_desktop_cap_topology` | 0 |
| G5 `--g5-witness-transport` | EXIT0 | `pass_component_desktop_cap_topology` | 0 |
| CR `--clean-room-product` | EXIT0 | `pass_component_desktop_cap_topology` | 0 |
| G6 `--g6-origin-audit-binding` | EXIT0 | `pass_component_desktop_cap_topology` | 0 |

## 2. Cap=0 tip-bound honesty @ `809e2290`

| Selector | Exit | Note |
| --- | --- | --- |
| `matrix-completeness --expect-head 809e2290…` | 0 | 84 required cells; clean_room_required=47; tip_bind ok |
| `--n06-owner-auth-handoff-honesty` | 0 | AUTH absent; Cap free=false |
| `--browser-ready-interactive-at-honesty` | 0 | AT absent; BrowserReady=false |
| `--g2-collector-product-binding-readiness` | 0 | G2 Cap topology live; product_binding_ready=false |
| `--g5-witness-transport-honesty` | 0 | authenticated_witness_transport=false |

Private evidence (not Git):
`.spec.local/root/evidence/20261109041000-cursor-cap0-product-enumeration/`
`.spec.local/root/evidence/20261109041100-cursor-hard-stop-tip-bound/`

## 3. Exact remaining product closers

1. **AUTH-HANDOFF** — `humanymous.root-n06-owner-join-auth-handoff/1`
2. **AT-HANDOFF** — `humanymous.root-interactive-at-windows-tuple/1`
3. **G1** full Gate TLS/H2/capture — root_only
4. **G2** session-bound real browser collector — Cap topology spent; product open
5. **G3** Pass recovery — root_only
6. **G4** human/keyboard/assistive controls — needs AT-HANDOFF
7. **G5** authenticated witness transport — Cap feeder spent; product open
8. **G6** origin/audit product binding — Cap topology spent; product open
9. **G7** signed remote target attestation — root_only; no Cap-ready feeder
10. **G8** native OS qualification — root_only; no Cap-ready feeder
11. **G9** provider deployment — root_only; no Cap-ready feeder

Priority matrix cells remain `required_unavailable` / `fail` / `blocked*` /
`unrun` (see private `PRODUCT-CLOSERS.md`). Clean-room required open count: 47;
priority non-product cells enumerated: 13.

## 4. Decision

`hard_stop_root_ask`. Cap free before=after=**false**. Cap burn=0 this turn.
`blocked_on_root=true`.

**Next closer:** Root places AUTH-HANDOFF or AT-HANDOFF; Cursor Cap=0 measured
apply only when present. Never Cap spam / invent `pass_product_clean_room` /
BrowserReady.

## 5. Tracked delta this tip

- `test/lab/runner/src/clean_room_matrix.rs` — completeness `blocked_on_root` /
  `next` honesty after spent G2/G5/CR/G6 Cap twins (prefer AUTH/AT; no Cap spam)
- this plan
