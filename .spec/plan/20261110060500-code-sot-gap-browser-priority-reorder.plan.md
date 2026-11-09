---
id: plan:root/20261110060500-code-sot-gap-browser-priority-reorder
title: Cap=0 CODE↔SoT gap survey — BrowserReady first-class priority reorder
status: active
created: 2026-11-09T21:05:00Z
updated: 2026-11-09T21:25:00Z
rel:
  depends_on:
    - plan:root/20261110044500-production-status-priority-reorder
    - plan:root/20261110043600-production-status-live-cap0
    - plan:root/20261108230100-bottom-up-inner-outer-parallelization
    - sot:root/program-roadmap
    - sot:root/clean-room-usage-matrix
    - sot:root/structural-development
    - sot:root/browser-ui
    - sot:root/client-collector
    - sot:root/chromium-lab-browser
  governs: []
---

# Cap=0 CODE↔SoT gap survey + Browser-visible priority reorder

Brief: Full-survey gaps between CURRENT CODE and CANON at tip `6373c436`.
Explicit pain: **humanymous browser had no visible priority** in the prior
reorder (AT-HANDOFF token only; G4 buried under lumped G1/G3/G4). This plan
puts **Browser / CEF / AT / BrowserReady** on the critical path as a
first-class unit. Cap=0 only this lane. Detection freeze. Never invent
`pass_product_clean_room` / Cap free / AUTH / AT / `BrowserReady`.

Private evidence:
`.spec.local/root/evidence/20261110060500-cursor-code-sot-gap-survey/`

## 1. LIVE Cap=0 probe (this turn · tip-bind)

