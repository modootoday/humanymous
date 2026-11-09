---
id: plan:root/20261110061400-full-remaining-priority-no-cutoff
title: Cap=0 FULL remaining priority 1..N — no quantity cutoff
status: active
created: 2026-11-09T21:14:00Z
updated: 2026-11-09T23:32:00Z
rel:
  depends_on:
    - plan:root/20261110060500-code-sot-gap-browser-priority-reorder
    - plan:root/20261110044500-production-status-priority-reorder
    - plan:root/20261110043600-production-status-live-cap0
    - sot:root/program-roadmap
    - sot:root/clean-room-usage-matrix
    - sot:root/structural-development
    - sot:root/browser-ui
    - sot:root/client-collector
    - sot:root/chromium-lab-browser
  governs: []
---

# Cap=0 FULL remaining priority (수량제한없이 / no N-cap)

Brief: User correction — priorities must be set **without quantity limits**.
Do **not** truncate to top-10 / top-15. This plan replaces truncated tables in
20261110060500 / 20261110044500 with a **complete ordered list of ALL
remaining units** (N=80). Cap=0 only. Detection freeze.
Never invent pass_product_clean_room / Cap free / AUTH / AT / BrowserReady.

Private evidence:
.spec.local/root/evidence/20261110061400-cursor-full-priority-no-cutoff/

Machine table: PRIORITY-FULL.json (authoritative) · PRIORITY-FULL.tsv

## 0. Parallel-serial GOAL (tip-bound)

**goal_name:** Cap0∥+Cap1serial → pass_product_clean_room

**goal_text_exact:** FULL end-state: achieve pass_product_clean_room + Docker clean-room full matrix + red-blue-validate (Docker-only). Execution contract: Cap0∥ — run ALL claimable Cap0 honesty / Cap-ready / prep / tooling in parallel (including Cap realloc apply CLI absorb when landed); Cap1/Docker — SERIAL only when Cap free=true AND NEW non-twin Cap-ready matching GRANT→TRANSFER (no twin G2/G5/CR/G6/RB/PM reburn); Root_blocked — Cap-ready/handoff prep + wake only, never forge AUTH/AT/Cap free/BrowserReady/pass_product; Cap burn=0 while Cap free=false. goal_complete=false until product gates verified.

Receipt: `.spec.local/root/evidence/20261110083200-cursor-parallel-serial-goal/GOAL-PARALLEL-SERIAL.json`

## 1. LIVE Cap=0 probe (tip-bind)

| Field | Measured |
| --- | --- |
| tip | fdad38658af0056bd8677c8e4ea48a8c83a84a4f (Cap realloc apply CLI landed + parallel-serial goal tipbind; product=0) |
| branch | `codex/rust` |
| decision | **B** (all remaining product cells need AUTH/AT/BrowserReady or Root structural; Cap burn=**0**) |
| product / pass_product_clean_room | **0** / **0** |
| Cap free before / after | **false** / **false** (spent false→true→false on prior PM Cap1) |
| Cap realloc LAST CHECK | **no** NEW non-twin Cap-ready with matching GRANT→TRANSFER preimages → Cap burn=**0**; Cap0 `cap-realloc-apply` CLI **landed** (absorb; live apply HOLD) |
| Cap EXIT this turn | **none** |
| prior Cap1 PM | EXIT **2** · 6/0 · `--clean-room-product-matrix --require-full` · feeder≠product |
| AUTH-HANDOFF | **absent** (`humanymous.root-n06-owner-join-auth-handoff/1`) |
| AT-HANDOFF | **absent** (`humanymous.root-interactive-at-windows-tuple/1`) |
| BrowserReady / n06_owner_join | **false** / **false** |
| CEF Cap-day / CEF env / apps/humanymous-browser / crates/browser | **bound** (durable allowlist) / **unset** (`HUMANYMOUS_CEF_PILOT_BUNDLE`) / **absent** / **present** |
| matrix tip_bind | LIVE tip-bound **fdad3865** (Cap realloc CLI + parallel-serial goal Cap0 tipbind); tip_bind.ok=**true** |
| Unlock C | **false** · transfer-probe 6 digest mismatches · Cap0 rematch digests packet (no spam) |
| required / clean_room_required / product | 84 / 47 / 0 |
| missing product cells (EXIT2) | **47** clean-room-required non-product; **16** product_matrix_open_cells |
| missing SoT scenario | `red-blue-product` (1) |
| Unlock A / B / C | true / true / false |
| Cap burn this turn | **0** |
| Cap0∥ remaining | goal inventory + Cap realloc CLI absorb + tipbind + Cap1 HOLD (product-closing Cap0 still 0) |
| Cap0 advanced (this wave) | parallel-serial GOAL define; Cap realloc apply CLI absorb (5 tests); pending inventory A–H; Cap free/AUTH/AT inspect; tipbind; Cap1 HOLD |
| Exact ranks still Root/Cap1 | **1, 4–9, 14–16+** (14=PM Cap1 spent component; product still open) |
| CONSOLIDATED closer | `.spec.local/root/evidence/20261110083200-cursor-parallel-serial-goal/` |
| goal_complete | **false** |

