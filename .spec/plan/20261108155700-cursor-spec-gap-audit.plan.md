---
id: plan:root/20261108155700
title: Cursor spec gap audit against live Rust layout
status: active
created: 2026-11-08T15:57:00Z
updated: 2026-11-08T16:12:00Z
rel:
  depends_on: [sot:root/crate-architecture, sot:root/runtime-application, sot:root/adapter-authority, sot:root/structural-development, sot:root/audit-key-custody, sot:root/native-security, sot:root/resource-watermark]
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

## Wave-2 residual closure (measured)

Closed under this plan: audit-key-custody / native-security package IDs,
resource-watermark package supersession, and local untracked ghost-dir hygiene.

## Wave-2b residual closure (measured)

Closed: `cross-os-runners.sot.md` audit package selectors retargeted from
`humanymous-audit` / `key_custody` to live `humanymous-runtime` /
`audit-key-custody` (plus `audit-durable-witness` / `audit-key-backups`
normative contract IDs). Detection freeze preserved; no runner invention.
Remaining Root/foreign items: dirty convention-audit historical snippets,
optional roadmap index notes, optional platform SoT. See evidence
`proposed-patches.md` and `wave2b-cross-os-runners.json`.

## Verification

- Branch must remain `codex/rust`
- `cargo metadata --locked --no-deps` member count remains the measurement basis
- `spec check` on touched SoT/plan paths after edits
- No scoring fixture or Engine changes
- Private measurement receipt:
  `.spec.local/root/evidence/20261108155700-cursor-spec-gap-audit/`
  (`inventory.json`, `gaps.md`, `proposed-patches.md`, `index.evidence.md`)
