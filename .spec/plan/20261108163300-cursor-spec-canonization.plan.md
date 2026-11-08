---
id: plan:root/20261108163300
title: Cursor SoT product-verification canonization
status: active
created: 2026-11-08T16:33:00Z
updated: 2026-11-08T16:33:00Z
rel:
  depends_on:
    - sot:root/container-test-environments
    - sot:root/structural-development
    - sot:root/delivery
    - sot:root/adapter-authority
    - sot:root/runtime-application
    - sot:root/cli-installation
    - plan:root/20261108155700
---

# Cursor SoT product-verification canonization

## Brief

Continue SoT honesty after package-path Waves 1–2b (`f40b43c1`, `4cc1e9a0`,
`48124c14`). Align product-verification wording and remaining live path IDs to
measured Cargo/lab layout without inventing product/M3 green. Detection freeze.

## Non-goals

- Detection scoring, thresholds, hard rules, verdict bands
- Editing foreign dirty `convention-audit` (red-catalog loan; document only)
- Overwriting coordinator-owned roadmap/migration index narratives
- Running or claiming Docker product e2e / M3 pass
- Public Diátaxis rewrites; remote push

## Decisions (applied)

1. `container-test-environments` gains a product-verification canon: Docker-only,
   fail-closed, ENVIRONMENTS matrix + red-blue/rule60 pointers, live lab tree,
   `humanymous-lab-runner` as live coordinator (Python scripts = oracles).
2. `structural-development` and `delivery` gain L5/product honesty cross-links;
   component greens ≠ product/M3.
3. Residual path IDs: authority INTEGRATION/fixtures, runtime gateway
   qualification-contract, CLI `native-security` feature (not absent package).
4. Skip dirty convention-audit; leave program-roadmap M3 “Not qualified” intact.

## Verification

- Branch `codex/rust` only; session `cursor-integrator` / lane `sot-plan`
- `spec check --no-local` on touched SoT/plan paths
- Receipt:
  `.spec.local/root/evidence/20261108163300-cursor-spec-canonization/`