## 2. Decision rules (no cutoff)

R0. **No quantity limit.** Emit ranks 1..N for every remaining unit/gap. Same
rank only if true Cap=0 parallel siblings that cannot be ordered — this table
uses a **strict total order** (no shared ranks).

R1. BrowserReady stack stays first-class near the top (ranks 1–6) where the
critical path requires — but nothing below is hidden by a cut-off.

R2. Critical path still first: AT/CEF/Browser → AUTH/N06 → Cap realloc → Cap=1
once → clean-room product → red-blue → M5/M6.

R3. Cap=0 parallel OK when path-disjoint. Cap=1 serial after realloc once.
Never forge AUTH/AT. Deprioritize Cap twin spam (still listed, not truncated).

R4. Clean-room **cell classes** (scenarios + blocker classes) are enumerated
individually; individual matrix cells remain under those classes (84 required /
47 clean-room-required).

## 3. Class vocabulary

| Class | Meaning |
| --- | --- |
| Cap0_parallel | Cap=0 path-disjoint work this lane may progress |
| Cap1_serial | Requires Cap free=true + one Cap=1/Docker burn; HOLD now |
| Root_blocked | Root AUTH/AT/structural/TRANSFER; Cap=0 wake/honesty only |
| Deprioritized_component_twin | Listed complete; must not burn Cap or invent product |

Class counts: {"Root_blocked": 28, "Cap0_parallel": 24, "Cap1_serial": 14, "Deprioritized_component_twin": 14}

## 4. FULL priority table 1..N (N=80)

