---
id: plan:root/20261108155700
title: Cursor spec gap audit against live Rust layout
status: active
created: 2026-11-08T15:57:00Z
updated: 2026-11-08T15:57:00Z
rel:
  depends_on: [sot:root/crate-architecture, sot:root/runtime-application, sot:root/adapter-authority, sot:root/structural-development]
---

# Cursor spec gap audit plan

## Brief

Measure written `.spec/sot` contracts against the live 29-member Cargo workspace
after core/runtime consolidation. Close high-confidence path/package honesty
gaps under the Cursor `sot-plan` claim. Leave coordinator-owned and loaned
documents as proposed deltas only.

## Non-goals

- Detection scoring, thresholds, hard rules, verdict bands
- Ghost directory deletion or Cargo membership changes
- Public Diátaxis rewrites
- Product e2e claims from oracle compose or component-only gates

## Decisions (applied)

1. Treat empty `crates/{audit,authority,runtime,native-security,watermark,...}`
   trees as **ghost skeletons**, not implementation roots.
2. Authority/audit/native-security/network/capture/gateway live under
   `humanymous-runtime` feature modules; pure engines under `humanymous-core`.
3. Update claimed SoT frontmatter/`governs` and the one stale
   `humanymous_integrity::` import string in `runtime-application`.
4. Preserve historical 13-package crate-architecture research text, but add an
   explicit live-layout correction that agents must prefer.

## Residual for Root / owners

See evidence `proposed-patches.md`: audit-key-custody verify package ID,
native-security package ID, resource-watermark package supersession,
cross-os/convention-audit selectors, optional platform SoT, ghost cleanup.

## Verification

- Branch must remain `codex/rust`
- `cargo metadata --locked --no-deps` member count remains the measurement basis
- `spec check` on touched SoT/plan paths after edits
- No scoring fixture or Engine changes
- Private measurement receipt:
  `.spec.local/root/evidence/20261108155700-cursor-spec-gap-audit/`
  (`inventory.json`, `gaps.md`, `proposed-patches.md`, `index.evidence.md`)
