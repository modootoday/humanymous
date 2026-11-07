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

## Follow-up: inert generators and vendor entrypoints

After the active frozen-oracle workflow and owned AGENTS/README/manifest guidance
stopped invoking mirror generation/legacy layout validation, remove six inert
files: root CLAUDE.md/GEMINI.md, sync-adapters.ps1/.sh and
verify-agents-layout.ps1/.sh. The canonical Rust inventory requires neither
vendor stub, keeps canonical roles mandatory and treats absent mirrors as normal.
Preserve the historical frozen-go-ci.yml.txt snapshot and immutable lessons as
recorded evidence, not operational regeneration instructions. Save the six file
hashes in private vendor-entrypoint-retirement-before.json before removal.
This brings the actual removed file count to 143; settings/hooks and untracked
local state remain explicitly outside these batches. Fresh vendor discovery and
the complete source-bound CI qualification are still pending.
The root ignore policy replaces old regeneration guidance with explicit retired
provider-tree/stub exclusions. Tracked settings remain visible; no untracked
local state is deleted or silently promoted into the repository.

## Empty-directory follow-up

At the user's request, remove 88 physically empty directories left by the retired
vendor copies. Resolve every directory inside the four approved retired roots,
skip reparse points, check hidden entries and remove deepest-first with
non-recursive Directory.Delete(path,false). No file-bearing directory, build
cache, private evidence or active source tree is removed. The private receipt
empty-retired-directories-cleanup.json records all removed relative paths.
Postcheck finds zero removed paths remaining and zero empty directories within
the retired roots. Git does not track empty directories; this is a filesystem
cleanup in addition to the 143 tracked file removals.
