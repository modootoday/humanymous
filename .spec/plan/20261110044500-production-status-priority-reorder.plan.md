---
id: plan:root/20261110044500-production-status-priority-reorder
title: Cap=0 priority reorder — shortest path to pass_product_clean_room then red-blue
status: active
created: 2026-11-09T19:45:00Z
updated: 2026-11-09T19:45:00Z
rel:
  depends_on:
    - plan:root/20261110043600-production-status-live-cap0
    - plan:root/20261108230100-bottom-up-inner-outer-parallelization
    - plan:root/20261109041000-cap0-product-gate-enumeration-hard-stop
    - sot:root/program-roadmap
    - sot:root/clean-room-usage-matrix
    - sot:root/structural-development
  governs: []
---

# Cap=0 priority reorder (tip `77bcbd46`)

Brief: Renew full-roadmap goal through verification + clean-room Docker pass.
**Cap=0 only** this lane (sibling may hold Cap/Docker). Re-prioritize remaining
units by shortest critical path to `pass_product_clean_room`, then red-blue.
Detection freeze. Never invent `pass_product_clean_room` / Cap free / AUTH / AT /
`BrowserReady`. Explicitly deprioritize component Cap twin spam and
documentation-only busywork.

Private evidence:
`.spec.local/root/evidence/20261110044500-cursor-priority-reorder/`

## 1. LIVE Cap=0 probe (this turn)

| Field | Measured |
| --- | --- |
| tip | `77bcbd46c8427116bd901401293fb58a29d14d07` |
| branch | `codex/rust` |
| product / pass_product_clean_room | **0** / **0** |
| Cap free | **false** |
| AUTH-HANDOFF | **absent** |
| AT-HANDOFF | **absent** |
| BrowserReady / n06_owner_join | **false** / **false** |
| matrix-completeness | EXIT0; required=84; clean_room_required=47; product=0 |
| tip_bind.ok (matrix head vs LIVE) | **false** until matrix `head_sha` rebound |
| Unlock A / B / C | true / true / false (`unlock_any=true`) |
| Cap burn this turn | **0** |
| spent Cap1 twins | G2 / G5 / CR / G6 (component topology only) |

## 2. Decision rules (reorder)

R1. **Critical path first.** Anything not on AUTH/AT → Cap realloc → Unlock-matched
product Cap=1 → clean-room product cells → red-blue is deferred unless Cap=0
and path-disjoint.

R2. **Cap=0 parallel OK** when paths disjoint and no Cap/Docker burn.

R3. **Cap=1 serial** after Cap realloc only; one burn; matching Cap-ready pack;
never G2/G5/CR/G6 twin spam.

R4. **Root-blocked AUTH/AT** never forged; Cap=0 wake is measured presence +
honesty refuse only.

R5. **Deprioritize:** component Cap twin reburns; adapter Desktop Cap loops;
a11y/`--require-full` Cap spam; docs-only busywork that does not shorten the
product critical path; G7/G8/G9 Cap-ready invention.

## 3. Remaining units (full roadmap → clean-room → red-blue)

Class: `P0-cap0` = parallel Cap=0 · `P0-serial-cap1` = serial Cap=1/Docker ·
`ROOT` = Root-blocked AUTH/AT/structural · `DEPRI` = deprioritized.

| # | Unit | Class | Parallel? | Why this order |
| ---: | --- | --- | --- | --- |
| 1 | **AUTH-HANDOFF** (`humanymous.root-n06-owner-join-auth-handoff/1`) | ROOT | Cap=0 wake∥ | Hard-stop; unlocks N06 owner-join product path |
| 2 | **AT-HANDOFF** (`humanymous.root-interactive-at-windows-tuple/1`) | ROOT | Cap=0 wake∥ #1 | Hard-stop; BrowserReady / interactive AT |
| 3 | **Cap realloc** via live Cap topology TRANSFER (not invent Cap free) | ROOT + Cap0 probe | Cap=0 probe∥ | Cap free=false after G2/G5/CR/G6; free≠closable |
| 4 | **Matrix tip-bind** `head_sha` → LIVE tip (honesty only) | P0-cap0 | Yes | Completeness tip_bind.ok currently false @ 45004c72 vs 77bcbd46 |
| 5 | **Cap=0 Unlock C digest mismatch diagnosis** (no Cap burn) | P0-cap0 | Yes | Unlock C still false; live digests ≠ candidate |
| 6 | **Cap=1 once after realloc** — prefer **clean-room-product** (AUTH/AT present) or Unlock-B **w2-h2-product-binding** if AUTH still absent but topology live | P0-serial-cap1 | **No** | Shortest Cap burn toward product cells; **not** G2/G5/CR/G6 twins |
| 7 | Clean-room required cells → `pass_product_clean_room` (47 open) | P0-serial-cap1 + ROOT | After #6 | Sole product authority |
| 8 | **red-blue-validate** Docker require-full (oracle phases then Rust M3) | P0-serial-cap1 | After #7 | Cap free / allocation required; host≠completion |
| 9 | G1 TLS/H2 capture product close | ROOT | Cap0 inventory∥ | Structural `required_unavailable` |
| 10 | G3 Pass recovery (SoT-36) | ROOT | Cap0 prep∥ | Detection freeze; no Cursor invent |
| 11 | G4 interactive AT / CEF product (post AT-HANDOFF) | ROOT→Cap1 | After #2 | Unlock A bound; CEF bundle still missing |
| 12 | G2/G5/G6 **product** re-qualify (not Cap topology twin) | ROOT after AUTH/AT | After #1–3 | Component holds ≠ product |
| 13 | CR-GO-HOSTED-8 Specfold (orthogonal hosted) | ROOT allocate | ∥ Desktop Cap | `blocked_hosted_linux`; no GITHUB spoof |
| 14 | M1–M4 path-disjoint component/docs progress | P0-cap0 | Yes (owners) | Parallel; does not close M3 alone |
| 15 | M5 → M6 shared main / main-only | serial final | After M1–M4 | Not started for product close |
| 16 | G7 remote attestation · G8 native OS · G9 provider deploy | DEPRI / ROOT | After M3 | Post-M3 outer; publication unauthorized |
| 17 | G2/G5/CR/G6 Cap1 **component twin reburns** | DEPRI | Never | Spent; product still 0 |
| 18 | Documentation-only / CONSOLIDATED-ROOT-ASK spam | DEPRI | Never | Does not shorten critical path |

