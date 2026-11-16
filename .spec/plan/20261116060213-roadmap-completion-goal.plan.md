---
id: plan:root/20261116060213
title: Roadmap completion goal and unattended execution contract
status: active
created: 2026-11-16T06:02:13Z
updated: 2026-11-16T10:55:00Z
rel:
  depends_on: [sot:root/milestones, sot:root/program-roadmap, plan:root/20261110061400, sot:root/clean-room-usage-matrix]
---

# Roadmap completion goal and unattended execution contract

## Goal

**goal:** reach `goal_complete=true` as defined in [[sot:root/milestones]]
(FM1–FM11 exit gates, Go↔Rust differential parity, Docker clean-room `pass_product_clean_room` for every
clean-room-required cell, Docker `red-blue-validate`, IM5 and IM6).

The user set this goal on 2026-11-16 and asked for unattended execution. An
unattended agent may only do what it can finish **without forging authority**.
The contract below says what that means in practice.

### Unattended execution contract

1. **Do:** work that one agent can finish and prove inside its own environment
   (class A below). Examples: canon and Specfold hygiene, rustfmt/Clippy/test
   gates, removing stale assertions, evidence receipts, re-measuring with Cap0
   status tools.
2. **Never:**
   - forge or template AT-HANDOFF, AUTH-HANDOFF, Cap-free GRANT, TRANSFER,
     BrowserReady or `pass_product_clean_room`;
   - burn Cap=1 while `cap_free=false`;
   - change detection scoring (freeze);
   - skip or weaken a test to get green;
   - publish, tag, exchange OIDC tokens, merge to main or push without an
     explicit permission that is in force.
3. **Stop** when every remaining unit is in class D, R, H or P. Report each
   blocker with the exact artifact or permission that would unblock it. Do not
   loop or poll while nothing external can change.
4. Every status change in [[sot:root/milestones]] cites a new evidence document.

### Blocker classes

| Class | Meaning | Who unblocks |
| --- | --- | --- |
| A | Autonomous: one agent can finish and prove it here | the agent |
| D | Needs a Cap=1 Docker product run; the Cap-free GRANT and a matching TRANSFER must exist first | Root |
| R | Needs Root or human authority (handoffs, dual-control, structural ownership, reviewed dispositions) | Root / owner |
| H | Needs hardware or an OS that this container lacks (Windows CEF + interactive AT, macOS, GPU, native tuples) | owner with that host |
| P | Needs ref or publication authorization (push, main merge, Pages deploy, release) | user |

## Steps

Full unit list. Ranks follow [[plan:root/20261110061400]]. Only A units run
unattended.