| Rank | ID | Unit | Class | Blocker | depends_on | Why this rank |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | U-AT-HANDOFF | AT-HANDOFF (root interactive AT Windows tuple) | Root_blocked | root_at_handoff | [] | Hard-stop for BrowserReady; Cap=0 wake measured absent |
| 2 | U-CEF-BUNDLE | CEF pilot bundle (HUMANYMOUS_CEF_PILOT_BUNDLE) | Root_blocked | root_at_handoff+bundle_absent | [U-AT-HANDOFF] | G4 Cap CLI bound but bundle unset; Unlock A ≠ ready |
| 3 | U-BROWSER-SURFACES | crates/browser/** + scripts/ci/cef-pilot/** Cap=0 wake surfaces | Cap0_parallel | path_release/AT | [] | Keep Browser stack visible; honesty refuse; path-disjoint Cap=0 |
| 4 | U-HUMANUMOUS-BROWSER-PKG | Package consolidation 3→1 crates/apps/humanymous-browser | Root_blocked | structural_missing_apps_path | [U-BROWSER-SURFACES] | SoT census requires apps path; live tree still crates/browser only |
| 5 | U-MEARI-CEF-QUAL | Meari+CEF UI real qualification (nav/keyboard/IME/AT/DPI/sandbox) | Root_blocked | BrowserReady=false | [U-AT-HANDOFF,U-CEF-BUNDLE] | Component pilots ≠ BrowserReady; external browse disabled |
| 6 | U-BROWSERREADY-JOIN | BrowserReady measured join (interactive AT honesty→true) | Root_blocked | root_at_handoff | [U-AT-HANDOFF,U-CEF-BUNDLE,U-MEARI-CEF-QUAL] | Apex BrowserReady closer; never invent true |
| 7 | U-AUTH-HANDOFF | AUTH-HANDOFF (N06 owner-join schema) | Root_blocked | root_auth_handoff | [] | Hard-stop for N06; Cap=0 wake measured absent; Cap0 sibling of AT |
| 8 | U-N06-JOIN | N06 owner-join measured apply | Root_blocked | root_auth_handoff | [U-AUTH-HANDOFF] | n06_owner_join=false until AUTH present + Cap=0 apply |
| 9 | U-CAP-REALLOC | Cap realloc via live Cap topology TRANSFER | Root_blocked | cap_free_false | [] | Cap free=false after G2/G5/CR/G6/RB/PM spends; NEW non-twin feeder only; never invent Cap free |
| 10 | U-UNLOCK-C-DIAG | Unlock C Cap=0 digest mismatch diagnosis | Cap0_parallel | blocked_claim_digest_mismatch | [] | Cap0 done; C=false; live digests≠candidate; no Cap burn / no forge |
| 11 | U-UNLOCK-A-RESIDUAL | Unlock A residual honesty (g4 bound ≠ BrowserReady) | Cap0_parallel | Unlock_A_true_ne_BrowserReady | [U-BROWSERREADY-JOIN] | Cap0 done; A=true already; residual is close BrowserReady not re-unlock |
| 12 | U-UNLOCK-B-RESIDUAL | Unlock B residual honesty (w2-h2 bound ≠ product) | Cap0_parallel | Unlock_B_true_ne_product | [U-CAP-REALLOC] | Cap0 done; B=true already; residual is product Cap burn after realloc |
| 13 | U-MATRIX-TIPBIND | Matrix tip-bind head_sha→LIVE tip (Cap=0 hygiene) | Cap0_parallel | tip_drift | [] | Cap0 done; tip_bind.ok=ad8b68a2 Decision B; ≠ pass_product |
| 14 | U-CAP1-CR-ONCE | Cap=1 once: --clean-room-product / product-matrix (preferred) | Cap1_serial | cap_free_false+AUTH_AT_absent+PM_spent_component | [U-CAP-REALLOC,U-AUTH-HANDOFF,U-AT-HANDOFF] | PM Cap1 EXIT2 feeder spent; product still 0; HOLD until AUTH/AT + NEW Cap realloc |
| 15 | U-CAP1-W2H2-ALT | Cap=1 alternate: Unlock-B --w2-h2-product-binding once | Cap1_serial | cap_free_false+Decision_B_hold | [U-CAP-REALLOC,U-AUTH-HANDOFF,U-AT-HANDOFF] | Decision B: Cap1 without AUTH/AT cannot close product cells; never twin spam |
| 16 | U-RB-SCENARIO-LAND | Land missing SoT scenario red-blue-product into matrix | Cap1_serial | cap_free_false+missing_sot_scenario | [U-CAP-REALLOC,U-AUTH-HANDOFF,U-AT-HANDOFF] | missing_sot_scenario_count=1; GRANT present; Cap free + AUTH/AT block |
| 17 | U-CR-CLASS-CLEANROOM | CR cell class: clean-room-product → pass_product_clean_room | Cap1_serial | measured_component_hold+cap_topology_spent | [U-CAP1-CR-ONCE,U-AUTH-HANDOFF,U-AT-HANDOFF] | Sole product authority status; Cap twin/PM hold ≠ product |
| 18 | U-CR-CLASS-BROWSER | CR cell class: browser-product (desktop+hosted require-full/a11y) | Root_blocked | BrowserReady_AT_absent+fail_hosted | [U-BROWSERREADY-JOIN,U-AT-HANDOFF] | Decision B: open cells need BrowserReady/AT; hosted fail_needs_structural_delta |
| 19 | U-CR-CLASS-M3FULL | CR cell class: m3-full-product (root_structural_g1_g9) | Root_blocked | root_structural_g1_g9 | [U-G1,U-G3,U-BROWSERREADY-JOIN,U-G2,U-G5,U-G6] | required_unavailable until Root structural G1–G9 deltas |
| 20 | U-CR-CLASS-G2 | CR cell class: g2-collector-product-binding PRODUCT re-qualify | Root_blocked | measured_component_hold+cap_topology_spent | [U-AUTH-HANDOFF,U-AT-HANDOFF,U-CAP-REALLOC] | Cap1 twin spent; product cell still open |
| 21 | U-CR-CLASS-G5 | CR cell class: g5-witness-transport PRODUCT re-qualify | Root_blocked | measured_component_hold+cap_topology_spent | [U-CAP-REALLOC] | Cap1 twin spent; authenticated witness product still open |
| 22 | U-CR-CLASS-G6 | CR cell class: g6-origin-audit-binding PRODUCT re-qualify | Root_blocked | measured_component_hold+cap_topology_spent | [U-CAP-REALLOC] | Cap1 twin spent; L5 origin/audit product still open |
| 23 | U-CR-CLASS-W2H2 | CR cell class: w2-h2-product-binding (+ hosted venue) | Cap1_serial | measured_component_hold | [U-CAP1-W2H2-ALT] | Unlock B bound; product promotion still Cap1 after realloc |
| 24 | U-CR-CLASS-AUTHIPC | CR cell class: authority-node-ipc require-full | Cap1_serial | Unlock_C_blocked_claim | [U-UNLOCK-C-DIAG,U-AUTHORITY-C-TRANSFER] | Component holds; Unlock C digest mismatch blocks Cap path |
| 25 | U-CR-CLASS-CUSTODY | CR cell class: custody → product auth | Cap0_parallel | measured_component_hold | [U-AUDIT-PRODUCT] | Custody pilots ≠ product auth; Cap0 path-disjoint OK |
| 26 | U-CR-CLASS-RUNTIME | CR cell class: runtime require-full product | Cap0_parallel | measured_component_hold | [U-RUNTIME-GATE] | Component Docker/process ≠ full Gate/browser/Pass |
| 27 | U-CR-CLASS-WITNESS | CR cell class: witness require-full product | Root_blocked | measured_component_hold | [U-CR-CLASS-G5] | Witness product waits G5 product re-qualify |
| 28 | U-CR-CLASS-COLLECTOR | CR cell class: collector-component → session-bound product | Root_blocked | measured_component_hold | [U-CR-CLASS-G2,U-COLLECTOR-WASM] | Collector component ≠ session-bound browser product |
| 29 | U-CR-CLASS-TOPOLOGY | CR cell class: topology-fixture clean-room gates | Cap0_parallel | measured_component_hold | [] | Component holds; Cap0 hygiene/parallel; not product closer alone |
| 30 | U-CR-CLASS-RUSTCTL | CR cell class: rust-controls clean-room gates | Cap0_parallel | measured_component_hold | [] | Component holds; Cap0 parallel; not product closer alone |
| 31 | U-CR-CLASS-GO-FAIL | CR cell class: go-product desktop fail_needs_structural_delta | Root_blocked | fail_needs_structural_delta | [] | lab_runner_require_go_component fail; Cap retry forbidden pre-delta |
| 32 | U-CR-CLASS-GO-HOSTED | CR cell class: go-product hosted blocked_hosted_linux | Root_blocked | blocked_hosted_linux | [] | Hosted Linux allocation / Specfold provenance required |
| 33 | U-CR-CLASS-HOSTED-BROWSER | CR cell class: hosted browser-product require-full fail | Root_blocked | fail_needs_structural_delta | [U-BROWSERREADY-JOIN] | Hosted browser fail needs Root structural delta |
| 34 | U-CR-CLASS-HOSTED-M3 | CR cell class: hosted m3-full-product require-full fail | Root_blocked | fail_needs_structural_delta | [U-CR-CLASS-M3FULL] | Hosted M3 fail needs Root structural delta |
| 35 | U-CR-CLASS-ORACLE-RB | CR cell class: oracle-detector-bots red-blue phases | Cap1_serial | cap_free_false+blocked_root_claim | [U-RB-VALIDATE,U-RB-SCENARIO-LAND] | red_blue_* unrun/blocked; host≠completion; Cap free required |
| 36 | U-CR-CLASS-ENGINE | CR cell class: engine-affinity (component hold) | Deprioritized_component_twin | measured_component_hold | [] | Already component pass; not on product critical path |
| 37 | U-CR-BLOCKER-MCH | CR blocker class: measured_component_hold (~71 cells) promotion ban | Deprioritized_component_twin | measured_component_hold | [] | Honesty class: never promote component/fixture to product |
| 38 | U-CR-BLOCKER-CAPFREE-ADAPTERS | CR blocker class: cap_free_false adapter-home-pilot unrun | Cap1_serial | cap_free_false | [U-CAP-REALLOC] | apache/python/ruby/dotnet/native adapter_pilot_contract unrun |
| 39 | U-G1 | G1 TLS/H2 capture + session binding product | Root_blocked | root_structural_g1_g9 | [] | Structural required_unavailable; network-capture SoT |
| 40 | U-G2 | G2 session-bound collector product (not Cap twin) | Root_blocked | root_grant_g2+AUTH/AT | [U-AUTH-HANDOFF,U-AT-HANDOFF,U-CAP-REALLOC] | Cap topology spent; product still open |
| 41 | U-G3 | G3 Pass recovery (SoT-36) | Root_blocked | detection_freeze+Root_only | [] | Detection freeze; Cursor must not invent Pass deltas |
| 42 | U-G4 | G4 interactive AT / CEF product close | Root_blocked | root_at_handoff | [U-BROWSERREADY-JOIN] | First-class critical path continuation after BrowserReady |
| 43 | U-G5 | G5 authenticated witness transport product | Root_blocked | root_grant_g5 | [U-CAP-REALLOC] | Cap twin spent; product re-qualify required |
| 44 | U-G6 | G6 L5 origin · audit binding product | Root_blocked | root_structural_g1_g9 | [U-CAP-REALLOC,U-AUDIT-PRODUCT] | Cap twin spent; cross-namespace product still open |
| 45 | U-G7 | G7 remote attestation | Deprioritized_component_twin | post_M3_outer | [U-M3] | Deprioritized until M3 product cells; no Cap-ready invent |
| 46 | U-G8 | G8 native OS tuples | Deprioritized_component_twin | post_M3_outer | [U-M3] | Deprioritized until M3 product cells |
| 47 | U-G9 | G9 provider deployment | Deprioritized_component_twin | publication_unauthorized | [U-M3,U-M6] | Publication unauthorized; post-product |
| 48 | U-RB-VALIDATE | red-blue-validate Docker require-full (product) | Cap1_serial | cap_free_false | [U-CR-CLASS-CLEANROOM,U-RB-SCENARIO-LAND] | Docker-only completion; host/loopback ≠ authority |
| 49 | U-COLLECTOR-WASM | Collector WASM session-bound browser binding | Cap0_parallel | G2_product_open | [U-G2] | WASM source live; product binding waits G2/AUTH/AT |
| 50 | U-RUNTIME-GATE | Runtime / gate fail-closed full composition | Cap0_parallel | component_ne_full_Gate | [] | Component process/Docker OK; full Gate+browser+Pass open |
| 51 | U-AUDIT-PRODUCT | Audit / custody dual-control product auth | Cap0_parallel | component_ne_product_auth | [] | Custody witness hold; dual-control erasure Root product |
| 52 | U-AUTHORITY-C-TRANSFER | Authority Unlock C path transfer (Root digest rematch) | Root_blocked | blocked_claim | [U-UNLOCK-C-DIAG] | Root must rematch digests or re-issue TRANSFER |
| 53 | U-ADAPTER-NGINX | Adapter nginx Desktop residual pilots | Cap0_parallel | component_hold | [] | Path-disjoint Cap=0; not M3 closer alone |
| 54 | U-ADAPTER-HOME-UNRUN | Adapter home-pilot unrun pack (apache/python/ruby/dotnet/native) | Cap1_serial | cap_free_false | [U-CAP-REALLOC,U-CR-BLOCKER-CAPFREE-ADAPTERS] | Five adapter_pilot_contract cells unrun under Cap free=false |
| 55 | U-PKG-ADAPTERS | Package consolidation humanymous-adapters (12 contexts→1) | Root_blocked | structural_consolidation | [] | structural-dev census; Root/integration path release |
| 56 | U-M1 | M1 shared contracts · package structure residuals | Cap0_parallel | in_progress | [] | Path-disjoint Cap=0 OK; shared Cargo Root-only |
| 57 | U-M2 | M2 component runtime qualification residuals | Cap0_parallel | component_ne_product | [] | Component pilots continue; deprioritize Cap twin spam |
| 58 | U-M3 | M3 composed product e2e (clean-room apex) | Cap1_serial | G1-G9+product=0 | [U-CR-CLASS-M3FULL,U-RB-VALIDATE] | Not qualified; critical-path apex after product cells |
| 59 | U-M4 | M4 Pages · Draft v0 | Cap0_parallel | in_progress | [] | Parallel Cap=0 when paths disjoint |
| 60 | U-M5 | M5 shared main integration | Cap1_serial | held_by_M1-M4 | [U-M1,U-M2,U-M3,U-M4] | Serial final after M1–M4 gates |
| 61 | U-M6 | M6 main-only · approved Pages | Cap1_serial | not_started | [U-M5] | After M5; retire obsolete only with evidence |
| 62 | U-FM0 | Feature M0 Specfold / agents hygiene residual | Cap0_parallel | maintain | [] | Implemented maintain; final agents cleanup ties M11 |
| 63 | U-FM1 | Feature M1 pure engine / model / Pass residuals | Cap0_parallel | detection_freeze | [] | Partial; detection freeze on scoring; Cap0 tests OK |
| 64 | U-FM2 | Feature M2 network/raw capture residuals | Root_blocked | overlaps_G1 | [U-G1] | TLS ClientHello/H2/session binding open |
| 65 | U-FM3 | Feature M3 audit/keys/state residuals | Cap0_parallel | overlaps_audit | [U-AUDIT-PRODUCT] | Witness/rotation/dual-control/backup open |
| 66 | U-FM4 | Feature M4 Core/Gate + authority residuals | Cap0_parallel | bounded_runtime | [U-RUNTIME-GATE,U-AUTHORITY-C-TRANSFER] | Collector/capture/RIT/proxy/admin paths incomplete |
| 67 | U-FM5 | Feature M5 Rust collector residuals | Cap0_parallel | overlaps_G2 | [U-COLLECTOR-WASM,U-G2] | Loader/session/RIT/recovery/Pass/API matrix open |
| 68 | U-FM6 | Feature M6 server adapters residuals | Cap0_parallel | partial_research | [U-ADAPTER-HOME-UNRUN,U-PKG-ADAPTERS] | Peer/route/header/stream/authority open |
| 69 | U-FM10 | Feature M10 Cross-OS / OIDC / Pages | Deprioritized_component_twin | post_product_wiring | [U-M4,U-M6] | Scoped wiring; full runners/registry after product |
| 70 | U-FM11 | Feature M11 Rust-only / supply chain / agents | Cap0_parallel | partial | [U-FM0] | Foreign disposition/CVE/SBOM/agents eval Cap0 OK |
| 71 | U-FM12 | Feature M12 retain only final main | Cap1_serial | deferred | [U-M5,U-M6] | Main update + Pages verify + branch removal serial |
| 72 | U-DOCS-SOT-DRIFT | Docs/SoT drift remediation (roadmap vs LIVE tip) | Cap0_parallel | tip_doc_drift | [U-MATRIX-TIPBIND] | Cap=0 honesty docs; never invent product green in docs |
| 73 | U-TWIN-G2 | G2 Cap1 component twin reburn | Deprioritized_component_twin | spent_twin | [] | Spent; product still 0; forbidden Cap spam |
| 74 | U-TWIN-G5 | G5 Cap1 component twin reburn | Deprioritized_component_twin | spent_twin | [] | Spent; product still 0; forbidden Cap spam |
| 75 | U-TWIN-CR | CR Cap1 component twin reburn | Deprioritized_component_twin | spent_twin | [] | Spent topology hold; never reburn |
| 76 | U-TWIN-G6 | G6 Cap1 component twin reburn | Deprioritized_component_twin | spent_twin | [] | Spent; product still 0; forbidden Cap spam |
| 77 | U-TWIN-A11Y-CAP | a11y / --require-full Cap spam loops | Deprioritized_component_twin | cap_spam | [] | Does not shorten product path; Cap free=false |
| 78 | U-TWIN-DOCS-SPAM | Documentation-only / CONSOLIDATED-ROOT-ASK spam | Deprioritized_component_twin | busywork | [] | Does not shorten critical path |
| 79 | U-TWIN-G789-FEEDER | G7/G8/G9 Cap-ready invention before M3 | Deprioritized_component_twin | pre_M3_forbidden | [] | No Cursor Cap-ready feeder; invent forbidden |
| 80 | U-TWIN-DESKTOP-ORACLE | Full e2e desktop oracle ≠ product claim | Deprioritized_component_twin | component_pass_desktop_oracle | [] | Honesty hold: EXIT0 oracle class is not pass_product |

## 5. Cap=0 parallel tracks (exhausted)

| Track | Units | Exit |
| --- | --- | --- |
| T1 Browser wake | ranks 1–6 Cap0 surfaces + honesty | Cap0 done/honesty; BrowserReady=false; AT still Root |
| T2 AUTH/N06 wake | ranks 7–8 | Cap0 wake absent; AUTH still Root |
| T3 Unlock C + tip-bind | ranks 10, 13 | Cap0 done; C blocked_claim; tip_bind closer ok |
| T4 Full priority plan + LIVE canvas | this plan | N=80 visible; Cap0-done vs Root-open; no truncation |
| Cap0∥ remaining | — | **none** — wait Root AT/AUTH + Cap realloc |

## 6. Cap=1 serial (HOLD) — Decision B

Hold until Root AUTH/AT + Cap realloc → Cap free=true. Then **one** burn:

1. Preferred: product Cap path with AUTH/AT present (rank 14 residual; PM feeder already spent as component)
2. Alternate: Unlock-B --w2-h2-product-binding once (rank 15) **only after** AUTH/AT — Cap1 alone cannot close product
3. Forbidden: ranks 73–80 Cap twin / spam paths **including spent PM twin**

Decision B (2026-11-09T22:05:45Z tip ad8b68a2): Cap burn=0; no Cap0/Cap-ready feeder
closes product cells without AUTH/AT/BrowserReady.

## 7. Explicit non-claims

- Unlock A∨B true ≠ Cap free ≠ BrowserReady ≠ product green
- tip_bind.ok=true ≠ pass_product_clean_room
- Cap1 CR/G2/G5/G6/PM EXIT0/EXIT2 = component topology / feeder only
- e2e EXIT0 desktop oracle ≠ Cap realloc ≠ pass_product
- CEF Cap-day bind ≠ Cap free ≠ BrowserReady
- pressure-guard Cap=0 ≠ Cap realloc
- Full table completeness ≠ product completion
- Cap0∥ inventory done ≠ goal complete
- No forge AUTH/AT; no invent Cap free / BrowserReady / pass_product

## 8. Exact next closer (Cap0 Decision B CONSOLIDATED)

Root AT-HANDOFF (`humanymous.root-interactive-at-windows-tuple/1`) + AUTH-HANDOFF
(`humanymous.root-n06-owner-join-auth-handoff/1`) → Cap=0 apply BrowserReady/N06 →
documented Cap realloc (NEW structural feeder, not spent G2/G5/CR/G6/RB/PM twin) →
Cap free=true → Cap=1 product path toward `pass_product_clean_room`. Cap burn=0 until then.

Receipt: `.spec.local/root/evidence/20261110070300-cursor-product-blockers-cap0/`

## 9. Wiring

| Artifact | Path |
| --- | --- |
| This plan | .spec/plan/20261110061400-full-remaining-priority-no-cutoff.plan.md |
| Cap=0 evidence | .spec.local/root/evidence/20261110061400-cursor-full-priority-no-cutoff/ |
| Cap0∥ wave | .spec.local/root/evidence/20261110063500-cursor-full-cap0-parallel-wave/ |
| Cap0 reaudit wave | .spec.local/root/evidence/20261110072000-cursor-cap0-reaudit-wave/ |
| Cap0 Cap1-hold landers | .spec.local/root/evidence/20261110075800-cursor-cap0-cap1hold-landers/ |
| Cap0 Cap-free-restore | .spec.local/root/evidence/20261110080500-cursor-cap0-cap-free-restore/ |
| Cap0 Cap-free-apply CLI | .spec.local/root/evidence/20261110082900-cursor-cap0-cap-free-apply/ |
| Cap0∥+Cap1serial goal | .spec.local/root/evidence/20261110083200-cursor-parallel-serial-goal/ |
| SoT/code honesty sync | .spec.local/root/evidence/20261110080900-cursor-sot-code-sync/ |
| CONSOLIDATED closer | .spec.local/root/evidence/20261110083200-cursor-parallel-serial-goal/ |
| Prior tipbind readiness | .spec.local/root/evidence/20261110074900-cursor-cap0-tipbind-readiness/ |
| Prior Decision B closer | .spec.local/root/evidence/20261110070300-cursor-product-blockers-cap0/ |
| Prior Cap0∥ closer | .spec.local/root/evidence/20261110064200-cursor-cap0-consolidated-closer/ |
| Supersedes truncated tables | plan:root/20261110060500-code-sot-gap-browser-priority-reorder, plan:root/20261110044500-production-status-priority-reorder |
| Matrix | .spec.local/root/evidence/20261108164500-cursor-clean-room-matrix/ |
| LIVE canvas | canvases/humanymous-live-roadmap.canvas.tsx |

## LIVE PM Cap wave honesty (2026-11-09T22:00:03Z)

| Field | Measured |
| --- | --- |
| tip (pre-commit) | 7dbd697080621c744b331d2528a1844826bf8c35 |
| Cap free before | **false** |
| Cap free after realloc | **true** |
| Cap free after Cap=1 burn | **false** |
| Cap realloc | **yes** `humanymous.cursor-clean-room-product-matrix-cap-realloc/1` |
| Cap EXIT | **2** (require-full feeder not product) |
| Cap burn | **1** once `--clean-room-product-matrix --require-full` |
| Structural feeder | NEW product-matrix Cap CLI+topology (not spent CR twin) |
| product / pass_product_clean_room | **0** / **0** |
| BrowserReady / AUTH / AT | **false** / absent / absent |
| Rank 9 U-CAP-REALLOC | Cap0 feeder landed + realloc executed this wave |
| Rank 14 U-CAP1-CR-ONCE | Cap1 preferred path burned via product-matrix feeder; product still 0 |
| goal_complete | **false** |

## Decision B Cap=0 hard closer (2026-11-09T22:05:45Z)

| Field | Measured |
| --- | --- |
| tip | ad8b68a21d22e83477b00eba08c36ddcc6f78e99 |
| decision | **B** |
| Cap burn | **0** |
| Cap EXIT | **none** |
| Cap free | **false** |
| product / pass_product_clean_room / BrowserReady | **0** / **0** / **false** |
| AUTH / AT wake | absent / absent |
| EXIT2 missing product cells | **47** (all counts_as_product=false) |
| product_matrix_open_cells | **16** (all need AUTH/AT/BrowserReady or Root structural / Cap free) |
| Cap0∥ remaining | **none** |
| exact next closer | Root AT-HANDOFF + AUTH-HANDOFF → Cap0 apply → Cap realloc (NEW non-twin) → Cap=1 product |
| receipt | `.spec.local/root/evidence/20261110070300-cursor-product-blockers-cap0/` |
| goal_complete | **false** |

