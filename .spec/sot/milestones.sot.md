---
id: sot:root/milestones
title: Canonical milestone register
status: active
created: 2026-11-16T05:56:01Z
updated: 2026-11-16T05:56:01Z
verify: ["spec check --no-local"]
rel:
  depends_on: [sot:root/program-roadmap, sot:root/clean-room-usage-matrix, sot:root/rust-migration]
---

# Canonical milestone register

## Rule

This document is the **only** canonical register of humanymous milestone
identifiers, scopes, statuses and exit gates. Other SoTs, plans, evidence and
public docs cite milestone ids from here. They must not keep a competing status
table. Ordering and dependency rationale lives in
[[sot:root/program-roadmap]]. Priority lists inside one wave live in the current
active priority plan ([[plan:root/20261110061400]]). The goal, its blocker
classes and the unattended execution contract live in
[[plan:root/20261116060213]]. Dated sections in the
roadmap and earlier plans are historical snapshots.

### Goal and definition of done

`goal_complete` stays **false** until every item below holds on one exact final
candidate source:

1. FM1–FM11 exit gates are met. A gate can be met with actual evidence or with
   a reviewed, recorded disposition, but never by deleting the requirement.
2. Docker clean-room `pass_product_clean_room` is true for every
   clean-room-required cell of [[sot:root/clean-room-usage-matrix]].
3. `red-blue-validate` passes in Docker only.
4. IM5 main integration and IM6 main-only state are verified.

Package publication, release tags and OIDC token exchange are not part of done.
They stay unauthorized unless the user authorizes them separately.

### Identifier namespaces

Earlier documents reused the bare label `M1…` for two different series. These
are the canonical names:

| Canonical | Series | Historical aliases |
| --- | --- | --- |
| FM0–FM12 | Feature milestones | roadmap "Goal-completion milestones" `M0–M12`; priority plan `U-FMn` |
| IM1–IM6 | Integration gates (serial, final) | roadmap "Final integration milestone order" `M1–M6`; priority plan `U-Mn` |
| B0–B6 | Browser / native tool track | unchanged |
| PA–PH | Program phases | roadmap phases `A–H` |
| W0–W5 | Execution waves (dependency order, not dates) | unchanged |
| G1–G9 | FullProduct blocker classes | unchanged; owned by IM3 |

New documents must use the canonical names. Historical text is not rewritten.

### Status vocabulary

| Status | Meaning |
| --- | --- |
| Implemented | Exit gate met with source-bound actual evidence |
| Partial | Source exists. Some, but not all, exit-gate parts have actual evidence |
| Component-only | Only component or fixture results exist; promotion to product is forbidden |
| Research | Design or draft only |
| Not started | No qualifying work |
| Deferred | Intentionally ordered after another milestone |

A status may rise only when a cited evidence document proves the exit gate
(no folklore). Unit tests, host or loopback runs, and mocks never raise a status
past Component-only for runtime, product or native scope.

### Feature milestones

| ID | Scope | Status | Exit gate (remaining) |
| --- | --- | --- | --- |
| FM0 | Branch, Specfold and development rules | Implemented | Maintain `.spec`, attribution and evidence; final agents cleanup moves to FM11 |
| FM1 | Pure engine, model and Pass | Partial | All original regressions, ML lifecycle, full Pass, actual runtime parity (detection freeze) |
| FM2 | Network / raw capture | Partial | TLS ClientHello, ordered headers, HPACK/H2 settings and priority, connection and session binding (G1) |
| FM3 | Audit, keys, state | Partial | Independent durable witness, operational rotation, dual-control, backup, erasure and sinks (G5, G6) |
| FM4 | Core / Gate and authority | Partial | All actual collector, capture and RIT paths, plus proxy, settings, abuse and admin paths |
| FM5 | Rust collector | Partial | Guard hooks/CDP, fingerprint-ID/ICE, loader, session, RIT, recovery, Pass and API matrix; session-bound product binding (G2) |
| FM6 | Server adapters | Component-only | Peer, route, header, body, stream, provenance and actual authority per host; one consolidated adapters package |
| FM7 | Operational CLI and installation | Partial | Signed package → service activation and readiness → update and rollback; native peer negative pilots |
| FM8 | Product Docker wargame | Component-only | Live Gate, browser and Pass, egress, origin and audit, attack/ordinary/human/AT denominators |
| FM9 | Automation, mobile, provider and standards | Research | Owned execution, wire format and vectors, interoperability, security/privacy/a11y review |
| FM10 | Cross-OS, OIDC and Pages | Partial | Three full runners, final-source six-tuple, registry and package trust, docs freshness, Pages artifact |
| FM11 | Rust-only, supply chain, agents and skills | Partial | Each of the 314 legacy Go responsibility rows has exact Rust behavior with actual evidence, or a reviewed retirement disposition. Go image builders migrate. CVE, SBOM and image scan. Agents and skills evaluation |
| FM12 | Retain only final main | Deferred | Follows IM5 and IM6 |

