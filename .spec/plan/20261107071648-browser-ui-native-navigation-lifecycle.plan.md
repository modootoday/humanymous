---
id: plan:root/20261107071648
title: Native browser navigation lifecycle and strict probe closure
status: active
created: 2026-11-07T07:16:48Z
updated: 2026-11-07T07:16:48Z
rel:
  governs: [crates/browser/**]
  depends_on: [sot:root/browser-ui, plan:root/20261107050329]
---

# Native navigation lifecycle and strict probe closure

Goal active: implement actual Rust Meari/CEF host lifecycle, permissions, input and
accessibility with supported-tuple evidence. REA stays disabled; no new browser
research, local process/runtime/CEF execution or duplicate dispatch. Shared
Cargo/lock/toolchain/push belongs to root. Central execution policy remains
sot:root/program-roadmap and sot:root/delivery.

- [x] Claim browser-only source/spec paths after product-lab owner released misc;
  root-approved transition to the formal browser-ui lane completed.
- [x] Inspect existing seams; accept UI-25..27 and record same-URL attribution limit.
- [x] Implement one pure ticket/terminal state owner and ordered seven-step probe.
- [x] Wire native task rejection, unavailable frame, revoke/closed callbacks and
  strict report/exit closure; collect failure outcomes and receipt-loss count.
- [x] Bound owner/query/resource parsing before allocation; reset canceled IME.
- [x] Run only root-assigned resource-bounded pure unit/fmt/scoped Clippy checks.
- [ ] Obtain source-bound hosted compile/native safe URL/lifecycle/failure tests;
  required unsupported GUI/IME/AT/GPU/DPI capability remains not_run.
- [x] Register exact public/private evidence and coordinated owned-path checkpoint
  for implemented source/pure units; hosted qualification remains open.

Review: P0 cleanup-only exit0 on failed/missing internal probe is rejected by
UI-26; P1 post-task failure left requested state and missing closed guards are
covered by UI-25; URL/parser and canceled composition bounds by UI-27. Residual
same-URL engine callback attribution is explicit and grants no authority. Do not
claim native compilation or a completed browser from pure unit success.

Dependency owners: root assigns the sole local Cargo slot and remote source
closure/dispatch; CI/native-runner owners provide suitable hosted execution and
raw recovery policy. Product-lab ownership is deployments/test-lab only. This
browser writer edits no shared manifests, CI workflow or other owner's canon.

Implementation checkpoint is source-only until assigned pure-unit and hosted gates
execute. Report revision2 adds requested/finished/observed/passed and receipt loss;
older schema/missing fields remain unknown. Native task/source ticket protection
does not resolve same-URL engine callback attribution. No local native build or
CEF/runtime execution has occurred in this slice.


Hosted runner seam (not_run): positive --native-internal-probe must supply report
navigation_schema2, requested/finished/passed true, observed7, dropped0 and real
before_close/host_closed. Also combine --native-cancel-before-created with the
probe flag: expected nonzero with explicit incomplete probe, while cleanup must
still be measured separately. Do not add a synthetic expected-failure flag that
returns the desired code without exercising actual owned lifecycle. These cases
need CI/native-runner owner execution after root source closure; no duplicate
dispatch and no CEF workflow edits by the browser owner.

Source-only evidence:root/20261107073932 records actual formatter/spec/whitespace
checks and zero executed unit/native tests. Linked change:root/20261107073933
records all governing historical plan deltas without changing root-owned canon.
Goal and verification queue remain active; no completion claim.

Root-assigned A/shared target completed default pure23/23 and Clippy with jobs1/
incremental0. First missing-field-rustdoc Clippy failure and corrected rerun are
preserved separately in evidence:root/20261107073932. Native adapter compile and
all actual hosted native/GUI/IME/AT/GPU/DPI gates remain not_run; goal active.


Root expanded the exclusive writer scope to crates/native-cli (scaffold consumer),
not operational crates/cli. Source helper cfg now matches its Windows-only library
consumer; integration process helpers are unchanged. Existing native-embedding
SoT/plan07042614 still own its API contract. New hosted Linux Clippy is not_run;
no Cargo during root's graph window. Track that source fix separately from the
23 passing browser pure units, which did not compile/test native-cli.
