---
id: plan:root/20261109033000-matrix-completeness
title: Cap=0 clean-room matrix completeness gate (tip a984a871)
status: active
created: 2026-11-08T18:35:00Z
updated: 2026-11-08T18:35:00Z
rel:
  depends_on:
    - plan:root/20261108221000-clean-room-matrix-cap-driver
    - sot:root/container-test-environments
    - sot:root/program-roadmap
  governs:
    - test/lab/runner/src/clean_room_matrix.rs
---

# Cap=0 matrix completeness

Brief: While G2/G5 GRANT + AUTH/AT remain absent and Cap free=false, Cursor lands
a Cap=0 `matrix-completeness` gate that enumerates every SoT-required
usage-condition cell with an honest blocker, tip-bound to settled `a984a871`.
No Cap burn. product=0. Never invent `pass_product_clean_room` / Cap free.

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

Root G2 GRANT (prefer) → TRANSFER-apply → Cap realloc → Cap=1 once; AUTH/AT
still separately required.