| # | Unit | Milestones | Class | Unblocking artifact / action |
| ---: | --- | --- | --- | --- |
| 1 | Specfold canon: 0 errors, 0 warnings in tracked `.spec` | FM0 | A | done 2026-11-16 |
| 2 | Workspace rustfmt clean; session scripts executable | FM0, FM11 | A | done 2026-11-16 |
| 3 | Stale lane-count assertion | FM11 | A | done 2026-11-16 |
| 4 | Re-measure Cap0 board at the current tip | IM3 | A | done 2026-11-16 (`until-complete-status`, `cap0-chain --inspect-only`) |
| 5 | AT-HANDOFF (interactive AT Windows tuple) | B2, G4 | R+H | Root places `humanymous.root-interactive-at-windows-tuple/1` from a real Windows AT session |
| 6 | CEF pilot bundle (`HUMANYMOUS_CEF_PILOT_BUNDLE`) | B2 | H | owner with the Windows CEF build host |
| 7 | BrowserReady measured join | B2, G4 | R+H | follows 5 and 6; `handoff-apply --kind browser-ready-join` |
| 8 | AUTH-HANDOFF and N06 owner-join | FM4, FM5 | R | AUTH-HANDOFF recorded 2026-11-16 on owner approval; measured join still needs the hosted venue |
| 9 | Cap-free-restore GRANT and live-matching TRANSFER (Unlock C) | IM3 | R | Root issues GRANT.json and TRANSFER manifest matching live digests |
| 10 | `crates/apps/humanymous-browser` package landing (3 → 1) | IM1, B0 | R | Root structural path release |
| 11 | Adapters package consolidation (12 → 1) | IM1, FM6 | R | Root structural path release |
| 12 | Cap=1 clean-room product run and product matrix | IM3, FM8 | D | follows 5–9 |
| 13 | Docker `red-blue-validate` (product) | IM3 | D | follows 12 |
| 14 | G1 TLS/H2 capture and session binding | FM2 | R | Root structural delta (network-capture SoT) |
| 15 | G2 session-bound collector product | FM5 | D+R | follows 8, 9 |
| 16 | G3 Pass recovery | FM1 | R | Root only; detection freeze |
| 17 | G5 authenticated witness transport product | FM3 | D | follows 9 |
| 18 | G6 L5 origin and audit binding product | FM3 | D | follows 9 |
| 19 | Dual-control erasure / kill switch / bans | FM3 | R | two human controllers by design |
| 20 | G7 remote attestation, G8 native OS tuples | FM10, B6 | H | after IM3 |
| 21 | G9 provider deployment | FM9 | P | publication unauthorized |
| 22 | Adapter home pilots (Apache, Python, Ruby, .NET, native) | FM6 | D | follows 9 |
| 23 | Cross-OS full runners (Windows, macOS, Linux six-tuple) | FM10, IM2 | H | hosted runners |
| 24 | Legacy Go: 314 rows reviewed (104 ported_unit, 90 partial, 118 missing, 2 n/a); each product row needs a Rust port with actual evidence. No per-row retirement (owner decision 2026-11-16) | FM11 | A done; residual R/H/P | review done 2026-11-16, [[evidence:root/20261116071500]]; class A sweep 2026-11-16 ported every authorized gap (163 ported_unit, 97 partial, 52 missing); each remaining row is classified with a cited blocker in `.agents/retirement/fm11-class-a-sweep.md` |
| 25 | Go Gate/Core image builders migrate to Rust images | FM11 | R+P | release lane; parity evidence comes from 12–13 |
| 26 | CVE, SBOM and image scan | FM11 | A after 25 | runs once images are final |
| 27 | Pages and Draft v0 current-source review | IM4 | R+P | manual review and deploy approval |
| 28 | IM5 main integration | IM5 | P | user authorizes merge after 1–27 |
| 29 | IM6 main-only and approved Pages | IM6, FM12 | P | follows 28 |
| 30 | Push of agent commits on `codex/rust` | — | P | user approves push (AGENTS.md reserves ref work for Root) |
| 31 | TLS drain refusal-propagation race (flaky gateway TLS test) | FM4, IM2 | R | `cursor-integration` owner decides code vs test under the runtime-application drain contract |
| 32 | Make the lab-runner Unlock-A test hermetic (it now needs an untracked private closer) | IM2 | R | product-lab with Root: fixture or explicit ignore with a reason |
| 33 | `git_coord` test needs `pwsh`; Rust replacement of the coordination scripts | FM11 | R | agents-meta owner; scripts stay until replaced and qualified |
| 34 | Workspace Clippy `-D warnings` clean | FM0 | A | done 2026-11-16 |
| 35 | Real Rust Docker product e2e harness (Gate + origin + authority + audit + browser + attacker), replacing the scaffold product labs | IM3, FM8 | A done (increments 1–5); residual R+H | partial topology measured in Docker (Gate, origin, attacker, witness, health, real browser and WASM collector, DNS canary); residual needs Authority as a service (Gate decides in-process), the clean-room loopback policy, a Unix witness input for the CLI host (R1/R3), a Linux secret provider (needs-human) and a readiness SoT delta ([[evidence:root/20261116114735]]) |
| 36 | Retire scaffold `*-cap-pilot.sh` shell pilots | FM11 | R (Cap) | the pilots are wired into the Cap0 chain, transfer applicator and snapshots; five stand in for components 35 cannot run yet |
| 37 | Go↔Rust differential parity harness: golden vectors recorded from the frozen Go baseline, compared against Rust outputs in CI | FM1, FM11 | A (harness); R (approving an intended difference) | definition-of-done item 5 in [[sot:root/milestones]] |
| 38 | Merge `codex/rust` into `main` and delete the Go tree in the same change | IM5, FM12 | P | owner approval once items 1–5 hold |

## Record

- 2026-11-16: goal set by the user. Class A units 1–4 were done in this session.
  Units 5–30 are D, R, H or P; the unattended run stops there. The fresh Cap0
  probe at this tip confirms the blockers: `open_incomplete_ids` = AT-HANDOFF,
  AUTH-HANDOFF, Cap-free-restore-GRANT, BrowserReady, pass_product_clean_room,
  docker_clean_room_full_matrix_green, red_blue_validate_product,
  docker_e2e_validate_product, Unlock-C, apps_humanymous_browser. Also product
  0, `cap_free` false, `cap_burn` 0. Gate results are in
  [[evidence:root/20261116055601]] and [[evidence:root/20261116060831]].
