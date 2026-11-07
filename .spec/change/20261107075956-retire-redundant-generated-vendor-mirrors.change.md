---
id: change:root/20261107075956
title: Retire redundant generated vendor mirrors
status: active
created: 2026-11-07T07:59:56Z
updated: 2026-11-07T07:59:56Z
rel:
  depends_on: [sot:root/rust-migration]
---

# Retire redundant generated vendor mirrors

## Change

Remove 137 tracked generated vendor mirror files from the approved skills,
rules, agents and lessons subtrees of .claude, .codex, .grok and .gemini.
Every non-marker file has an existing canonical .agents counterpart; generated
marker files are disposable projections. Before deletion, preserve relative
paths and SHA256 hashes in private vendor-mirror-retirement-before.json under
evidence:root/20261106194410. Git history remains intact.

The explicit text-file deletion patch leaves zero selected files and zero missing
canonical sources. No recursive filesystem deletion is used. Settings, hooks,
untracked local state, canonical .agents content, product Go/oracle sources and
build/runtime outputs are outside this first batch. This is an actual worktree
cleanup, not full Rust migration or fresh vendor discovery qualification.

## Migration

The agent-maintenance owner updates canonical-only manifest/inventory/instructions
without regenerating mirrors. The delivery owner retires the frozen oracle
workflow's sync/diff/legacy-validator consumer in favor of the Rust canonical
checks; unrelated frozen Go and attack-oracle evidence stays preserved.
Root removes obsolete generators/entrypoint stubs only after their concrete
consumers are transitioned. Provider discovery and full candidate CI remain
pending and are not inferred from file deletion or a static counterpart check.
