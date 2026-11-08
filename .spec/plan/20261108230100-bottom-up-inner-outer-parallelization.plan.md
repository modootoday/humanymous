---
id: plan:root/20261108230100-bottom-up-inner-outer-parallelization
title: Bottom-up / inner-to-outer parallelization priority (tip 87be1b76)
status: active
created: 2026-11-08T14:01:00Z
updated: 2026-11-08T14:01:00Z
rel:
  depends_on:
    - sot:root/structural-development
    - sot:root/program-roadmap
    - sot:root/rust-migration
    - plan:root/20261107151000
    - plan:root/20261108220000-post-g4-w2-residual-root-ask
    - plan:root/20261108220600-cap-ready-g4-w2-harness
    - plan:root/20261108221000-clean-room-matrix-cap-driver
  governs: []
---

# Bottom-up / inner-to-outer parallelization priority

Brief: Canonize the tip-`87be1b76` bottom-up / inner-to-outer parallelization
priority table for concurrent workstreams. Detection freeze. **product=0**.
Cap conserved (no Cap burn in this plan). Never invent `pass_product` /
`pass_product_clean_room`. Titles do not grant paths.

**SoT delta:** **not warranted** this turn. L0–L5 layer vocabulary and promotion
rules already live in `sot:root/structural-development`; Unlock A/B/C wake gates
already live in sibling Cap-ready plans. This plan binds tip-observed priorities,
mutexes and honesty without changing scoring, layer definitions or product
authority. Root may later promote a slice into SoT after review.

Private receipt (gitignored):  
`.spec.local/root/evidence/20261108230100-cursor-bottom-up-parallelization/`

Source table: prior Cursor read-only delivery at tip ~`87be1b76` (agent
`888ff6a7`), mapped to SoT layer roles in `structural-development.sot.md` and
milestones/G1–G9 in `program-roadmap.sot.md`.

## 1. Problem

Multiple chats can dirty distinct paths in parallel, but without an explicit
inner→outer priority they burn Cap on outer feeders, invent product greens, or
collide on scoring / Cap / git-ops. Need one canon plan that states:

1. Layer rings L0→L5 with priorities P0–P3
2. What may run in parallel vs required mutexes
3. Unlock A/B/C Cap wake gates
4. Honesty: component ≠ product; product=0 until matrix admits

## 2. Vocabulary

| Term | Meaning |
| --- | --- |
| Ring | Core (L0–L1) → Skeleton (L2–L3) → Outer (L4–L5) → Final (M5–M6) |
| P0–P3 | Implementation priority within current tip; P0 = unlocks most blocked consumers or product closers |
| Cap=1 | Single heavy Docker / CEF / product burn slot; free ≠ closable |
| Unlock A/B/C | Mutually sufficient Root path transfers; any one wakes Cap=1 |
| Owner class | Root-only · Cursor-claimable · blocked Unlock A\|B\|C |
| Status | 완료 / 진행 / 보류 / 대기 — component receipts never upgrade to product |

## 3. Decision rules

R-P1. **Inner before outer for product closers.** Prefer Core/Skeleton
contracts and actual boundary providers before Outer Cap burns.

R-P2. **Parallelize by nonoverlapping paths.** Distinct file ownership may
progress concurrently; shared Cargo/lock/toolchain remains Root-coordinated.

R-P3. **Mutexes are mandatory:**
- `detection-core` ↔ `gate-edge` on scoring / Pass / Engine seams
- **Cap=1** global for heavy Docker / CEF / product `--require-full`
- **git-ops** serial for index / commit / push
- CEF / native-heavy / Docker cold streams are mutually exclusive when
  resource-saturated

R-P4. **Unlock wake.** Cap=1 may burn only after Root transfers **any one** of
A (G4 CEF/AT), B (W2 H2 product-binding), or C (authority Cap TRANSFER), with
Cap free and no Cap-spam reburn. Matching Cap-ready pack only.

R-P5. **Honesty.** `pass_component_*` ≠ `pass_product(_clean_room)`. Cap free
does not authorize Cap spam. `product=0` / `full_product_qualified=false` until
ENVIRONMENTS × clean-room matrix admits. Detection freeze holds.

R-P6. **M5→M6 serial.** M1–M4 may parallel under separate ownership; M5 and M6
remain final serial gates held by M1–M4.

## 4. Priority table (L0→L5 / P0–P3)

Statuses are tip-`87be1b76` honest observations. Component green ≠ product.

