---
id: plan:root/20261109033000-clean-room-usage-matrix
title: Canonize clean-room usage-matrix SoT from Cap=0 completeness gate
status: accepted
created: 2026-11-08T18:35:00Z
updated: 2026-11-08T19:10:00Z
rel:
  depends_on: [sot:root/clean-room-usage-matrix]
  governs: []
---

# Plan — clean-room usage-matrix SoT

Cap=0 after Cap-ready GRANT + create-only path-release. product=0.

1. Applied ROOT-ONE-STEP grant for `.spec/sot/clean-room-usage-matrix.sot.md`.
2. Cursor claimed path; Cap=0 vocabulary refresh from COMPLETENESS tip `b3bce0c0` then tip-bind `10de63c1`.
3. Cap=0 CR Cap CLI + Cap-ready topology + GRANT/TRANSFER + Cap realloc + Cap=1
   `--clean-room-product` EXIT0 component bind (evidence
   `20261109040100-cursor-clean-room-product-cleanup-realloc`); Cap free=false.
4. Never invent pass_product_clean_room / Cap free / BrowserReady / N06 join /
   AUTH-HANDOFF / AT-HANDOFF.
