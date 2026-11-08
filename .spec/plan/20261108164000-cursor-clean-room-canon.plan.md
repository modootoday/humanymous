---
id: plan:root/20261108164000
title: Cursor SoT Docker clean-room completion authority
status: active
created: 2026-11-08T16:40:00Z
updated: 2026-11-08T16:40:00Z
rel:
  depends_on:
    - sot:root/container-test-environments
    - sot:root/structural-development
    - sot:root/delivery
    - plan:root/20261108163300
---

# Cursor SoT Docker clean-room completion authority

## Brief

Strengthen product-completion SoT so authority is explicitly **Docker
clean-room** for all usage-condition combinations. Host/loopback e2e remains
non-authority. Point the product-completion matrix path as inventory only until
cells are `pass_product`. Detection freeze; no invented M3 green.

## Non-goals

- Detection scoring, thresholds, hard rules, verdict bands
- Editing foreign-owned matrix bytes under root-coordination
- Editing `deployments/lab/ENVIRONMENTS.md` (e2e-infra claim)
- Running Docker product e2e / claiming M3 or `pass_product`
- Public Diátaxis rewrites; remote push

## Measured clean-room definition (applied)

1. Affinity-checked Docker Engine (Desktop `desktop-linux` or genuine hosted
   Linux); unallocated WSL fallback forbidden.
2. L5 lab topology: internal nets, role separation; host coordinates Docker
   only.
3. Owned Compose project prefix `humanymous-<family>-*`; never
   `humanymous-pages-preview`.
4. Coordinator: `humanymous-lab-runner` / red-blue Docker suite with exact
   source/image/topology binding and owned cleanup.
5. act cannot mint hosted product qualification.
6. Forbidden hosts: `node test/e2e/runner.mjs`, `node test/gate/e2e.mjs` as done.
7. Desktop component pilots ≤ `pass_component_only`.
8. Matrix
   `.spec.local/root/evidence/20261108163200-cursor-product-completion-matrix/`
   is inventory until required cells are `pass_product`.

## Decisions (applied)

1. Expand `container-test-environments` product-verification canon with the
   clean-room table and matrix inventory pointer.
2. Cross-link `structural-development` L5 honesty and `delivery` product e2e
   authority to the same clean-room / `pass_product` vocabulary.
3. Leave program-roadmap M3 “Not qualified” intact; zero invented green.

## Verification

- Branch `codex/rust`; session `cursor-integrator` / lane `sot-plan`
- `spec check --no-local` on touched SoT/plan paths
- Receipt:
  `.spec.local/root/evidence/20261108164000-cursor-clean-room-canon/`