- 2026-11-16: the unattended run stopped after class A units 1–4 and 34. Every
  open unit (5–33) is D, R, H or P.
- 2026-11-16: the owner confirmed that no other session is active, approved
  push, and as Root approved AUTH-HANDOFF and Cap-free. AUTH-HANDOFF was
  recorded. Cap-free could not be applied without forging the absent Unlock A
  closer, and the product labs turned out to be scaffolds (units 35–36). Units
  31 (TLS drain race) and 32 (hermetic Unlock A test) were fixed. The owner chose
  FM11 (unit 24) as the next structural work.
- 2026-11-16: FM11 row review landed as
  `.agents/retirement/legacy-go-row-review.json`. No row is retired; the
  missing and partial rows are now explicit port or retirement work.
- 2026-11-16: 158 force-added `.spec.local` files were untracked after a
  consumer measurement showed no test or gate change, and CI `delivery policy`
  now refuses tracked private evidence ([[evidence:root/20261116081000]]).
- 2026-11-16: the owner decided that `codex/rust` merges into `main` when Rust
  gives results symmetric to Go, and Go is retired at that merge. They chose a
  differential parity gate as the measure (units 37–38). Unit 24 no longer
  includes per-row retirement.
- 2026-11-16: unit 35 increments 1 and 2 ran the real Rust Gate in Docker
  with a recording origin, an attacker, an independent audit witness, health
  checks and dependency loss after readiness; every assertion passed
  ([[evidence:root/20261116095026]], [[evidence:root/20261116103304]]). The
  topology is partial and not clean-room. Authority cannot run as a separate
  container (the Gate decides in-process), the clean-room L5 split needs an
  owner decision on the loopback-only listen and origin checks, and the
  browser and WASM collector are not wired. Unit 36 stays open: the scaffold
  pilots are wired into the Cap0 chain, transfer applicator and snapshot code,
  and five of them stand in for components unit 35 does not cover yet.
- 2026-11-16: unit 35 increment 3 drove a real Firefox through the Rust
  `wasm-bindgen-test-runner` and ran the real Rust WASM collector against a
  witnessed runtime Gate in Docker. Session, collect, CHALLENGE and 422 paths
  answered as the fixtures expected, Origin and Referer held, and the audit
  stayed fully witnessed ([[evidence:root/20261116110546]]). Readiness now
  turns unhealthy after witness loss, but only after the first failed append.
  Still open: Authority, the shipped binary's witness and live readiness,
  original client TLS, a DNS canary, and the clean-room split.
- 2026-11-16: unit 35 increment 4 added an owned DNS canary (zero browser
  arrivals, positive controls answered) and replaced the client-asserted
  collect fixtures with the real collector's captured report
  ([[evidence:root/20261116113042]]). Outcomes matched increment 3. The engine
  answered COLLECTED for a report carrying a webdriver BOT signal; that is an
  owner observation under the detection freeze, not a harness change.
- 2026-11-16: unit 35 increment 5 tried to run the shipped CLI runtime host
  as the witnessed Gate and stopped before Docker
  ([[evidence:root/20261116114735]]). The Unix host has no witness input and
  non-Windows secret providers return Unsupported, so it needs a Linux
  secret provider (needs-human) and R1/R3 review; live diagnostic readiness
  needs a SoT delta. It fixed the Unix `runtime-service` build (E0107).
- 2026-11-16: units 24, 35 and 36 reclassified in the table. The class A parts
  are done (unit 35 increments 1–5; the FM11 class A sweep on PR #21, with
  all 90 mapped Go gate wargame tests passing at 981d25c). Every residual has
  a cited R, H or Cap blocker, consolidated in the needs-owner issue.
- 2026-11-16: unit 37 harness landed. `scripts/ci/parity-surfaces.json` binds
  27 Go vectors to the baseline, the Rust parity tests and the FM11 packages,
  and `delivery policy` enforces it ([[evidence:root/20261116090000]]). 25
  packages remain named gaps. Unit 35 is blocked: the remote product runner
  needs a signed-nonce installation identity in the runtime (runtime SoT delta,
  class R) and running it is class D.