## 4. Top 10 ordered (shortest critical path)

| Rank | Unit | Class |
| ---: | --- | --- |
| 1 | Root AUTH-HANDOFF (or Cap=0 measured wake refuse) | ROOT / P0-cap0 |
| 2 | Root AT-HANDOFF (or Cap=0 measured wake refuse) | ROOT / P0-cap0 |
| 3 | Cap realloc (live topology TRANSFER) | ROOT + Cap0 probe |
| 4 | Matrix tip-bind to LIVE tip | P0-cap0 |
| 5 | Unlock C Cap=0 digest mismatch diagnosis | P0-cap0 |
| 6 | Cap=1 serial: clean-room-product (preferred) / w2-h2 if pack-matched | P0-serial-cap1 |
| 7 | Promote/measure clean-room cells → `pass_product_clean_room` | Cap1 + matrix |
| 8 | red-blue Docker require-full | Cap1 serial |
| 9 | G1 / G3 / G4 product structural closes | ROOT |
| 10 | M5→M6 after M1–M4 | serial final |

## 5. Next 3 Cap=0 parallel tracks

| Track | Owner lane | Work | Exit |
| --- | --- | --- | --- |
| **T1 AUTH/AT wake** | cursor-integration | Measured presence + honesty refuse; write WAKE; never forge | AUTH/AT present=false receipt EXIT0 |
| **T2 Priority + canvas** | cursor-integration | This plan + LIVE roadmap canvas reorder | Plan landed; canvas shows lanes |
| **T3 Tip-bind + Unlock C diagnose** | cursor-integration | Matrix `head_sha`→`77bcbd46`; Unlock C digest mismatch Cap=0 note | tip_bind.ok; C still blocked_claim honest |

## 6. Next 1 Cap=1 serial burn (recommendation)

**Hold until Cap realloc makes Cap free=true.** Then burn **once**:

1. **Preferred:** `--clean-room-product` with AUTH/AT present and matching Cap-ready pack (counts only if cells become `pass_product_clean_room`).
2. **If AUTH/AT still absent after realloc:** Unlock-B `--w2-h2-product-binding` once (component≠product honesty) — still not twin G2/G5/CR/G6.
3. **Forbidden now:** G2/G5/CR/G6 Cap twin spam; a11y/`--require-full` Cap loops; Cap=1 while Cap free=false.

Sibling Cap1 agents must not collide; this lane stays Cap=0 until realloc is observed.

## 7. Explicit non-claims

- Unlock A∨B true ≠ Cap free true ≠ product green
- Cap1 CR/G2/G5/G6 EXIT0 = `pass_component_desktop_cap_topology` only
- Completeness EXIT0 with product=0 is honesty, not success
- tip_bind.ok=false until matrix rebound
- No forge AUTH/AT; no invent Cap free; no invent `pass_product_clean_room`

## 8. Wiring

| Artifact | Path |
| --- | --- |
| This plan | `.spec/plan/20261110044500-production-status-priority-reorder.plan.md` |
| Cap=0 evidence | `.spec.local/root/evidence/20261110044500-cursor-priority-reorder/` |
| Prior STATUS | `plan:root/20261110043600-production-status-live-cap0` |
| Matrix | `.spec.local/root/evidence/20261108164500-cursor-clean-room-matrix/` |
| LIVE canvas | `canvases/humanymous-live-roadmap.canvas.tsx` (Cursor projects) |
