---
id: plan:root/20261116060213
title: Roadmap completion goal and unattended execution contract
status: active
created: 2026-11-16T06:02:13Z
updated: 2026-11-16T06:02:13Z
rel:
  depends_on: [sot:root/milestones, sot:root/program-roadmap, plan:root/20261110061400, sot:root/clean-room-usage-matrix]
---

# Roadmap completion goal and unattended execution contract

## Goal

**goal:** reach `goal_complete=true` as defined in [[sot:root/milestones]]
(FM1–FM11 exit gates, Docker clean-room `pass_product_clean_room` for every
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
| 24 | Legacy Go: 255 pending + 59 mapped rows need Rust proof or a reviewed disposition | FM11 | R | the review owner named on each row in `.agents/retirement/legacy-go-owner-routing.json` |
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
| 35 | Real Rust Docker product e2e harness (Gate + origin + authority + audit + browser + attacker), replacing the scaffold product labs | IM3, FM8 | A (large) | none; the current product labs `cat` fixed receipts ([[evidence:root/20261116062411]]) |
| 36 | Retire scaffold `*-cap-pilot.sh` shell pilots | FM11 | A after 35 | replaced by 35 |

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