| Ring | Feature / workstream | Pri | Parallel OK / mutex | Owner class | Status | Why |
| --- | --- | --- | --- | --- | --- | --- |
| Core · L0 | Shared contracts · package family · typed ownership (→ **M1**) | **P0** | Most streams; **git-ops** serial; shared Cargo/lock **Root-only** | Root-only (graph) | 진행 | Contract baseline for every upper column |
| Core · L1/X | Shared core: signals · Integrity · Pass · frozen Engine | **P0** | Column-parallel with Model/Watermark/History/UIState; scoring changes need **detection↔gate mutex**; detection freeze | Root + X Integration | 진행 (component) | Inner engine; Pass/scoring frozen |
| Core · L1 | Model training/calibration kernels | **P1** | Parallel if paths disjoint; GPU/heavy needs separate Cap; no production activation | Module owner | 진행/보류 (lab) | Pure→L2 lab; not product activation |
| Core · L1 | Watermark · resource codecs | **P1** | Parallel with Core/Model; carrier/file pilots after L2 | Module owner | 진행 (pure) | Pure first; product pilots are outer |
| Core · L1 | Network history: MAD · rate · correlation · traffic state | **P0** | Vertical with Capture(L2); contract-prep parallel with Runtime | L2 Network owner | 진행 (pure) | Pure base for G1 capture |
| Core · L1 | ABI handles/buffers · Browser UI state (pure) | **P1** | Column-separated from Native ABI / CEF | ABI · Browser owners | 진행 (pure) | Pure before boundary; GUI ≠ native success |
| Skeleton · L2 | Native security · process/pipe ownership | **P0** | Consumed by Audit/Authority/CLI/Dotnet; **native-heavy exclusive** vs CEF/Docker cold | L2 Native | 진행 (boundary partial) | Shared process primitives; no forks |
| Skeleton · L2 | Audit WAL · Merkle · recovery inputs | **P0** | Parallel with Native if disjoint; actual proof before Authority/Runtime consume | L2 Audit | 진행 (component≠prod auth) | Durable base for authenticated services |
| Skeleton · L2 | **G1** TLS/H2 capture · connection epoch | **P0** | History∥; Runtime vertical; Cap=1 capture feeder ≠ product; Unlock **B** for W2 H2 binding | Root-only product close; capture lane claimable for component | 진행/대기 | Highest M3 network closer |
| Skeleton · L2 | Native ABI · C/C++ interop | **P1** | Prep parallel with Framework/Dotnet; unsafe stays narrow | L2 ABI | 진행 (pure→boundary) | Shared L4 adapter boundary |
| Skeleton · L2→L4 | CEF host boundary (pre-GUI) | **P1** | Mutex with native-heavy; Cap execute blocked until Unlock **A** | Root path release → Cursor Cap | 보류 / `blocked_claim` | G4 AT prerequisite; pure UI ≠ CEF |
| Skeleton · L3 | Authority · authenticated IPC | **P0** | After Audit/Native; Framework/Servers vertical; Unlock **C** Cap path | L3 Authority; Unlock C TRANSFER waiting | 진행 + Cap `blocked_claim` | Auth skeleton; Cap green ≠ `pass_product` |
| Skeleton · L3 | Runtime Gate · session/RIT/ACK · audit-before-effect | **P0** | Consumes Capture/Authority; **no duplicate scoring** (detection↔gate mutex) | L3 Runtime | 진행 (component) | Service composition center; Pass handler owner |
| Skeleton · L3/X | **G2** session-bound collector provenance | **P0** | Runtime/Collector vertical; Cap=1 browser feeder = `pass_component_*` only | Root-only (acceptance join) | 진행/대기 | M3 collector path; no invented predicates |
| Skeleton · L3 | **G3** Pass recovery (SoT-36) | **P1** | Detection freeze; Pass lane∥docs; no concurrent scoring-seam edits | Root-only | 대기 | Protocol closer; Cursor must not invent |
| Outer · L4 | CLI · service host | **P1** | Consumes shared Native; parallel with adapter columns | L4 CLI | 진행 (component) | Host outer shell; no process-primitive forks |
| Outer · L4 | Meari · CEF browser (+ **G4** AT/IME) | **P1** | Unlock **A**; CEF heavy exclusive; a11y non-motor | Root unlock → Cursor Cap-ready | 대기 Unlock A | Interactive/AT; never motor-gate Pass |
| Outer · L4 | Node/Python/Ruby · nginx · Apache · Go · ASP.NET | **P2** | **Mutual parallel** (disjoint paths) after Authority/ABI qualify | Each adapter owner | 혼재 (component / scaffold) | Outer shell; component ≠ M3 |
| Outer · L5 | **M3** Docker clean-room product e2e (**G5–G6** witness/origin·audit bind) | **P0\*** | **Cap=1 global**; after Unlock **A∨B∨C** one burn; no feeder reburn | Root allocate; Cursor Cap-ready waiting | 대기 (product=0) | Sole product authority: `pass_product_clean_room` |
| Outer · L5 | Wargame · report validation | **P2** | After Lab; prefer Cap serial; preserve unsupported denominators | L5 Wargame | 진행 (component) | Adversarial; full ≠ ignore unsupported |
| Outer · L5 | **G7–G9** attestation · native OS · provider deploy (+ Delivery/CI) | **P2** | Cross-OS∥Pages prep; publication/OIDC unauthorized | Root-only | 대기/보류 | Post-M3 outer qualification |
| Outer · L5 | **M4** Pages · Draft v0 | **P2** | Parallel with M1–M3 when paths disjoint; no over-claim | Pages owner (`docs/**`) | 진행 | Public docs; artifact ≠ serving |
| Final | **M5** shared main → **M6** main-only/Pages | **P3** | **Serial**; M1–M4 hold; **git-ops** exclusive | Root-only | 보류 / 미착수 | Final gate; no invented main/force |
| X · now | Cap-ready packs · matrix honesty · Unlock C candidate TRANSFER | **P0 (Cursor)** | Cap burn=0 until unlock; git-ops for owned evidence/plans only | **Cursor claimable** (prep); unlock Root | 완료(prep) / 대기(unlock) | Conserve Cap; forbid product invention |