### Browser and native tool track

| ID | Scope | Status | Exit gate (remaining) |
| --- | --- | --- | --- |
| B0 | Browser design | Partial | Pin and provenance |
| B1 | Engine and profile | Partial | Permission, network and lifecycle qualification |
| B2 | Meari + CEF UI | Partial | Sandbox, input, IME, AX, DPI, crash, OS tuples, capability matrix; BrowserReady (G4) |
| B3 | Diagnostics and receipt | Not started | Qualified target and input; typed collector/session/capture/audit join |
| B4 | Controlled experiments | Not started | Ordinary Chrome, human/AT, observer on/off, false positives |
| B5 | Tool distribution | Not started | Native CVE/SBOM, signing, update and rollback, user docs |
| B6 | Native SDK, framework, CLI | Partial | Versioned ABI consumer, owned lifecycle and unload, framework matrix, scaffold/doctor/verify |

### Integration gates (serial)

| ID | Scope | Status | Exit gate |
| --- | --- | --- | --- |
| IM1 | Shared contracts and the final eight-package structure | Partial | Live graph moves from 29 members to the accepted eight packages; native owner, context, epoch and identity boundaries; source-bound units |
| IM2 | Component runtime qualification | Partial | Current-source adapter, CEF and Cross-OS actual pilots; failures, unknown and ignored cases resolved or explicitly outside the contract |
| IM3 | Composed product e2e | Component-only | G1–G9 closed. Docker clean-room product cells pass, with source/image binding and exact cleanup |
| IM4 | Pages and Draft v0 | Partial | Current-source reviews, fresh builds, browser and AT validation; no unsupported public claims |
| IM5 | Shared main integration | Not started | IM1–IM4 met on one exact candidate, then integration and post-merge verification |
| IM6 | Main-only and approved Pages | Not started | Integrated main verified, approved serving, then authorized branch and file retirement |

### Crosswalk

| Phase | Covers |
| --- | --- |
| PA architecture and ownership | FM0, IM1 (contracts) |
| PB Core/Platform/Runtime foundations | FM1–FM4 |
| PC Tools integration | FM7, FM10 |
| PD Adapters and Browser | FM6, B0–B6 |
| PE Final eight graph and shipping roles | IM1, IM2 |
| PF Public docs and delivery | IM4, FM10 |
| PG Main integration | IM5 |
| PH Final Pages and branch cleanup | IM6, FM12 |

Waves W0–W5 order this work as in [[sot:root/program-roadmap]]. W3 is IM3 and
W5 is IM5–IM6.

## Why

Before this register, milestone status lived in at least three places: the
roadmap goal-completion table, the roadmap integration-order table, and the
priority plans. Two of them reused the same `M1–M6` labels for different
meanings, and the plans renamed them again (`U-Mn`, `U-FMn`). A single register
with namespaced ids removes that ambiguity. It also makes every status change a
reviewable edit bound to evidence.

The statuses above carry forward the last recorded roadmap tables unchanged,
with these normalizations. FM6, FM8 and IM3 ("Partial/research", "Limited
pilot", "Not qualified") are worded Component-only to match their
recorded evidence boundary. B3, B4 and B5 use Not started for the recorded
"Unimplemented" and "Not run". The other recorded labels ("In progress",
"Bounded runtime", "Provisioning implemented", "Scoped verification/wiring",
"Design/partial", "Partial on Windows") map to Partial, and "Held by M1-M4"
maps to Not started. The 2026-11-16 audit
([[evidence:root/20261116055601]]) found nothing that raises a status. It
confirmed `goal_complete=false`.

## Verification

- `spec check --no-local` passes with zero errors.
- Each status raise cites a new evidence document id in the same change.
- No other current SoT or plan carries its own milestone status table; the
  roadmap points here.
- The latest product board lives in the roadmap's LIVE section and the
  clean-room matrix. This register cites it and never copies product counts as
  its own claim.