| Field | Measured |
| --- | --- |
| tip | `726f59c889ad260ac9ea0433099306df53a9cb3d` (Browser PRIORITY#1 Cap=0 track base) |
| branch | `codex/rust` |
| product / pass_product_clean_room | **0** / **0** |
| Cap free | **false** |
| AUTH-HANDOFF | **absent** |
| AT-HANDOFF | **absent** |
| BrowserReady / n06_owner_join | **false** / **false** |
| matrix-completeness | EXIT0; required=84; clean_room_required=47; product=0; **`browser_ready_stack.rank=1` first-class** |
| tip_bind.ok | **true** (matrix `head_sha` tip-bound @ `726f59c8`) |
| missing SoT scenario | `red-blue-product` (1; Cap free=false blocker) |
| Unlock A / B / C | true / true / false (`unlock_any=true`) |
| `handoff-apply --kind at` | EXIT0; AT absent; refuse_closed; BrowserReady=false |
| `cef-bundle-probe` | EXIT0; exact missing `env:HUMANYMOUS_CEF_PILOT_BUNDLE`; private host **found** |
| `--browser-ready-interactive-at-honesty` | EXIT0; BrowserReady=false; AT absent |
| `--n06-owner-auth-handoff-honesty` | EXIT0; n06_owner_join=false; AUTH absent |
| `--n06-owner-join-readiness` | EXIT0; join open; BrowserReady=false |
| `unlock-probe` | EXIT0; Cap free=false; g4 bound; CEF env absent |
| Unlock C Cap=0 diagnose | blocked_claim; mismatches `snapshot.rs` + `authority.rs` |
| Cap burn this turn | **0** |
| full e2e EXIT0 class | `component_pass_desktop_oracle` only (not product) |

### Cap=0 Browser PRIORITY#1 landings (shorten AT→BrowserReady)

| Deliverable | Status |
| --- | --- |
| AT-HANDOFF apply helper / ROOT-ONE-STEP twin | **ready** — `handoff-apply --kind at` + `cap-ready/at-handoff-apply/` |
| CEF bundle presence probe | **ready** — env absent exact missing; private host complete |
| `crates/apps/humanymous-browser` path-transfer | Cap-ready pack only (`applied=false`); live path still absent |
| BrowserReady first-class matrix tip-bind | **landed** — `browser_ready_stack` rank 1 cell blocker |

## 2. Pain diagnosis (why browser was invisible)

| Prior model | Failure |
| --- | --- |
| Rank 2 = "AT-HANDOFF" only | Token wake, not the **browser stack** (crates/browser, CEF pilot, package consolidation, interactive AT tuple) |
| G4 buried in rank-9 "G1/G3/G4" | BrowserReady critical path diluted among TLS/Pass structural gaps |
| Unlock A=true | Read as "browser unlocked"; honesty still BrowserReady=false, CEF bundle missing, `crates/apps/humanymous-browser` absent |
| Bottom-up L4 Meari/CEF = P1 "대기 Unlock A" | Stale after Unlock A Cap EXIT0; never re-ranked to post-A BrowserReady close |

## 3. Decision rules (delta over 20261110044500)

R1. **BrowserReady stack is first-class.** Rank and canvas must name
`crates/browser/**` + CEF pilot + AT-HANDOFF + interactive AT Windows tuple
explicitly — not only a Root handoff filename.

R2–R5. Unchanged from prior reorder: critical path first; Cap=0 parallel OK;
Cap=1 serial after realloc once; never forge AUTH/AT; deprioritize Cap twin spam.

## 4. Gap inventory (CODE vs SoT) — top 18

| # | Area | SoT requirement | Code status | Evidence | Blocker class |
| ---: | --- | --- | --- | --- | --- |
| 1 | **BrowserReady / interactive AT** | `browser-ui` + clean-room `root_at_handoff`; measured AT Windows tuple | Cap=0 honesty refuse; Unlock A g4 component executed ≠ ready | AT-HANDOFF absent; honesty EXIT0 BrowserReady=false | **ROOT** |
| 2 | **CEF pilot bundle / G4 Cap** | CEF GUI/IME/AT on exact Windows tuple; Unlock A pack | Cap CLI `--g4-cef-at` **bound**; `HUMANYMOUS_CEF_PILOT_BUNDLE` **absent** | unlock-probe; path surfaces `crates/browser/**`, `scripts/ci/cef-pilot/**` | **ROOT** + Cap1 serial after realloc |
| 3 | **humanymous-browser package** | structural-dev: consolidate 3→1 under `crates/apps/humanymous-browser` | Live `crates/browser` (+ cef-bootstrap/desktop); **`crates/apps/humanymous-browser` missing** | SoT package census vs tree | **ROOT** / integration |
| 4 | **Meari+CEF UI qualification** | real navigation/keyboard/IME/AT/DPI/sandbox | Component compile/pilot history; BrowserReady=false; external browse disabled | NATIVE-QUALIFICATION.md; program-roadmap Meari row | **ROOT** |
| 5 | **AUTH-HANDOFF / N06 owner-join** | clean-room `root_auth_handoff`; N06 hosted join | Cap=0 honesty + readiness landed; join=false | AUTH-HANDOFF absent; honesty EXIT0 | **ROOT** |
| 6 | **Cap realloc / Cap free** | live Cap topology TRANSFER before Cap=1 | Cap free=false after G2/G5/CR/G6 spends | wake grants present; realloc unauthorized without live topology | **ROOT** + Cap0 probe |
| 7 | **Clean-room product cells** | only `pass_product_clean_room` counts | 47 clean-room-required open; product=0 | matrix-completeness tip-bound | Cap1 serial + ROOT |
| 8 | **red-blue product scenario** | SoT-required matrix scenario | Missing from matrix combinations (count=1) | completeness `missing_sot_scenarios` | Cap free_false / Cap1 |
| 9 | **Collector WASM / session-bound (G2)** | client-collector: session-bound collector + actual browser binding | WASM source live; Cap1 twin spent component ≠ product | G2 Cap topology hold; product cell open | Cap1 after AUTH/AT · DEPRI twin |
| 10 | **H2 / W2 product-binding** | gateway H2 + N06; Unlock B | Cap CLI bound; AUTH/N06 still open | Unlock B=true ≠ product | ROOT + Cap1 alternate |
| 11 | **Witness transport (G5)** | authenticated witness | Cap1 twin spent; not product | measured_component_hold | DEPRI twin / ROOT product |
| 12 | **Origin/audit binding (G6)** | L5 origin·audit product | Cap1 twin spent; not product | measured_component_hold | DEPRI twin / ROOT product |
| 13 | **TLS/H2 capture (G1)** | network-capture / M2 | required_unavailable structural | root_structural_g1_g9 | **ROOT** |
| 14 | **Pass recovery (G3)** | SoT-36; detection freeze | not Cursor-inventable | Root-only | **ROOT** |
| 15 | **Authority Unlock C** | Cap CLI authority-node-ipc | blocked_claim; live digests ≠ candidate | unlock-probe C=false | Cap0 diagnose · ROOT transfer |
| 16 | **Audit / custody** | audit SoT; dual-control erasure | component/custody pilots ≠ product auth | custody witness hold | Cap0 parallel / ROOT product |
| 17 | **Runtime / gate** | runtime-application; fail-closed | component Docker/process; not full Gate/browser/Pass | program-roadmap Runtime row | Cap0 parallel |
| 18 | **G7/G8/G9 outer** | attestation / native OS / provider deploy | Cap-ready invention forbidden pre-M3 | deprioritized | **DEPRI** / ROOT after M3 |

## 5. New priority order (top 10) — Browser ranked visibly

| Rank | Unit | Class |
| ---: | --- | --- |
| **1** | **BrowserReady stack** — AT-HANDOFF + CEF bundle + `crates/browser/**` / `scripts/ci/cef-pilot/**` + interactive AT Windows tuple (+ humanymous-browser consolidation) | ROOT / Cap0 wake∥ |
| **2** | **AUTH-HANDOFF** (N06 owner join) | ROOT / Cap0 wake∥ |
| **3** | Cap realloc (live topology TRANSFER) | ROOT + Cap0 probe |
| **4** | Unlock C Cap=0 digest mismatch diagnosis | P0-cap0 |
| **5** | Cap=1 once after realloc — prefer **clean-room-product** (AUTH/AT present) else Unlock-B **w2-h2** | P0-serial-cap1 |
| **6** | Clean-room required cells → `pass_product_clean_room` (47 open) | Cap1 + matrix |
| **7** | red-blue-validate Docker require-full (+ land missing `red-blue-product` scenario) | Cap1 serial |
| **8** | G1 TLS/H2 capture + G3 Pass recovery product closes | ROOT |
| **9** | G2/G5/G6 **product** re-qualify (not Cap topology twin) | ROOT after #1–3 |
| **10** | M5 → M6 shared main / main-only | serial final |

Matrix tip-bind remains Cap=0 hygiene (currently **ok** @ `6373c436`); not a top-10 blocker.

## 6. Parallel Cap=0 tracks vs serial Cap=1

### Cap=0 parallel (this lane / path-disjoint)

| Track | Work | Exit |
| --- | --- | --- |
| **T1 BrowserReady wake** | Measured AT/CEF/bundle presence + honesty refuse; keep Browser stack visible | BrowserReady=false receipt EXIT0; never forge |
| **T2 AUTH/N06 wake** | Measured AUTH presence + N06 honesty | n06_owner_join=false receipt EXIT0 |
| **T3 Unlock C + Cap realloc probe** | Digest mismatch note; Cap free probe; no invent Cap free | C blocked_claim honest; Cap free=false |
| **T4 Gap plan + LIVE canvas** | This plan; canvas shows Browser+AUTH/AT+Cap realloc+clean-room first-class | Plan landed; canvas updated |

### Cap=1 serial (HOLD)

Hold until Cap realloc → Cap free=true. Then **one** burn:

1. Preferred: `--clean-room-product` with AUTH/AT present + matching Cap-ready pack
2. Alternate if AUTH/AT still absent: Unlock-B `--w2-h2-product-binding` once
3. Forbidden: G2/G5/CR/G6 Cap twin spam; a11y/`--require-full` Cap loops; Cap=1 while Cap free=false

## 7. Explicit non-claims

- Unlock A∨B true ≠ Cap free ≠ BrowserReady ≠ product green
- Cap1 CR/G2/G5/G6 EXIT0 = component topology only
- tip_bind.ok=true ≠ pass_product_clean_room
- full e2e EXIT0 desktop oracle ≠ product
- No forge AUTH/AT; no invent Cap free / BrowserReady / pass_product_clean_room

## 8. Wiring

| Artifact | Path |
| --- | --- |
| This plan | `.spec/plan/20261110060500-code-sot-gap-browser-priority-reorder.plan.md` |
| Cap=0 evidence | `.spec.local/root/evidence/20261110060500-cursor-code-sot-gap-survey/` |
| Supersedes priority table of | `plan:root/20261110044500-production-status-priority-reorder` |
| Matrix | `.spec.local/root/evidence/20261108164500-cursor-clean-room-matrix/` |
| LIVE canvas | `canvases/humanymous-live-roadmap.canvas.tsx` (Cursor projects) |