\*M3 is product-priority P0 but **not executable now** (Unlock waiting). Cap free =
empty slot ≠ closable cell.

## 5. Unlock A / B / C wake gates

Any **one** unlock wakes Cap=1 on the matching Cap-ready pack. Authority Cap
green alone is not `pass_product_clean_room`.

| Unlock | Need | Cap-ready | Tip status @ `87be1b76` |
| --- | --- | --- | --- |
| **A** | G4 CEF/AT path release (`crates/browser/**`, `scripts/ci/cef-pilot/**` or `browser-ui` execution admission) | `cap-ready/g4-cef-at/` | 대기 (`blocked_claim`; prior “권고대로” ≠ Unlock A) |
| **B** | W2 hosted gateway H2 product-binding + N06 (not capture `observe_h2` alone) | `cap-ready/w2-h2-product-binding/` | 대기 (Root-blocked) |
| **C** | Authority Cap CLI paths (`deployments/lab/**`, lab-runner orchestration, …) | `cap-ready/authority-node-ipc/` + `candidate-pack/TRANSFER.md` | candidate-pack authored; live transfer **not applied** |

Governing sibling plans: `plan:root/20261108220000-post-g4-w2-residual-root-ask`,
`plan:root/20261108220600-cap-ready-g4-w2-harness`,
`plan:root/20261108221000-clean-room-matrix-cap-driver`.

## 6. M1–M6 mapping

| ID | Status | Note |
| --- | --- | --- |
| **M1** contracts/package | 진행 | Core ring P0 |
| **M2** component runtime | 진행 | L2–L4 component; ≠ product |
| **M3** composed product e2e | **미자격** | G1–G9 Root-only; Cap feeders insufficient |
| **M4** Pages/Draft v0 | 진행 | Path-disjoint parallel with M1–M3 OK |
| **M5** main integration | M1–M4 hold | Serial |
| **M6** main-only | 미착수 | After M5 |

## 7. Wiring

| Artifact | Path |
| --- | --- |
| This plan | `.spec/plan/20261108230100-bottom-up-inner-outer-parallelization.plan.md` |
| Layer canon | `.spec/sot/structural-development.sot.md` (L0–L5; unchanged) |
| Roadmap / Cursor contract | `.spec/sot/program-roadmap.sot.md` (unchanged this turn) |
| Earlier pyramid plan | `.spec/plan/20261107151000-program-roadmap-bottom-up.plan.md` |
| Cap-ready evidence | `.spec.local/root/evidence/20261108192100-cursor-tls-h2-at-gap-inventory/cap-ready/` |
| This receipt | `.spec.local/root/evidence/20261108230100-cursor-bottom-up-parallelization/` |

No lab-runner heavy edits in this turn (impl sibling owns Cap-ready
executability). No Cap burn. No SoT file writes.

## 8. Verification

- Branch `codex/rust` @ tip containing this plan after commit
- Session `cursor-integrator` / lane `cursor-integration` claimed before write
- Plan-only: no `.spec/sot/**` mutation; no detection/scoring edits
- Cap burn this turn: **false**; product: **0**; unlock: **false**
- Private evidence INDEX + table snapshot under claimed `.spec.local/...`

## 9. Residual

Root may (a) transfer Unlock A/B/C and allocate Cap=1, and/or (b) after
adversarial critique, promote a short SoT cross-link from
`structural-development` / `program-roadmap` to this plan. Until then this plan
is the tip-bound parallelization canon. Prefer Cap burn over more docs once
unlocked.
