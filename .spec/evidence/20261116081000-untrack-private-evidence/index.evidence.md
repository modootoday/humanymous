---
id: evidence:root/20261116081000
title: Untrack private .spec.local evidence after a consumer measurement
status: active
created: 2026-11-16T06:52:24Z
updated: 2026-11-16T06:52:24Z
rel:
  depends_on: [sot:root/rust-migration, sot:root/delivery, plan:root/20261116060213]
files:
  - {path: untracked-manifest.json, sha256: 2545639824839f849a541152ce421e3e524c7dec165f46a15e4bf7d1f9bd56b6, bytes: 34875, store: repo}
---

# Untrack private .spec.local evidence after a consumer measurement

## What was measured

AGENTS.md and `sot:root/rust-migration` define `.spec.local/` as private raw
evidence, and `.gitignore` lists `/.spec.local/`. Even so, 23 commits on
2026-11-10 (first `fa9f862`) force-added 158 files (1,379,962 bytes) under
`.spec.local/root/evidence/`. All measurements below were taken on
source `d8ce44c` in a Linux cloud container.

Sensitivity scan of the 158 files:

- Two `CONSOLIDATED-CLOSER.json` files hold a local Windows user-profile path.
- No tokens, private keys, cloud keys or e-mail addresses were found by
  pattern.
- Eight foreign executables (`*.sh`, one `*.py`) sit in one red-blue run folder.
  That conflicts with the Rust-only rule.

Consumer measurement, with `.spec.local/` moved aside to simulate a fresh clone:

| Check | With | Without |
| --- | --- | --- |
| `cargo test --locked --no-fail-fast -p humanymous-lab-runner -p humanymous-delivery -p humanymous-agent-maintenance` | — | all pass except `git_coord` (needs `pwsh`; fails with the files present too) |
| `lab-runner cap0-chain --inspect-only` | 3 blockers | 4 blockers; the extra one is AUTH-HANDOFF, which exists only as an untracked local file |
| `lab-runner until-complete-status` | 9 incomplete | 10 incomplete, the same AUTH-HANDOFF difference |

Removing the 158 tracked files changes no test result and no gate status. Each
tool fails closed when its private input is absent.

## What changed

- `git rm --cached` on all 158 files. They stay on disk in the checkout that
  holds them, and remain in Git history; this change does not rewrite history.
- `untracked-manifest.json` in this folder records path, size and SHA-256 for
  each file at index commit `d8ce44c`. Anyone who holds the private bundle can
  check it against the manifest.
- `delivery policy` (CI `policy` job) now refuses any tracked `.spec.local`
  file. Unit test: `tracked_private_evidence_is_refused`. Before the untrack,
  the policy run failed with "158 private .spec.local file(s) are tracked".

## How to read it

- The canonical summaries of these runs are the `.spec/evidence/*` documents
  that already cite them. The raw bytes are private again.
- The user-profile path remains in history. Purging it would need a history
  rewrite, which is a Root and ref decision. It was not done.
- A fresh cloud session has no private evidence. Cap0 tools therefore report
  every handoff as absent, which is the honest state for that host.
