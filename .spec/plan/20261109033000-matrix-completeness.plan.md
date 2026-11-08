---
id: plan:root/20261109033000-matrix-completeness
title: Cap=0 clean-room matrix completeness gate (tip a984a871)
status: active
created: 2026-11-08T18:35:00Z
updated: 2026-11-08T19:05:00Z
rel:
  depends_on:
    - plan:root/20261108221000-clean-room-matrix-cap-driver
    - sot:root/container-test-environments
    - sot:root/program-roadmap
  governs:
    - test/lab/runner/src/clean_room_matrix.rs
---

# Cap=0 matrix completeness

Brief: Cap=0 `matrix-completeness` gate enumerates every SoT-required
usage-condition cell with an honest blocker. Tip rebound to `6ba329ad` with G2
Cap=1 component bind (`pass_component_desktop_cap_topology`) and G5 Cap=1
component bind after Desktop baseline Cap realloc. Cap free=false. product=0.
Never invent `pass_product_clean_room` / Cap free / authenticated transport.

## Decision

R1. Cap realloc without live topology/GRANT is **not** authorized (documented NO).
R2. Completeness gate Cap=0 EXIT0; missing SoT scenarios fail closed.
R3. G2/G5 matrix cells added as `blocked` / `root_grant_*` — not Cap CLI twins.
R4. Optional Cap-ready SoT path-release pack for new
    `clean-room-usage-matrix.sot.md` (no foreign SoT overwrite).

## Verification

- `cargo test --locked -j1 -p humanymous-lab-runner clean_room_matrix`
- `matrix-completeness --matrix … --expect-head a984a871` EXIT0 · product=0

## Residual

G2+G5 Cap=1 Cap topology components bound (≠ product). AUTH/AT still separately
required for N06/BrowserReady.
