# PRODUCT-BLOCKERS — Cap=0 tip-bound Decision B hard closer

**tip:** `ad8b68a21d22e83477b00eba08c36ddcc6f78e99` (`codex/rust`)  
**observed_utc:** `2026-11-09T22:05:45Z`  
**decision:** **B** — Cap burn=**0**  
**product / pass_product_clean_room / BrowserReady:** **0** / **0** / **false**  
**Cap free:** **false** (spent false→true→false on PM Cap1)  
**AUTH-HANDOFF / AT-HANDOFF:** **absent** / **absent** · n06_owner_join=**false**

Source Cap1 EXIT2 receipt:
`.spec.local/root/evidence/20261110065100-cursor-clean-room-product-matrix-cap-realloc/CAP1-RECEIPT.json`
+ `cap1-bundle-r2/clean-room-product-matrix-require-full-honesty.json`
+ `cap1-bundle-r2/lab-report.json`

## Decision (honest)

**A rejected.** No Cap=0 or Cap-ready→realloc→Cap1 path can close a
`pass_product_clean_room` cell without AUTH/AT/BrowserReady (or Root
structural G1–G9). Cap1 `--clean-room-product-matrix --require-full` already
proved feeder component only: EXIT **2**, **6/0**, product=**0**. Cap free
spent. Cap twin reburn of spent G2/G5/CR/G6/RB/PM feeders is forbidden.

**B applied.** Cap=0 tip-bound blockers + FULL priority rank update; Cap
burn=0; hard closer until Root AT/AUTH land.

## Exact signals (EXIT2 auth_at_refuse)

| Signal | Measured |
| --- | --- |
| at_present | false |
| auth_present | false |
| browser_ready | false |
| n06_owner_join | false |
| status | absent_ROOT-ONE-STEP_refuse |
| clean_room_required_non_product | 47 |
| product / pass_product_clean_room | 0 / 0 |

## Exact missing product cells (47; all counts_as_product=false)

Grouped from Cap1 require-full honesty `missing_product_cells`:

| Scenario | n | Status classes |
| --- | ---: | --- |
| topology-fixture | 5 | pass_component_desktop |
| rust-controls | 5 | pass_component_desktop |
| authority-node-ipc | 4 | pass_component_desktop |
| w2-h2-product-binding | 1 | pass_component_desktop |
| w2-h2-hosted-venue | 1 | pass_component_hosted |
| go-product | 6 | fixture_ready / pass_component_desktop / fail / pass_component_hosted / blocked_hosted_linux |
| browser-product | 5 | pass_component_desktop + hosted fail |
| custody | 2 | pass_component_desktop / pass_component_hosted |
| runtime | 2 | pass_component_desktop |
| witness | 2 | pass_component_desktop |
| m3-full-product | 5 | required_unavailable + hosted fail |
| oracle-detector-bots | 5 | pass_component_desktop |
| g2-collector-product-binding | 1 | pass_component_desktop_cap_topology |
| g5-witness-transport | 1 | pass_component_desktop_cap_topology |
| clean-room-product | 1 | pass_component_desktop_cap_topology |
| g6-origin-audit-binding | 1 | pass_component_desktop_cap_topology |

### product_matrix_open_cells (16; still not product)

1. `desktop-linux|none|browser-product|source_bound_image_build` — pass_component_desktop → needs **BrowserReady/AT**
2. `desktop-linux|none|browser-product|compose_lifecycle` — pass_component_desktop → needs **BrowserReady/AT**
3. `desktop-linux|none|browser-product|lab_runner_require_full` — pass_component_desktop → needs **BrowserReady/AT**
4. `hosted-linux-github|none|browser-product|lab_runner_require_full` — fail → Root structural + **BrowserReady**
5. `desktop-linux|none|browser-product|a11y_non_motor_checks` — pass_component_desktop → needs **BrowserReady/AT**
6. `desktop-linux|none|m3-full-product|lab_runner_require_full` — required_unavailable → Root G1–G9 + **BrowserReady**
7. `hosted-linux-github|none|m3-full-product|lab_runner_require_full` — fail → Root structural
8. `desktop-linux|none|m3-full-product|a11y_non_motor_checks` — required_unavailable → Root G1–G9
9. `desktop-linux|none|m3-full-product|owned_cleanup` — required_unavailable → Root G1–G9
10. `desktop-linux|none|m3-full-product|source_image_topology_binding` — required_unavailable → Root G1–G9
11. `desktop-linux|none|oracle-detector-bots|red_blue_up` — pass_component_desktop → Cap1 after product cells; Cap free=false
12. `desktop-linux|none|oracle-detector-bots|red_blue_attack` — same
13. `desktop-linux|none|oracle-detector-bots|red_blue_e2e_assert` — same
14. `desktop-linux|none|oracle-detector-bots|red_blue_gate_e2e` — same
15. `desktop-linux|none|oracle-detector-bots|red_blue_swarm` — same
16. `desktop-linux|none|clean-room-product|lab_runner_require_full` — pass_component_desktop_cap_topology → spent Cap twin; product needs AUTH/AT + Cap realloc

### lab-report required_unavailable (product signals)

- full Gate TLS/H2/capture
- session-bound real browser collector (**AUTH/AT/BrowserReady**)
- Pass recovery
- human/keyboard/assistive controls (**AT/BrowserReady**)
- runtime binary authenticated witness transport
- cross-namespace origin/audit effect product binding
- signed remote target attestation
- native OS qualification
- provider deployment

## Why Cap0 / Cap twin cannot close product now

- Cap free=**false** after PM Cap1 burn; no Cap cycle without documented NEW Cap-ready realloc (not spent twin).
- Cap1 without AUTH/AT already measured: feeder EXIT2, product stays 0.
- Cap0∥ remaining: **none** (custody/runtime/topology Cap0 ≠ product alone).
- Forbidden Cap twin spam: spent G2 / G5 / CR / G6 / RB / PM feeders.

## Exact next closer

Root **AT-HANDOFF** (`humanymous.root-interactive-at-windows-tuple/1`) + Root
**AUTH-HANDOFF** (`humanymous.root-n06-owner-join-auth-handoff/1`) → Cap=0 apply
BrowserReady / N06 → documented Cap realloc (NEW structural feeder, not spent
twin) → Cap free=true → Cap=1 product path toward `pass_product_clean_room`.

Never invent pass_product_clean_room / BrowserReady / AUTH / AT / Cap free.
